//! Proxy handler that forwards requests to TypeScript via IPC
//!
//! When a TypeScript handler is routed, this handler:
//! 1. Serializes the request to IPC protocol
//! 2. Sends to TypeScript via Unix socket (using connection pool)
//! 3. Waits for response with timeout
//! 4. Converts response back to HTTP
//!
//! Supports both regular and streaming responses from TypeScript handlers.

use crate::connection_pool::ConnectionPool;
use crate::error::{ZapError, ZapResult};
use crate::handler::Handler;
use crate::ipc::{IpcClient, IpcEncoding, IpcMessage, IpcRequest};
use crate::request_id;
use crate::response::{StreamingResponse, ZapResponse};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use zap_core::Request;

/// Handler that proxies requests to TypeScript via IPC
pub struct ProxyHandler {
    /// Unique identifier for this handler
    handler_id: String,

    /// Path to the Unix socket for IPC communication
    ipc_socket_path: Arc<String>,

    /// Request timeout in seconds
    timeout_secs: u64,

    /// Optional connection pool (if None, uses global pool or creates per-request connections)
    connection_pool: Option<Arc<ConnectionPool>>,
}

impl ProxyHandler {
    /// Create a new proxy handler
    pub fn new(handler_id: String, ipc_socket_path: String) -> Self {
        Self {
            handler_id,
            ipc_socket_path: Arc::new(ipc_socket_path),
            timeout_secs: 30,
            connection_pool: None,
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(
        handler_id: String,
        ipc_socket_path: String,
        timeout_secs: u64,
    ) -> Self {
        Self {
            handler_id,
            ipc_socket_path: Arc::new(ipc_socket_path),
            timeout_secs,
            connection_pool: None,
        }
    }

    /// Create with a specific connection pool
    pub fn with_pool(
        handler_id: String,
        ipc_socket_path: String,
        pool: Arc<ConnectionPool>,
    ) -> Self {
        Self {
            handler_id,
            ipc_socket_path: Arc::new(ipc_socket_path),
            timeout_secs: 30,
            connection_pool: Some(pool),
        }
    }

    /// Create with custom timeout and connection pool
    pub fn with_timeout_and_pool(
        handler_id: String,
        ipc_socket_path: String,
        timeout_secs: u64,
        pool: Arc<ConnectionPool>,
    ) -> Self {
        Self {
            handler_id,
            ipc_socket_path: Arc::new(ipc_socket_path),
            timeout_secs,
            connection_pool: Some(pool),
        }
    }

    /// Make an IPC request to the TypeScript handler
    /// Returns the response which may be a regular response or a streaming start message
    async fn invoke_handler(&self, request: IpcRequest) -> ZapResult<ZapResponse> {
        debug!(
            "📤 Invoking TypeScript handler: {} for {} {}",
            self.handler_id, request.method, request.path
        );

        // Create invocation message
        let msg = IpcMessage::InvokeHandler {
            handler_id: self.handler_id.clone(),
            request,
        };

        // For streaming support, we need a dedicated connection that we can keep reading from
        // We can't use the connection pool for this because streaming needs multiple reads
        // So we create a dedicated connection for the entire request lifecycle
        let response = self.invoke_with_streaming_support(msg).await?;

        debug!("📥 Received response from TypeScript handler");

        Ok(response)
    }

    /// Invoke handler with full streaming support
    /// This uses a dedicated connection so we can handle streaming responses
    async fn invoke_with_streaming_support(&self, msg: IpcMessage) -> ZapResult<ZapResponse> {
        // Connect to TypeScript's IPC server
        let mut client = IpcClient::connect_with_encoding(
            self.ipc_socket_path.as_str(),
            IpcEncoding::MessagePack,
        )
        .await
        .map_err(|e| {
            error!("Failed to connect to IPC: {}", e);
            e
        })?;

        // Send the invocation
        client.send_message(msg).await.map_err(|e| {
            error!("Failed to send IPC message: {}", e);
            e
        })?;

        // Wait for first response with timeout
        let timeout_duration = std::time::Duration::from_secs(self.timeout_secs);

        let first_response = tokio::time::timeout(timeout_duration, client.recv_message())
            .await
            .map_err(|_| {
                warn!(
                    "Handler {} timed out after {}s",
                    self.handler_id, self.timeout_secs
                );
                ZapError::timeout(
                    format!(
                        "Handler {} did not respond within {}s",
                        self.handler_id, self.timeout_secs
                    ),
                    self.timeout_secs * 1000,
                )
            })?
            .map_err(|e| {
                error!("IPC connection error: {}", e);
                ZapError::ipc("Connection error")
            })?
            .ok_or_else(|| {
                error!("Received None from IPC channel");
                ZapError::ipc("No response from handler")
            })?;

        // Handle the response based on type
        match first_response {
            // Regular handler response - return immediately
            IpcMessage::HandlerResponse {
                handler_id: _,
                status,
                headers,
                body,
            } => {
                debug!("Converting IPC response to HTTP response (status: {})", status);

                let status_code = zap_core::StatusCode::new(status);
                let mut zap_response = zap_core::Response::with_status(status_code).body(body);

                for (key, value) in headers {
                    zap_response = zap_response.header(key, value);
                }

                Ok(ZapResponse::Custom(zap_response))
            }

            // Streaming response - continue reading chunks until StreamEnd
            IpcMessage::StreamStart {
                stream_id,
                status,
                headers,
            } => {
                info!("Starting streaming response: {} (status: {})", stream_id, status);
                self.handle_streaming_response(&mut client, stream_id, status, headers)
                    .await
            }

            // Error response
            IpcMessage::Error { code, message, .. } => {
                error!(
                    "Handler {} returned error: {} - {}",
                    self.handler_id, code, message
                );
                Err(ZapError::handler_with_id(
                    format!("{}: {}", code, message),
                    &self.handler_id,
                ))
            }

            // Unexpected message type
            other => {
                error!(
                    "Handler {} returned unexpected message type: {:?}",
                    self.handler_id, other
                );
                Err(ZapError::handler_with_id(
                    "Invalid response type from TypeScript handler",
                    &self.handler_id,
                ))
            }
        }
    }

    /// Handle a streaming response by collecting all chunks until StreamEnd
    async fn handle_streaming_response(
        &self,
        client: &mut IpcClient,
        stream_id: String,
        status: u16,
        headers: std::collections::HashMap<String, String>,
    ) -> ZapResult<ZapResponse> {
        let mut streaming_response = StreamingResponse::new(status, headers);
        let timeout_duration = std::time::Duration::from_secs(self.timeout_secs);

        loop {
            // Read next message with timeout
            let msg = tokio::time::timeout(timeout_duration, client.recv_message())
                .await
                .map_err(|_| {
                    warn!(
                        "Streaming response {} timed out after {}s",
                        stream_id, self.timeout_secs
                    );
                    ZapError::timeout(
                        format!(
                            "Streaming response {} did not complete within {}s",
                            stream_id, self.timeout_secs
                        ),
                        self.timeout_secs * 1000,
                    )
                })?
                .map_err(|e| {
                    error!("IPC connection error during streaming: {}", e);
                    ZapError::ipc("Connection error during streaming")
                })?
                .ok_or_else(|| {
                    error!("Connection closed during streaming");
                    ZapError::ipc("Connection closed during streaming")
                })?;

            match msg {
                // Chunk received - decode and add to response
                IpcMessage::StreamChunk {
                    stream_id: chunk_stream_id,
                    data,
                } => {
                    if chunk_stream_id != stream_id {
                        warn!(
                            "Received chunk for wrong stream: expected {}, got {}",
                            stream_id, chunk_stream_id
                        );
                        continue;
                    }

                    // Decode base64 data
                    match BASE64.decode(&data) {
                        Ok(decoded) => {
                            debug!(
                                "Received chunk for stream {}: {} bytes",
                                stream_id,
                                decoded.len()
                            );
                            streaming_response.add_chunk(decoded);
                        }
                        Err(e) => {
                            error!("Failed to decode base64 chunk: {}", e);
                            // Try treating as raw UTF-8
                            streaming_response.add_chunk(data.into_bytes());
                        }
                    }
                }

                // Stream ended - return the collected response
                IpcMessage::StreamEnd {
                    stream_id: end_stream_id,
                } => {
                    if end_stream_id != stream_id {
                        warn!(
                            "Received end for wrong stream: expected {}, got {}",
                            stream_id, end_stream_id
                        );
                        continue;
                    }

                    info!(
                        "Streaming response {} completed: {} chunks, {} bytes total",
                        stream_id,
                        streaming_response.chunks.len(),
                        streaming_response.body_bytes().len()
                    );
                    return Ok(ZapResponse::Stream(streaming_response));
                }

                // Error during streaming
                IpcMessage::Error { code, message, .. } => {
                    error!(
                        "Error during streaming {}: {} - {}",
                        stream_id, code, message
                    );
                    return Err(ZapError::handler_with_id(
                        format!("Streaming error: {}: {}", code, message),
                        &self.handler_id,
                    ));
                }

                // Unexpected message
                other => {
                    warn!(
                        "Unexpected message during streaming {}: {:?}",
                        stream_id, other
                    );
                    // Continue waiting for proper stream messages
                }
            }
        }
    }

    /// Invoke handler using connection pool (for non-streaming responses)
    #[allow(dead_code)]
    async fn invoke_with_pool(
        &self,
        pool: &ConnectionPool,
        msg: IpcMessage,
    ) -> ZapResult<IpcMessage> {
        let timeout_duration = std::time::Duration::from_secs(self.timeout_secs);

        tokio::time::timeout(timeout_duration, pool.send_recv(msg))
            .await
            .map_err(|_| {
                warn!(
                    "Handler {} timed out after {}s",
                    self.handler_id, self.timeout_secs
                );
                ZapError::timeout(
                    format!(
                        "Handler {} did not respond within {}s",
                        self.handler_id, self.timeout_secs
                    ),
                    self.timeout_secs * 1000,
                )
            })?
    }
}

impl Handler for ProxyHandler {
    fn handle<'a>(
        &'a self,
        req: Request<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<ZapResponse, ZapError>> + Send + 'a>> {
        Box::pin(async move {
            // Convert Rust request to IPC request format
            let body_bytes = req.body();
            let body_string = String::from_utf8_lossy(body_bytes).to_string();

            // Use the request data that's already been parsed
            // Get or generate request ID for correlation
            let headers_map: std::collections::HashMap<String, String> = req
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            let request_id = request_id::get_or_generate(&headers_map);

            let ipc_request = IpcRequest {
                request_id,
                method: req.method().to_string(),
                path: req.path().to_string(), // Already includes query string
                path_only: req.path_only().to_string(),
                query: req
                    .query_params()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                params: req
                    .params()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                headers: headers_map,
                body: body_string,
                cookies: req
                    .cookies()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            };

            // Invoke TypeScript handler via IPC (handles both regular and streaming responses)
            self.invoke_handler(ipc_request).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_handler_creation() {
        let handler = ProxyHandler::new(
            "handler_0".to_string(),
            "/tmp/zap.sock".to_string(),
        );
        assert_eq!(handler.handler_id, "handler_0");
        assert_eq!(handler.timeout_secs, 30);
    }

    #[test]
    fn test_proxy_handler_with_custom_timeout() {
        let handler = ProxyHandler::with_timeout(
            "handler_1".to_string(),
            "/tmp/zap.sock".to_string(),
            60,
        );
        assert_eq!(handler.handler_id, "handler_1");
        assert_eq!(handler.timeout_secs, 60);
    }
}
