use bytes::Bytes;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

// Import protocol types
pub use splice::protocol::{
    Message, ExportMetadata, Role, RequestContext, AuthContext,
    PROTOCOL_VERSION, DEFAULT_MAX_FRAME_SIZE, CAP_STREAMING, CAP_CANCELLATION,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostState {
    Init,
    HandshakeSent,
    Ready,
    Shutdown,
}

pub struct MockHost {
    rx: mpsc::Receiver<Message>,
    tx: mpsc::Sender<Message>,
    pub state: HostState,
    pub pending_requests: HashMap<u64, oneshot::Sender<Result<Bytes, String>>>,
    pub next_request_id: u64,
    pub exports: Vec<ExportMetadata>,
    capabilities: u32,
}

pub struct MockHostBuilder {
    capabilities: u32,
}

impl MockHostBuilder {
    pub fn new() -> Self {
        Self {
            capabilities: CAP_STREAMING | CAP_CANCELLATION,
        }
    }

    pub fn with_capabilities(mut self, capabilities: u32) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn build(
        self,
        tx: mpsc::Sender<Message>,
        rx: mpsc::Receiver<Message>,
    ) -> MockHost {
        MockHost {
            rx,
            tx,
            state: HostState::Init,
            pending_requests: HashMap::new(),
            next_request_id: 1,
            exports: Vec::new(),
            capabilities: self.capabilities,
        }
    }
}

impl Default for MockHostBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MockHost {
    /// Perform handshake and list exports
    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Send handshake
        self.tx
            .send(Message::Handshake {
                protocol_version: PROTOCOL_VERSION,
                role: Role::Host,
                capabilities: self.capabilities,
                max_frame_size: DEFAULT_MAX_FRAME_SIZE,
            })
            .await?;

        self.state = HostState::HandshakeSent;

        // Wait for HandshakeAck
        match timeout(Duration::from_secs(5), self.rx.recv()).await {
            Ok(Some(Message::HandshakeAck { .. })) => {}
            Ok(Some(msg)) => {
                return Err(format!("Expected HandshakeAck, got {:?}", msg).into());
            }
            Ok(None) => {
                return Err("Channel closed".into());
            }
            Err(_) => {
                return Err("Handshake timeout".into());
            }
        }

        // Request exports
        self.tx.send(Message::ListExports).await?;

        // Wait for ListExportsResult
        match timeout(Duration::from_secs(5), self.rx.recv()).await {
            Ok(Some(Message::ListExportsResult { exports })) => {
                self.exports = exports;
                self.state = HostState::Ready;
            }
            Ok(Some(msg)) => {
                return Err(format!("Expected ListExportsResult, got {:?}", msg).into());
            }
            Ok(None) => {
                return Err("Channel closed".into());
            }
            Err(_) => {
                return Err("ListExports timeout".into());
            }
        }

        Ok(())
    }

    /// Invoke a function
    pub async fn invoke(
        &mut self,
        function_name: &str,
        params: JsonValue,
    ) -> Result<JsonValue, String> {
        if self.state != HostState::Ready {
            return Err("Not in ready state".to_string());
        }

        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);

        // Serialize params to MessagePack
        let params_bytes = rmp_serde::to_vec(&params)
            .map_err(|e| format!("Failed to serialize params: {}", e))?;

        let (response_tx, response_rx) = oneshot::channel();
        self.pending_requests.insert(request_id, response_tx);

        // Send Invoke message
        self.tx
            .send(Message::Invoke {
                request_id,
                function_name: function_name.to_string(),
                params: Bytes::from(params_bytes),
                deadline_ms: 30000,
                context: RequestContext {
                    trace_id: 1,
                    span_id: 1,
                    headers: vec![],
                    auth: None,
                },
            })
            .await
            .map_err(|e| format!("Failed to send invoke: {}", e))?;

        // Wait for the response by processing incoming messages
        let result = timeout(Duration::from_secs(5), async {
            // Keep processing messages until we get our response
            while let Some(msg) = self.rx.recv().await {
                self.handle_message(msg).await?;

                // Check if our response arrived
                if !self.pending_requests.contains_key(&request_id) {
                    // Response was processed, try to receive it
                    break;
                }
            }

            // Try to receive the response (it should be ready now)
            response_rx.await
                .map_err(|_| "Response channel closed".to_string())?
        })
        .await
        .map_err(|_| "Invoke timeout".to_string())??;

        // Deserialize result
        let json: JsonValue = rmp_serde::from_slice(&result)
            .map_err(|e| format!("Failed to deserialize result: {}", e))?;

        Ok(json)
    }

    /// Cancel a request
    pub async fn cancel(&mut self, request_id: u64) -> Result<(), String> {
        self.tx
            .send(Message::Cancel { request_id })
            .await
            .map_err(|e| format!("Failed to send cancel: {}", e))?;

        // Wait for CancelAck
        match timeout(Duration::from_secs(1), self.rx.recv()).await {
            Ok(Some(Message::CancelAck { .. })) => Ok(()),
            Ok(Some(msg)) => Err(format!("Expected CancelAck, got {:?}", msg)),
            Ok(None) => Err("Channel closed".to_string()),
            Err(_) => Err("Cancel timeout".to_string()),
        }
    }

    /// Shutdown the connection
    pub async fn shutdown(&mut self) -> Result<(), String> {
        self.tx
            .send(Message::Shutdown)
            .await
            .map_err(|e| format!("Failed to send shutdown: {}", e))?;

        self.state = HostState::Shutdown;

        // Wait for ShutdownAck
        match timeout(Duration::from_secs(1), self.rx.recv()).await {
            Ok(Some(Message::ShutdownAck)) => Ok(()),
            Ok(Some(msg)) => Err(format!("Expected ShutdownAck, got {:?}", msg)),
            Ok(None) => Err("Channel closed".to_string()),
            Err(_) => Err("Shutdown timeout".to_string()),
        }
    }

    /// Send a health check
    pub async fn health_check(&mut self) -> Result<(u64, u32, u64), String> {
        self.tx
            .send(Message::HealthCheck)
            .await
            .map_err(|e| format!("Failed to send health check: {}", e))?;

        // Wait for HealthStatus
        match timeout(Duration::from_secs(1), self.rx.recv()).await {
            Ok(Some(Message::HealthStatus {
                uptime_ms,
                active_requests,
                total_requests,
            })) => Ok((uptime_ms, active_requests, total_requests)),
            Ok(Some(msg)) => Err(format!("Expected HealthStatus, got {:?}", msg)),
            Ok(None) => Err("Channel closed".to_string()),
            Err(_) => Err("Health check timeout".to_string()),
        }
    }

    /// Handle incoming message
    async fn handle_message(&mut self, msg: Message) -> Result<(), String> {
        match msg {
            Message::InvokeResult {
                request_id,
                result,
                ..
            } => {
                if let Some(tx) = self.pending_requests.remove(&request_id) {
                    let _ = tx.send(Ok(result));
                }
                Ok(())
            }

            Message::InvokeError {
                request_id,
                message,
                ..
            } => {
                if let Some(tx) = self.pending_requests.remove(&request_id) {
                    let _ = tx.send(Err(message));
                }
                Ok(())
            }

            _ => {
                // Ignore other messages for now
                Ok(())
            }
        }
    }
}
