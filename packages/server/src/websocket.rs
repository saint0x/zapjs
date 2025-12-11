//! WebSocket Handler for ZapJS
//!
//! Provides bidirectional real-time communication between clients and TypeScript handlers.
//!
//! Architecture:
//! ```text
//! Client <--WS--> Rust Server <--IPC--> TypeScript Handler
//! ```
//!
//! Route Conventions:
//! - `WEBSOCKET` export in api/ folder routes
//! - Default export in ws/ folder routes
//!
//! IPC Message Flow:
//! - WsConnect: Client connected (Rust -> TS)
//! - WsMessage: Message received from client (Rust -> TS)
//! - WsSend: Message to send to client (TS -> Rust)
//! - WsClose: Connection closed (bidirectional)

use crate::error::{ZapError, ZapResult};
use crate::ipc::{IpcClient, IpcEncoding, IpcMessage};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Error as WsError, Message as WsMessage},
    WebSocketStream,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// WebSocket handler configuration
#[derive(Clone)]
pub struct WsConfig {
    /// IPC socket path for communication with TypeScript
    pub ipc_socket_path: String,
    /// Handler ID for this WebSocket route
    pub handler_id: String,
    /// Maximum message size (default: 64KB)
    pub max_message_size: usize,
    /// Ping interval in seconds (default: 30)
    pub ping_interval_secs: u64,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            ipc_socket_path: String::new(),
            handler_id: String::new(),
            max_message_size: 64 * 1024, // 64KB
            ping_interval_secs: 30,
        }
    }
}

impl WsConfig {
    /// Create a new WebSocket config
    pub fn new(ipc_socket_path: String, handler_id: String) -> Self {
        Self {
            ipc_socket_path,
            handler_id,
            ..Default::default()
        }
    }
}

/// Handle a WebSocket connection
///
/// This function upgrades the HTTP connection to WebSocket and manages
/// bidirectional message flow between the client and TypeScript handler.
pub async fn handle_websocket_connection<S>(
    stream: S,
    config: WsConfig,
    path: String,
    headers: HashMap<String, String>,
) -> ZapResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    // Accept the WebSocket connection
    let ws_stream = accept_async(stream).await.map_err(|e| {
        error!("WebSocket handshake failed: {}", e);
        ZapError::websocket(format!("Handshake failed: {}", e))
    })?;

    // Generate unique connection ID
    let connection_id = Uuid::new_v4().to_string();
    info!(
        "WebSocket connection established: {} on {}",
        connection_id, path
    );

    // Connect to TypeScript IPC server
    let mut ipc_client = IpcClient::connect_with_encoding(&config.ipc_socket_path, IpcEncoding::MessagePack)
        .await
        .map_err(|e| {
            error!("Failed to connect to IPC for WebSocket: {}", e);
            e
        })?;

    // Notify TypeScript of the new connection
    let connect_msg = IpcMessage::WsConnect {
        connection_id: connection_id.clone(),
        handler_id: config.handler_id.clone(),
        path: path.clone(),
        headers: headers.clone(),
    };
    ipc_client.send_message(connect_msg).await?;

    // Split the WebSocket stream
    let (ws_sink, ws_stream) = ws_stream.split();

    // Create channels for communication
    let (outbound_tx, outbound_rx) = mpsc::channel::<WsMessage>(32);

    // Spawn tasks for handling the connection
    let connection_id_clone = connection_id.clone();
    let config_clone = config.clone();

    // Task 1: Handle incoming WebSocket messages from client
    let inbound_handle = tokio::spawn(async move {
        handle_inbound_messages(ws_stream, ipc_client, connection_id_clone, config_clone).await
    });

    // Task 2: Handle outbound messages to client
    let outbound_handle = tokio::spawn(async move {
        handle_outbound_messages(ws_sink, outbound_rx).await
    });

    // Wait for either task to complete
    tokio::select! {
        result = inbound_handle => {
            if let Err(e) = result {
                error!("Inbound handler error: {}", e);
            }
        }
        result = outbound_handle => {
            if let Err(e) = result {
                error!("Outbound handler error: {}", e);
            }
        }
    }

    info!("WebSocket connection closed: {}", connection_id);
    Ok(())
}

