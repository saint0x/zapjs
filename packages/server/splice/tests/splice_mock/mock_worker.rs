use bytes::Bytes;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::timeout;

// Import protocol types
pub use splice::protocol::{
    Message, ExportMetadata, Role, ErrorKind, RequestContext, AuthContext,
    PROTOCOL_VERSION, DEFAULT_MAX_FRAME_SIZE, CAP_STREAMING, CAP_CANCELLATION,
    ERR_INVALID_PARAMS, ERR_EXECUTION_FAILED,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    Init,
    HandshakeReceived,
    Ready,
    Shutdown,
}

pub struct MockWorker {
    rx: mpsc::Receiver<Message>,
    tx: mpsc::Sender<Message>,
    state: WorkerState,
    exports: Vec<ExportMetadata>,
    dispatcher: Box<dyn Fn(String, JsonValue) -> Result<JsonValue, String> + Send + Sync>,
    pending_requests: HashMap<u64, Instant>,
    server_id: [u8; 16],
}

pub struct MockWorkerBuilder {
    exports: Vec<ExportMetadata>,
    dispatcher: Option<Box<dyn Fn(String, JsonValue) -> Result<JsonValue, String> + Send + Sync>>,
    server_id: [u8; 16],
}

impl MockWorkerBuilder {
    pub fn new() -> Self {
        Self {
            exports: Vec::new(),
            dispatcher: None,
            server_id: [0u8; 16],
        }
    }

    pub fn with_export(mut self, export: ExportMetadata) -> Self {
        self.exports.push(export);
        self
    }

    pub fn with_exports(mut self, exports: Vec<ExportMetadata>) -> Self {
        self.exports = exports;
        self
    }

    pub fn with_dispatcher<F>(mut self, dispatcher: F) -> Self
    where
        F: Fn(String, JsonValue) -> Result<JsonValue, String> + Send + Sync + 'static,
    {
        self.dispatcher = Some(Box::new(dispatcher));
        self
    }

    pub fn with_server_id(mut self, server_id: [u8; 16]) -> Self {
        self.server_id = server_id;
        self
    }

    pub fn build(
        self,
        rx: mpsc::Receiver<Message>,
        tx: mpsc::Sender<Message>,
    ) -> MockWorker {
        let dispatcher = self.dispatcher.unwrap_or_else(|| {
            Box::new(|_name: String, _params: JsonValue| {
                Err("No dispatcher configured".to_string())
            })
        });

        MockWorker {
            rx,
            tx,
            state: WorkerState::Init,
            exports: self.exports,
            dispatcher,
            pending_requests: HashMap::new(),
            server_id: self.server_id,
        }
    }
}

impl Default for MockWorkerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MockWorker {
    pub fn state(&self) -> WorkerState {
        self.state
    }

    pub fn pending_count(&self) -> usize {
        self.pending_requests.len()
    }

    /// Run the mock worker message loop
    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            let msg = match timeout(Duration::from_secs(30), self.rx.recv()).await {
                Ok(Some(msg)) => msg,
                Ok(None) => {
                    // Channel closed
                    return Ok(());
                }
                Err(_) => {
                    // Timeout - check if we're shutting down
                    if self.state == WorkerState::Shutdown {
                        return Ok(());
                    }
                    continue;
                }
            };

            match self.handle_message(msg).await {
                Ok(should_continue) => {
                    if !should_continue {
                        return Ok(());
                    }
                }
                Err(e) => {
                    eprintln!("MockWorker error: {}", e);
                    return Err(e);
                }
            }
        }
    }

    async fn handle_message(
        &mut self,
        msg: Message,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        match msg {
            Message::Handshake {
                protocol_version,
                role,
                capabilities,
                ..
            } => {
                // Validate role is Host
                if role != Role::Host {
                    return Err("Expected Host role".into());
                }

                // Validate protocol version
                if protocol_version != PROTOCOL_VERSION {
                    return Err("Protocol version mismatch".into());
                }

                self.state = WorkerState::HandshakeReceived;

                // Send HandshakeAck
                self.tx
                    .send(Message::HandshakeAck {
                        protocol_version: PROTOCOL_VERSION,
                        capabilities,
                        server_id: self.server_id,
                        export_count: self.exports.len() as u32,
                    })
                    .await?;

                Ok(true)
            }

            Message::ListExports => {
                if self.state != WorkerState::HandshakeReceived {
                    return Err("ListExports received before handshake".into());
                }

                self.state = WorkerState::Ready;

                // Send exports
                self.tx
                    .send(Message::ListExportsResult {
                        exports: self.exports.clone(),
                    })
                    .await?;

                Ok(true)
            }

            Message::Invoke {
                request_id,
                function_name,
                params,
                context,
                ..
            } => {
                if self.state != WorkerState::Ready {
                    return Err("Invoke received before ready state".into());
                }

                let start = Instant::now();
                self.pending_requests.insert(request_id, start);

                // Deserialize params from MessagePack to JSON
                let params_json: JsonValue = match rmp_serde::from_slice(&params) {
                    Ok(v) => v,
                    Err(e) => {
                        self.pending_requests.remove(&request_id);
                        self.tx
                            .send(Message::InvokeError {
                                request_id,
                                code: ERR_INVALID_PARAMS,
                                kind: ErrorKind::User,
                                message: format!("Failed to deserialize params: {}", e),
                                details: None,
                            })
                            .await?;
                        return Ok(true);
                    }
                };

                // Call dispatcher
                match (self.dispatcher)(function_name.clone(), params_json) {
                    Ok(result_json) => {
                        // Serialize result to MessagePack
                        let result_bytes = rmp_serde::to_vec(&result_json)
                            .map_err(|e| format!("Failed to serialize result: {}", e))?;

                        let duration = start.elapsed();
                        self.pending_requests.remove(&request_id);

                        self.tx
                            .send(Message::InvokeResult {
                                request_id,
                                result: Bytes::from(result_bytes),
                                duration_us: duration.as_micros() as u64,
                            })
                            .await?;
                    }
                    Err(error_msg) => {
                        self.pending_requests.remove(&request_id);

                        self.tx
                            .send(Message::InvokeError {
                                request_id,
                                code: ERR_EXECUTION_FAILED,
                                kind: ErrorKind::User,
                                message: error_msg,
                                details: None,
                            })
                            .await?;
                    }
                }

                Ok(true)
            }

            Message::Cancel { request_id } => {
                // Remove from pending if exists
                self.pending_requests.remove(&request_id);

                // Send CancelAck
                self.tx.send(Message::CancelAck { request_id }).await?;

                Ok(true)
            }

            Message::Shutdown => {
                self.state = WorkerState::Shutdown;

                // Send ShutdownAck
                self.tx.send(Message::ShutdownAck).await?;

                // Return false to stop the loop
                Ok(false)
            }

            Message::HealthCheck => {
                // Send health status
                self.tx
                    .send(Message::HealthStatus {
                        uptime_ms: 0, // Simplified for mock
                        active_requests: self.pending_requests.len() as u32,
                        total_requests: 0, // Simplified for mock
                    })
                    .await?;

                Ok(true)
            }

            _ => {
                return Err(format!("Unexpected message type: {:?}", msg).into());
            }
        }
    }
}
