///! Splice Protocol Worker Runtime
///!
///! This module provides the runtime for user-server binaries to connect to zap-splice
///! and serve exported Rust functions via the Splice protocol.

use bytes::Bytes;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::net::UnixStream;
use tokio::sync::RwLock;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};

// Import Splice protocol types
use crate::registry::{build_rpc_dispatcher, ExportedFunction};

/// Splice protocol message (simplified - full protocol in zap-splice crate)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SpliceMessage {
    Handshake {
        protocol_version: u32,
        role: u8, // 2 = Worker
        capabilities: u32,
        max_frame_size: u32,
    },
    HandshakeAck {
        protocol_version: u32,
        capabilities: u32,
        server_id: [u8; 16],
        export_count: u32,
    },
    ListExports,
    ListExportsResult {
        exports: Vec<ExportMetadata>,
    },
    Invoke {
        request_id: u64,
        function_name: String,
        params: Bytes,
        deadline_ms: u32,
        context: RequestContext,
    },
    InvokeResult {
        request_id: u64,
        result: Bytes,
        duration_us: u64,
    },
    InvokeError {
        request_id: u64,
        code: u16,
        kind: u8,
        message: String,
        details: Option<Bytes>,
    },
    Cancel {
        request_id: u64,
    },
    CancelAck {
        request_id: u64,
    },
    Shutdown,
    ShutdownAck,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportMetadata {
    pub name: String,
    pub is_async: bool,
    pub is_streaming: bool,
    pub params_schema: String,
    pub return_schema: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RequestContext {
    pub trace_id: u64,
    pub span_id: u64,
    pub headers: Vec<(String, String)>,
    pub auth: Option<AuthContext>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthContext {
    pub user_id: String,
    pub roles: Vec<String>,
}

/// Run the Splice worker runtime
///
/// This function should be called from the user-server's main function:
/// ```ignore
/// #[tokio::main]
/// async fn main() {
///     zap_server::splice_worker::run().await.unwrap();
/// }
/// ```
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Splice worker runtime");

    // Get socket path from environment
    let socket_path = env::var("ZAP_SOCKET")
        .map_err(|_| "ZAP_SOCKET environment variable not set")?;

    info!("Connecting to zap-splice at: {}", socket_path);

    // Connect to zap-splice
    let stream = UnixStream::connect(&socket_path).await?;
    let mut framed = create_framed_stream(stream);

    // Build RPC dispatcher from inventory
    let dispatcher = build_rpc_dispatcher();
    let exports = collect_exports();

    // Send handshake
    send_message(&mut framed, SpliceMessage::Handshake {
        protocol_version: 0x00010000,
        role: 2, // Worker
        capabilities: 0b11, // Streaming + Cancellation
        max_frame_size: 100 * 1024 * 1024,
    }).await?;

    // Wait for handshake ack
    match receive_message(&mut framed).await? {
        SpliceMessage::HandshakeAck { .. } => {
            info!("Handshake complete");
        }
        _ => {
            return Err("Expected HandshakeAck".into());
        }
    }

    // Message loop
    loop {
        match receive_message(&mut framed).await? {
            SpliceMessage::ListExports => {
                debug!("Sending exports list ({} functions)", exports.len());
                send_message(&mut framed, SpliceMessage::ListExportsResult {
                    exports: exports.clone(),
                }).await?;
            }

            SpliceMessage::Invoke {
                request_id,
                function_name,
                params,
                deadline_ms,
                context,
            } => {
                debug!("Invoking function: {} (request_id: {})", function_name, request_id);

                let start = std::time::Instant::now();

                // Deserialize params from MessagePack to JSON
                let params_json: serde_json::Value = rmp_serde::from_slice(&params)
                    .unwrap_or_else(|_| serde_json::json!({}));

                // Call function via dispatcher
                let result = dispatcher(function_name.clone(), params_json);

                let duration_us = start.elapsed().as_micros() as u64;

                match result {
                    Ok(result_json) => {
                        // Serialize result to MessagePack
                        let result_bytes = rmp_serde::to_vec(&result_json)
                            .map_err(|e| format!("Failed to serialize result: {}", e))?;

                        send_message(&mut framed, SpliceMessage::InvokeResult {
                            request_id,
                            result: Bytes::from(result_bytes),
                            duration_us,
                        }).await?;
                    }
                    Err(error_msg) => {
                        send_message(&mut framed, SpliceMessage::InvokeError {
                            request_id,
                            code: 2000, // ERR_EXECUTION_FAILED
                            kind: 1, // User error
                            message: error_msg,
                            details: None,
                        }).await?;
                    }
                }
            }

            SpliceMessage::Cancel { request_id } => {
                debug!("Cancel request: {}", request_id);
                // TODO: Implement cancellation support
                send_message(&mut framed, SpliceMessage::CancelAck { request_id }).await?;
            }

            SpliceMessage::Shutdown => {
                info!("Shutdown requested");
                send_message(&mut framed, SpliceMessage::ShutdownAck).await?;
                break;
            }

            msg => {
                warn!("Unexpected message: {:?}", msg);
            }
        }
    }

    info!("Worker runtime shutting down");
    Ok(())
}

/// Collect exported functions from inventory
fn collect_exports() -> Vec<ExportMetadata> {
    inventory::iter::<ExportedFunction>
        .into_iter()
        .map(|f| ExportMetadata {
            name: f.name.to_string(),
            is_async: f.is_async,
            is_streaming: false, // TODO: Support streaming
            params_schema: "{}".to_string(), // TODO: Extract from function
            return_schema: "{}".to_string(), // TODO: Extract from function
        })
        .collect()
}

// Simplified framing - in production would use zap-splice codec
fn create_framed_stream(stream: UnixStream) -> Framed<UnixStream, SpliceCodec> {
    Framed::new(stream, SpliceCodec::default())
}

async fn send_message(
    framed: &mut Framed<UnixStream, SpliceCodec>,
    msg: SpliceMessage,
) -> Result<(), Box<dyn std::error::Error>> {
    use futures::sink::SinkExt;
    framed.send(msg).await?;
    Ok(())
}

async fn receive_message(
    framed: &mut Framed<UnixStream, SpliceCodec>,
) -> Result<SpliceMessage, Box<dyn std::error::Error>> {
    use futures::stream::StreamExt;
    framed
        .next()
        .await
        .ok_or("Connection closed")?
        .map_err(Into::into)
}

// Simplified codec - matches zap-splice protocol
use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

pub struct SpliceCodec {
    max_frame_size: usize,
}

impl Default for SpliceCodec {
    fn default() -> Self {
        Self {
            max_frame_size: 100 * 1024 * 1024,
        }
    }
}

impl Decoder for SpliceCodec {
    type Item = SpliceMessage;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 5 {
            return Ok(None);
        }

        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_be_bytes(length_bytes) as usize;

        if length > self.max_frame_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Frame too large",
            ));
        }

        if src.len() < 5 + length {
            src.reserve(5 + length - src.len());
            return Ok(None);
        }

        src.advance(4);
        let _msg_type = src.get_u8();
        let payload = src.split_to(length).freeze();

        let message = rmp_serde::from_slice(&payload).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;

        Ok(Some(message))
    }
}

impl Encoder<SpliceMessage> for SpliceCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: SpliceMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let payload = rmp_serde::to_vec(&item).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;

        if payload.len() > self.max_frame_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Frame too large",
            ));
        }

        let msg_type = match &item {
            SpliceMessage::Handshake { .. } => 0x01,
            SpliceMessage::HandshakeAck { .. } => 0x02,
            SpliceMessage::ListExports => 0x10,
            SpliceMessage::ListExportsResult { .. } => 0x11,
            SpliceMessage::Invoke { .. } => 0x20,
            SpliceMessage::InvokeResult { .. } => 0x21,
            SpliceMessage::InvokeError { .. } => 0x22,
            SpliceMessage::Cancel { .. } => 0x40,
            SpliceMessage::CancelAck { .. } => 0x41,
            SpliceMessage::Shutdown => 0x03,
            SpliceMessage::ShutdownAck => 0x04,
        };

        dst.reserve(5 + payload.len());
        dst.put_u32(payload.len() as u32);
        dst.put_u8(msg_type);
        dst.put_slice(&payload);

        Ok(())
    }
}
