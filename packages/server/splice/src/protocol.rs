use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use std::io;
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};

/// Protocol version 1.0
pub const PROTOCOL_VERSION: u32 = 0x00010000;

/// Default maximum frame size (100MB)
pub const DEFAULT_MAX_FRAME_SIZE: u32 = 100 * 1024 * 1024;

/// Capability flags
pub const CAP_STREAMING: u32 = 1 << 0;
pub const CAP_CANCELLATION: u32 = 1 << 1;
pub const CAP_COMPRESSION: u32 = 1 << 2;

// Message type codes
pub const MSG_HANDSHAKE: u8 = 0x01;
pub const MSG_HANDSHAKE_ACK: u8 = 0x02;
pub const MSG_SHUTDOWN: u8 = 0x03;
pub const MSG_SHUTDOWN_ACK: u8 = 0x04;
pub const MSG_LIST_EXPORTS: u8 = 0x10;
pub const MSG_LIST_EXPORTS_RESULT: u8 = 0x11;
pub const MSG_INVOKE: u8 = 0x20;
pub const MSG_INVOKE_RESULT: u8 = 0x21;
pub const MSG_INVOKE_ERROR: u8 = 0x22;
pub const MSG_STREAM_START: u8 = 0x30;
pub const MSG_STREAM_CHUNK: u8 = 0x31;
pub const MSG_STREAM_END: u8 = 0x32;
pub const MSG_STREAM_ERROR: u8 = 0x33;
pub const MSG_STREAM_ACK: u8 = 0x34;
pub const MSG_CANCEL: u8 = 0x40;
pub const MSG_CANCEL_ACK: u8 = 0x41;
pub const MSG_LOG_EVENT: u8 = 0x50;
pub const MSG_HEALTH_CHECK: u8 = 0x60;
pub const MSG_HEALTH_STATUS: u8 = 0x61;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Frame too large: {0} bytes")]
    FrameTooLarge(usize),

    #[error("Invalid message type: {0}")]
    InvalidMessageType(u8),

    #[error("Protocol version mismatch")]
    VersionMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Role {
    Host = 1,
    Worker = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ErrorKind {
    User = 1,
    System = 2,
    Timeout = 3,
    Cancelled = 4,
}

// Error codes
pub const ERR_INVALID_REQUEST: u16 = 1000;
pub const ERR_INVALID_PARAMS: u16 = 1001;
pub const ERR_FUNCTION_NOT_FOUND: u16 = 1002;
pub const ERR_UNAUTHORIZED: u16 = 1003;
pub const ERR_FRAME_TOO_LARGE: u16 = 1004;
pub const ERR_EXECUTION_FAILED: u16 = 2000;
pub const ERR_TIMEOUT: u16 = 2001;
pub const ERR_CANCELLED: u16 = 2002;
pub const ERR_PANIC: u16 = 2003;
pub const ERR_INTERNAL_ERROR: u16 = 3000;
pub const ERR_UNAVAILABLE: u16 = 3001;
pub const ERR_OVERLOADED: u16 = 3002;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub trace_id: u64,
    pub span_id: u64,
    pub headers: Vec<(String, String)>,
    pub auth: Option<AuthContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub name: String,
    pub is_async: bool,
    pub is_streaming: bool,
    pub params_schema: String,
    pub return_schema: String,
}

/// Splice protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // Connection lifecycle
    Handshake {
        protocol_version: u32,
        role: Role,
        capabilities: u32,
        max_frame_size: u32,
    },
    HandshakeAck {
        protocol_version: u32,
        capabilities: u32,
        server_id: [u8; 16],
        export_count: u32,
    },
    Shutdown,
    ShutdownAck,

    // Function registry
    ListExports,
    ListExportsResult {
        exports: Vec<ExportMetadata>,
    },

    // Function invocation
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
        kind: ErrorKind,
        message: String,
        details: Option<Bytes>,
    },

    // Streaming
    StreamStart {
        request_id: u64,
        window: u32,
    },
    StreamChunk {
        request_id: u64,
        sequence: u64,
        data: Bytes,
    },
    StreamEnd {
        request_id: u64,
        total_chunks: u64,
    },
    StreamError {
        request_id: u64,
        code: u16,
        message: String,
    },
    StreamAck {
        request_id: u64,
        ack_sequence: u64,
        window: u32,
    },

    // Cancellation
    Cancel {
        request_id: u64,
    },
    CancelAck {
        request_id: u64,
    },

    // Logging and health
    LogEvent {
        level: String,
        message: String,
        fields: Vec<(String, String)>,
    },
    HealthCheck,
    HealthStatus {
        uptime_ms: u64,
        active_requests: u32,
        total_requests: u64,
    },
}