/// Handle incoming WebSocket messages from the client
async fn handle_inbound_messages<S>(
    mut ws_stream: futures::stream::SplitStream<WebSocketStream<S>>,
    mut ipc_client: IpcClient,
    connection_id: String,
    config: WsConfig,
) -> ZapResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    while let Some(msg_result) = ws_stream.next().await {
        match msg_result {
            Ok(msg) => {
                match msg {
                    WsMessage::Text(text) => {
                        debug!(
                            "Received text message from {}: {} bytes",
                            connection_id,
                            text.len()
                        );

                        // Forward to TypeScript
                        let ipc_msg = IpcMessage::WsMessage {
                            connection_id: connection_id.clone(),
                            handler_id: config.handler_id.clone(),
                            data: text,
                            binary: false,
                        };
                        if let Err(e) = ipc_client.send_message(ipc_msg).await {
                            error!("Failed to forward message to TypeScript: {}", e);
                            break;
                        }
                    }
                    WsMessage::Binary(data) => {
                        debug!(
                            "Received binary message from {}: {} bytes",
                            connection_id,
                            data.len()
                        );

                        // Forward to TypeScript (base64 encoded)
                        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
                        let encoded = BASE64.encode(&data);

                        let ipc_msg = IpcMessage::WsMessage {
                            connection_id: connection_id.clone(),
                            handler_id: config.handler_id.clone(),
                            data: encoded,
                            binary: true,
                        };
                        if let Err(e) = ipc_client.send_message(ipc_msg).await {
                            error!("Failed to forward binary message to TypeScript: {}", e);
                            break;
                        }
                    }
                    WsMessage::Ping(data) => {
                        debug!("Received ping from {}", connection_id);
                        // Pong is handled automatically by tungstenite
                    }
                    WsMessage::Pong(_) => {
                        debug!("Received pong from {}", connection_id);
                    }
                    WsMessage::Close(frame) => {
                        let (code, reason) = frame
                            .map(|f| (Some(f.code.into()), Some(f.reason.to_string())))
                            .unwrap_or((None, None));

                        info!(
                            "WebSocket {} closed by client: code={:?}, reason={:?}",
                            connection_id, code, reason
                        );

                        // Notify TypeScript
                        let close_msg = IpcMessage::WsClose {
                            connection_id: connection_id.clone(),
                            handler_id: config.handler_id.clone(),
                            code,
                            reason,
                        };
                        let _ = ipc_client.send_message(close_msg).await;
                        break;
                    }
                    WsMessage::Frame(_) => {
                        // Raw frames are not typically handled at this level
                    }
                }
            }
            Err(e) => {
                match e {
                    WsError::ConnectionClosed | WsError::AlreadyClosed => {
                        info!("WebSocket {} connection closed", connection_id);
                    }
                    _ => {
                        error!("WebSocket error for {}: {}", connection_id, e);
                    }
                }

                // Notify TypeScript of closure
                let close_msg = IpcMessage::WsClose {
                    connection_id: connection_id.clone(),
                    handler_id: config.handler_id.clone(),
                    code: None,
                    reason: Some(format!("Error: {}", e)),
                };
                let _ = ipc_client.send_message(close_msg).await;
                break;
            }
        }
    }

    Ok(())
}

/// Handle outbound WebSocket messages to the client
async fn handle_outbound_messages<S>(
    mut ws_sink: futures::stream::SplitSink<WebSocketStream<S>, WsMessage>,
    mut outbound_rx: mpsc::Receiver<WsMessage>,
) -> ZapResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    while let Some(msg) = outbound_rx.recv().await {
        if let Err(e) = ws_sink.send(msg).await {
            error!("Failed to send WebSocket message: {}", e);
            break;
        }
    }

    Ok(())
}

/// WebSocket handler that manages IPC communication for outbound messages
pub struct WsHandler {
    config: WsConfig,
    /// Channel sender for outbound messages (connection_id -> sender)
    senders: Arc<tokio::sync::RwLock<HashMap<String, mpsc::Sender<WsMessage>>>>,
}