impl Message {
    pub fn message_type(&self) -> u8 {
        match self {
            Message::Handshake { .. } => MSG_HANDSHAKE,
            Message::HandshakeAck { .. } => MSG_HANDSHAKE_ACK,
            Message::Shutdown => MSG_SHUTDOWN,
            Message::ShutdownAck => MSG_SHUTDOWN_ACK,
            Message::ListExports => MSG_LIST_EXPORTS,
            Message::ListExportsResult { .. } => MSG_LIST_EXPORTS_RESULT,
            Message::Invoke { .. } => MSG_INVOKE,
            Message::InvokeResult { .. } => MSG_INVOKE_RESULT,
            Message::InvokeError { .. } => MSG_INVOKE_ERROR,
            Message::StreamStart { .. } => MSG_STREAM_START,
            Message::StreamChunk { .. } => MSG_STREAM_CHUNK,
            Message::StreamEnd { .. } => MSG_STREAM_END,
            Message::StreamError { .. } => MSG_STREAM_ERROR,
            Message::StreamAck { .. } => MSG_STREAM_ACK,
            Message::Cancel { .. } => MSG_CANCEL,
            Message::CancelAck { .. } => MSG_CANCEL_ACK,
            Message::LogEvent { .. } => MSG_LOG_EVENT,
            Message::HealthCheck => MSG_HEALTH_CHECK,
            Message::HealthStatus { .. } => MSG_HEALTH_STATUS,
        }
    }
}

/// Splice protocol codec
///
/// Frame format:
/// ┌──────────────┬──────────────┬─────────────────────────┐
/// │ Length (4B)  │ Type (1B)    │ Payload (msgpack)       │
/// │ big-endian   │              │                         │
/// └──────────────┴──────────────┴─────────────────────────┘
pub struct SpliceCodec {
    max_frame_size: u32,
}

impl SpliceCodec {
    pub fn new(max_frame_size: u32) -> Self {
        Self { max_frame_size }
    }
}

impl Default for SpliceCodec {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_FRAME_SIZE)
    }
}

impl Decoder for SpliceCodec {
    type Item = Message;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Need at least 5 bytes for header (4 length + 1 type)
        if src.len() < 5 {
            return Ok(None);
        }

        // Read length prefix
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_be_bytes(length_bytes) as usize;

        // Check frame size
        if length > self.max_frame_size as usize {
            return Err(ProtocolError::FrameTooLarge(length));
        }

        // Wait for complete frame
        if src.len() < 5 + length {
            src.reserve(5 + length - src.len());
            return Ok(None);
        }

        // Consume header
        src.advance(4);
        let _msg_type = src.get_u8();

        // Consume payload
        let payload = src.split_to(length).freeze();

        // Deserialize message
        let message = rmp_serde::from_slice(&payload)
            .map_err(|e| ProtocolError::Serialization(e.to_string()))?;

        Ok(Some(message))
    }
}

impl Encoder<Message> for SpliceCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Serialize payload
        let payload = rmp_serde::to_vec(&item)
            .map_err(|e| ProtocolError::Serialization(e.to_string()))?;

        // Check frame size
        if payload.len() > self.max_frame_size as usize {
            return Err(ProtocolError::FrameTooLarge(payload.len()));
        }

        // Write frame
        dst.reserve(5 + payload.len());
        dst.put_u32(payload.len() as u32);
        dst.put_u8(item.message_type());
        dst.put_slice(&payload);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_codec_roundtrip() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let msg = Message::Handshake {
            protocol_version: PROTOCOL_VERSION,
            role: Role::Host,
            capabilities: CAP_STREAMING | CAP_CANCELLATION,
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
        };

        codec.encode(msg.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (msg, decoded) {
            (
                Message::Handshake { protocol_version: v1, role: r1, capabilities: c1, max_frame_size: m1 },
                Message::Handshake { protocol_version: v2, role: r2, capabilities: c2, max_frame_size: m2 },
            ) => {
                assert_eq!(v1, v2);
                assert_eq!(r1, r2);
                assert_eq!(c1, c2);
                assert_eq!(m1, m2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_frame_too_large() {
        let mut codec = SpliceCodec::new(1024);
        let mut buf = BytesMut::new();

        let large_data = vec![0u8; 2048];
        let msg = Message::InvokeResult {
            request_id: 1,
            result: Bytes::from(large_data),
            duration_us: 100,
        };

        let result = codec.encode(msg, &mut buf);
        assert!(matches!(result, Err(ProtocolError::FrameTooLarge(_))));
    }
}