impl WsHandler {
    /// Create a new WebSocket handler
    pub fn new(config: WsConfig) -> Self {
        Self {
            config,
            senders: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Register a connection's outbound sender
    pub async fn register_connection(
        &self,
        connection_id: String,
        sender: mpsc::Sender<WsMessage>,
    ) {
        let mut senders = self.senders.write().await;
        senders.insert(connection_id, sender);
    }

    /// Unregister a connection
    pub async fn unregister_connection(&self, connection_id: &str) {
        let mut senders = self.senders.write().await;
        senders.remove(connection_id);
    }

    /// Send a message to a specific connection
    pub async fn send_to_connection(
        &self,
        connection_id: &str,
        message: WsMessage,
    ) -> ZapResult<()> {
        let senders = self.senders.read().await;
        if let Some(sender) = senders.get(connection_id) {
            sender.send(message).await.map_err(|e| {
                ZapError::websocket(format!("Failed to send to {}: {}", connection_id, e))
            })?;
            Ok(())
        } else {
            Err(ZapError::websocket(format!(
                "Connection {} not found",
                connection_id
            )))
        }
    }

    /// Handle an IPC message for WebSocket (from TypeScript)
    pub async fn handle_ipc_message(&self, msg: IpcMessage) -> ZapResult<()> {
        match msg {
            IpcMessage::WsSend {
                connection_id,
                data,
                binary,
            } => {
                let ws_msg = if binary {
                    // Decode base64 for binary
                    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
                    let decoded = BASE64.decode(&data).map_err(|e| {
                        ZapError::websocket(format!("Invalid base64 data: {}", e))
                    })?;
                    WsMessage::Binary(decoded)
                } else {
                    WsMessage::Text(data)
                };

                self.send_to_connection(&connection_id, ws_msg).await?;
            }
            IpcMessage::WsClose {
                connection_id,
                handler_id: _,
                code,
                reason,
            } => {
                // Close the connection
                let close_frame = code.map(|c| {
                    tokio_tungstenite::tungstenite::protocol::CloseFrame {
                        code: c.into(),
                        reason: reason.unwrap_or_default().into(),
                    }
                });

                let ws_msg = WsMessage::Close(close_frame);
                let _ = self.send_to_connection(&connection_id, ws_msg).await;
                self.unregister_connection(&connection_id).await;
            }
            _ => {
                warn!("Unexpected IPC message for WebSocket handler: {:?}", msg);
            }
        }

        Ok(())
    }
}

/// Check if an HTTP request is a WebSocket upgrade request
pub fn is_websocket_upgrade(headers: &HashMap<String, String>) -> bool {
    headers
        .get("upgrade")
        .map(|v| v.to_lowercase() == "websocket")
        .unwrap_or(false)
        && headers
            .get("connection")
            .map(|v| v.to_lowercase().contains("upgrade"))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_websocket_upgrade() {
        let mut headers = HashMap::new();
        headers.insert("upgrade".to_string(), "websocket".to_string());
        headers.insert("connection".to_string(), "Upgrade".to_string());
        assert!(is_websocket_upgrade(&headers));

        // Case insensitive
        let mut headers2 = HashMap::new();
        headers2.insert("upgrade".to_string(), "WebSocket".to_string());
        headers2.insert("connection".to_string(), "keep-alive, Upgrade".to_string());
        assert!(is_websocket_upgrade(&headers2));

        // Not a WebSocket upgrade
        let mut headers3 = HashMap::new();
        headers3.insert("connection".to_string(), "keep-alive".to_string());
        assert!(!is_websocket_upgrade(&headers3));
    }

    #[test]
    fn test_ws_config_default() {
        let config = WsConfig::default();
        assert_eq!(config.max_message_size, 64 * 1024);
        assert_eq!(config.ping_interval_secs, 30);
    }

    #[test]
    fn test_ws_config_new() {
        let config = WsConfig::new("/tmp/test.sock".to_string(), "ws_handler_0".to_string());
        assert_eq!(config.ipc_socket_path, "/tmp/test.sock");
        assert_eq!(config.handler_id, "ws_handler_0");
    }
}
