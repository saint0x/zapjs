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

    // ========== Helper Module ==========
    mod helpers {
        use super::*;

        pub fn create_minimal_context() -> RequestContext {
            RequestContext {
                trace_id: 1,
                span_id: 1,
                headers: vec![],
                auth: None,
            }
        }

        pub fn create_full_context() -> RequestContext {
            RequestContext {
                trace_id: 123456789,
                span_id: 987654321,
                headers: vec![
                    ("x-request-id".to_string(), "req-123".to_string()),
                    ("x-forwarded-for".to_string(), "1.2.3.4".to_string()),
                ],
                auth: Some(AuthContext {
                    user_id: "test-user".to_string(),
                    roles: vec!["admin".to_string()],
                }),
            }
        }

        pub fn create_test_export(name: &str) -> ExportMetadata {
            ExportMetadata {
                name: name.to_string(),
                is_async: true,
                is_streaming: false,
                params_schema: "{}".to_string(),
                return_schema: "{}".to_string(),
            }
        }

        pub fn roundtrip(msg: Message) -> Message {
            let mut codec = SpliceCodec::default();
            let mut buf = BytesMut::new();
            codec.encode(msg, &mut buf).unwrap();
            codec.decode(&mut buf).unwrap().unwrap()
        }

        pub fn create_all_message_variants() -> Vec<Message> {
            vec![
                Message::Handshake {
                    protocol_version: PROTOCOL_VERSION,
                    role: Role::Host,
                    capabilities: 0,
                    max_frame_size: DEFAULT_MAX_FRAME_SIZE,
                },
                Message::HandshakeAck {
                    protocol_version: PROTOCOL_VERSION,
                    capabilities: 0,
                    server_id: [0u8; 16],
                    export_count: 0,
                },
                Message::Shutdown,
                Message::ShutdownAck,
                Message::ListExports,
                Message::ListExportsResult { exports: vec![] },
                Message::Invoke {
                    request_id: 1,
                    function_name: "test".to_string(),
                    params: Bytes::new(),
                    deadline_ms: 1000,
                    context: create_minimal_context(),
                },
                Message::InvokeResult {
                    request_id: 1,
                    result: Bytes::new(),
                    duration_us: 100,
                },
                Message::InvokeError {
                    request_id: 1,
                    code: ERR_EXECUTION_FAILED,
                    kind: ErrorKind::System,
                    message: "error".to_string(),
                    details: None,
                },
                Message::StreamStart {
                    request_id: 1,
                    window: 100,
                },
                Message::StreamChunk {
                    request_id: 1,
                    sequence: 1,
                    data: Bytes::new(),
                },
                Message::StreamEnd {
                    request_id: 1,
                    total_chunks: 10,
                },
                Message::StreamError {
                    request_id: 1,
                    code: ERR_EXECUTION_FAILED,
                    message: "error".to_string(),
                },
                Message::StreamAck {
                    request_id: 1,
                    ack_sequence: 1,
                    window: 100,
                },
                Message::Cancel { request_id: 1 },
                Message::CancelAck { request_id: 1 },
                Message::LogEvent {
                    level: "INFO".to_string(),
                    message: "test".to_string(),
                    fields: vec![],
                },
                Message::HealthCheck,
                Message::HealthStatus {
                    uptime_ms: 1000,
                    active_requests: 0,
                    total_requests: 100,
                },
            ]
        }
    }

    // ========== Category A: Message Type Code Tests (18 tests) ==========

    #[test]
    fn test_handshake_message_type() {
        let msg = Message::Handshake {
            protocol_version: PROTOCOL_VERSION,
            role: Role::Host,
            capabilities: 0,
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
        };
        assert_eq!(msg.message_type(), MSG_HANDSHAKE);
    }

    #[test]
    fn test_handshake_ack_message_type() {
        let msg = Message::HandshakeAck {
            protocol_version: PROTOCOL_VERSION,
            capabilities: 0,
            server_id: [0u8; 16],
            export_count: 0,
        };
        assert_eq!(msg.message_type(), MSG_HANDSHAKE_ACK);
    }

    #[test]
    fn test_shutdown_message_type() {
        assert_eq!(Message::Shutdown.message_type(), MSG_SHUTDOWN);
    }

    #[test]
    fn test_shutdown_ack_message_type() {
        assert_eq!(Message::ShutdownAck.message_type(), MSG_SHUTDOWN_ACK);
    }

    #[test]
    fn test_list_exports_message_type() {
        assert_eq!(Message::ListExports.message_type(), MSG_LIST_EXPORTS);
    }

    #[test]
    fn test_list_exports_result_message_type() {
        let msg = Message::ListExportsResult { exports: vec![] };
        assert_eq!(msg.message_type(), MSG_LIST_EXPORTS_RESULT);
    }

    #[test]
    fn test_invoke_message_type() {
        let msg = Message::Invoke {
            request_id: 42,
            function_name: "test".to_string(),
            params: Bytes::from_static(b"{}"),
            deadline_ms: 5000,
            context: helpers::create_minimal_context(),
        };
        assert_eq!(msg.message_type(), MSG_INVOKE);
    }

    #[test]
    fn test_invoke_result_message_type() {
        let msg = Message::InvokeResult {
            request_id: 1,
            result: Bytes::new(),
            duration_us: 100,
        };
        assert_eq!(msg.message_type(), MSG_INVOKE_RESULT);
    }

    #[test]
    fn test_invoke_error_message_type() {
        let msg = Message::InvokeError {
            request_id: 1,
            code: ERR_EXECUTION_FAILED,
            kind: ErrorKind::System,
            message: "error".to_string(),
            details: None,
        };
        assert_eq!(msg.message_type(), MSG_INVOKE_ERROR);
    }

    #[test]
    fn test_stream_start_message_type() {
        let msg = Message::StreamStart {
            request_id: 1,
            window: 100,
        };
        assert_eq!(msg.message_type(), MSG_STREAM_START);
    }

    #[test]
    fn test_stream_chunk_message_type() {
        let msg = Message::StreamChunk {
            request_id: 1,
            sequence: 1,
            data: Bytes::new(),
        };
        assert_eq!(msg.message_type(), MSG_STREAM_CHUNK);
    }

    #[test]
    fn test_stream_end_message_type() {
        let msg = Message::StreamEnd {
            request_id: 1,
            total_chunks: 10,
        };
        assert_eq!(msg.message_type(), MSG_STREAM_END);
    }

    #[test]
    fn test_stream_error_message_type() {
        let msg = Message::StreamError {
            request_id: 1,
            code: ERR_EXECUTION_FAILED,
            message: "error".to_string(),
        };
        assert_eq!(msg.message_type(), MSG_STREAM_ERROR);
    }

    #[test]
    fn test_stream_ack_message_type() {
        let msg = Message::StreamAck {
            request_id: 1,
            ack_sequence: 1,
            window: 100,
        };
        assert_eq!(msg.message_type(), MSG_STREAM_ACK);
    }

    #[test]
    fn test_cancel_message_type() {
        let msg = Message::Cancel { request_id: 1 };
        assert_eq!(msg.message_type(), MSG_CANCEL);
    }

    #[test]
    fn test_cancel_ack_message_type() {
        let msg = Message::CancelAck { request_id: 1 };
        assert_eq!(msg.message_type(), MSG_CANCEL_ACK);
    }

    #[test]
    fn test_log_event_message_type() {
        let msg = Message::LogEvent {
            level: "INFO".to_string(),
            message: "test".to_string(),
            fields: vec![],
        };
        assert_eq!(msg.message_type(), MSG_LOG_EVENT);
    }

    #[test]
    fn test_health_check_message_type() {
        assert_eq!(Message::HealthCheck.message_type(), MSG_HEALTH_CHECK);
    }

    #[test]
    fn test_health_status_message_type() {
        let msg = Message::HealthStatus {
            uptime_ms: 1000,
            active_requests: 0,
            total_requests: 100,
        };
        assert_eq!(msg.message_type(), MSG_HEALTH_STATUS);
    }

    // ========== Category B: Codec Roundtrip Tests (18 tests) ==========

    #[test]
    fn test_roundtrip_handshake() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::Handshake {
            protocol_version: PROTOCOL_VERSION,
            role: Role::Worker,
            capabilities: CAP_STREAMING | CAP_CANCELLATION,
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
        };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
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
    fn test_roundtrip_handshake_ack() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::HandshakeAck {
            protocol_version: PROTOCOL_VERSION,
            capabilities: CAP_STREAMING,
            server_id: [0xAB; 16],
            export_count: 42,
        };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (
                Message::HandshakeAck { protocol_version: v1, capabilities: c1, server_id: s1, export_count: e1 },
                Message::HandshakeAck { protocol_version: v2, capabilities: c2, server_id: s2, export_count: e2 },
            ) => {
                assert_eq!(v1, v2);
                assert_eq!(c1, c2);
                assert_eq!(s1, s2);
                assert_eq!(e1, e2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_roundtrip_shutdown() {
        let decoded = helpers::roundtrip(Message::Shutdown);
        assert!(matches!(decoded, Message::Shutdown));
    }

    #[test]
    fn test_roundtrip_shutdown_ack() {
        let decoded = helpers::roundtrip(Message::ShutdownAck);
        assert!(matches!(decoded, Message::ShutdownAck));
    }

    #[test]
    fn test_roundtrip_list_exports() {
        let decoded = helpers::roundtrip(Message::ListExports);
        assert!(matches!(decoded, Message::ListExports));
    }

    #[test]
    fn test_roundtrip_list_exports_result() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::ListExportsResult {
            exports: vec![
                helpers::create_test_export("func1"),
                helpers::create_test_export("func2"),
            ],
        };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (Message::ListExportsResult { exports: e1 }, Message::ListExportsResult { exports: e2 }) => {
                assert_eq!(e1.len(), e2.len());
                assert_eq!(e1[0].name, e2[0].name);
                assert_eq!(e1[1].name, e2[1].name);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_roundtrip_invoke() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::Invoke {
            request_id: 12345,
            function_name: "complex_function".to_string(),
            params: Bytes::from_static(b"{\"key\":\"value\"}"),
            deadline_ms: 30000,
            context: helpers::create_full_context(),
        };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (
                Message::Invoke { request_id: r1, function_name: f1, params: p1, deadline_ms: d1, context: c1 },
                Message::Invoke { request_id: r2, function_name: f2, params: p2, deadline_ms: d2, context: c2 },
            ) => {
                assert_eq!(r1, r2);
                assert_eq!(f1, f2);
                assert_eq!(p1, p2);
                assert_eq!(d1, d2);
                assert_eq!(c1.trace_id, c2.trace_id);
                assert_eq!(c1.span_id, c2.span_id);
                assert_eq!(c1.headers.len(), c2.headers.len());
                assert_eq!(c1.auth.as_ref().unwrap().user_id, c2.auth.as_ref().unwrap().user_id);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_roundtrip_invoke_result() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::InvokeResult {
            request_id: 999,
            result: Bytes::from_static(b"{\"success\":true}"),
            duration_us: 12345,
        };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (
                Message::InvokeResult { request_id: r1, result: res1, duration_us: d1 },
                Message::InvokeResult { request_id: r2, result: res2, duration_us: d2 },
            ) => {
                assert_eq!(r1, r2);
                assert_eq!(res1, res2);
                assert_eq!(d1, d2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_roundtrip_invoke_error() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::InvokeError {
            request_id: 555,
            code: ERR_FUNCTION_NOT_FOUND,
            kind: ErrorKind::User,
            message: "Function not found".to_string(),
            details: Some(Bytes::from_static(b"extra details")),
        };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (
                Message::InvokeError { request_id: r1, code: c1, kind: k1, message: m1, details: d1 },
                Message::InvokeError { request_id: r2, code: c2, kind: k2, message: m2, details: d2 },
            ) => {
                assert_eq!(r1, r2);
                assert_eq!(c1, c2);
                assert_eq!(k1, k2);
                assert_eq!(m1, m2);
                assert_eq!(d1, d2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_roundtrip_stream_start() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::StreamStart {
            request_id: 100,
            window: 1024,
        };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (Message::StreamStart { request_id: r1, window: w1 }, Message::StreamStart { request_id: r2, window: w2 }) => {
                assert_eq!(r1, r2);
                assert_eq!(w1, w2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_roundtrip_stream_chunk() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::StreamChunk {
            request_id: 100,
            sequence: 42,
            data: Bytes::from_static(b"chunk data"),
        };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (
                Message::StreamChunk { request_id: r1, sequence: s1, data: d1 },
                Message::StreamChunk { request_id: r2, sequence: s2, data: d2 },
            ) => {
                assert_eq!(r1, r2);
                assert_eq!(s1, s2);
                assert_eq!(d1, d2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_roundtrip_stream_end() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::StreamEnd {
            request_id: 100,
            total_chunks: 99,
        };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (Message::StreamEnd { request_id: r1, total_chunks: t1 }, Message::StreamEnd { request_id: r2, total_chunks: t2 }) => {
                assert_eq!(r1, r2);
                assert_eq!(t1, t2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_roundtrip_stream_error() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::StreamError {
            request_id: 100,
            code: ERR_TIMEOUT,
            message: "Stream timeout".to_string(),
        };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (
                Message::StreamError { request_id: r1, code: c1, message: m1 },
                Message::StreamError { request_id: r2, code: c2, message: m2 },
            ) => {
                assert_eq!(r1, r2);
                assert_eq!(c1, c2);
                assert_eq!(m1, m2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_roundtrip_stream_ack() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::StreamAck {
            request_id: 100,
            ack_sequence: 42,
            window: 512,
        };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (
                Message::StreamAck { request_id: r1, ack_sequence: a1, window: w1 },
                Message::StreamAck { request_id: r2, ack_sequence: a2, window: w2 },
            ) => {
                assert_eq!(r1, r2);
                assert_eq!(a1, a2);
                assert_eq!(w1, w2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_roundtrip_cancel() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::Cancel { request_id: 777 };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (Message::Cancel { request_id: r1 }, Message::Cancel { request_id: r2 }) => {
                assert_eq!(r1, r2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_roundtrip_cancel_ack() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::CancelAck { request_id: 777 };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (Message::CancelAck { request_id: r1 }, Message::CancelAck { request_id: r2 }) => {
                assert_eq!(r1, r2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_roundtrip_log_event() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::LogEvent {
            level: "ERROR".to_string(),
            message: "Something went wrong".to_string(),
            fields: vec![
                ("module".to_string(), "server".to_string()),
                ("line".to_string(), "42".to_string()),
            ],
        };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (
                Message::LogEvent { level: l1, message: m1, fields: f1 },
                Message::LogEvent { level: l2, message: m2, fields: f2 },
            ) => {
                assert_eq!(l1, l2);
                assert_eq!(m1, m2);
                assert_eq!(f1, f2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_roundtrip_health_check() {
        let decoded = helpers::roundtrip(Message::HealthCheck);
        assert!(matches!(decoded, Message::HealthCheck));
    }

    #[test]
    fn test_roundtrip_health_status() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let original = Message::HealthStatus {
            uptime_ms: 123456789,
            active_requests: 10,
            total_requests: 1000000,
        };

        codec.encode(original.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match (original, decoded) {
            (
                Message::HealthStatus { uptime_ms: u1, active_requests: a1, total_requests: t1 },
                Message::HealthStatus { uptime_ms: u2, active_requests: a2, total_requests: t2 },
            ) => {
                assert_eq!(u1, u2);
                assert_eq!(a1, a2);
                assert_eq!(t1, t2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    // ========== Category C: Framing Structure Tests (8 tests) ==========

    #[test]
    fn test_frame_length_prefix() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let msg = Message::HealthCheck;
        codec.encode(msg, &mut buf).unwrap();

        // Read length prefix
        let length_bytes = &buf[0..4];
        let length = u32::from_be_bytes([length_bytes[0], length_bytes[1], length_bytes[2], length_bytes[3]]);

        // Verify total frame size = 4 (length) + 1 (type) + payload
        assert_eq!(buf.len(), 4 + 1 + (length as usize));
    }

    #[test]
    fn test_frame_type_byte() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let msg = Message::Invoke {
            request_id: 1,
            function_name: "test".to_string(),
            params: Bytes::new(),
            deadline_ms: 1000,
            context: helpers::create_minimal_context(),
        };

        let expected_type = msg.message_type();
        codec.encode(msg, &mut buf).unwrap();

        // Type byte is at offset 4
        assert_eq!(buf[4], expected_type);
    }

    #[test]
    fn test_frame_minimal_size() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        // Unit variants should produce smallest frames
        codec.encode(Message::Shutdown, &mut buf).unwrap();

        // At minimum: 4 bytes (length) + 1 byte (type) + minimal msgpack payload
        assert!(buf.len() >= 5);
        assert!(buf.len() < 20); // Should be quite small
    }

    #[test]
    fn test_frame_with_empty_bytes() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let msg = Message::InvokeResult {
            request_id: 1,
            result: Bytes::new(),
            duration_us: 100,
        };

        codec.encode(msg, &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match decoded {
            Message::InvokeResult { result, .. } => {
                assert_eq!(result.len(), 0);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_frame_with_empty_vec() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let msg = Message::ListExportsResult { exports: vec![] };

        codec.encode(msg, &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match decoded {
            Message::ListExportsResult { exports } => {
                assert_eq!(exports.len(), 0);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_frame_boundary_at_5_bytes() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let msg = Message::HealthCheck;
        codec.encode(msg, &mut buf).unwrap();

        // Take only header bytes
        let partial = buf.split_to(4);

        // Decoder should return None for incomplete header
        let mut decode_buf = partial;
        let result = codec.decode(&mut decode_buf).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_partial_frame_decode() {
        let mut codec = SpliceCodec::default();
        let mut encode_buf = BytesMut::new();

        let msg = Message::HealthCheck;
        codec.encode(msg, &mut encode_buf).unwrap();

        // Split frame in middle
        let total_len = encode_buf.len();
        let partial = encode_buf.split_to(total_len / 2);

        // Decoder should return Ok(None) for partial frame
        let mut decode_buf = BytesMut::from(&partial[..]);
        let result = codec.decode(&mut decode_buf).unwrap();
        assert!(result.is_none());

        // Add remaining bytes
        decode_buf.extend_from_slice(&encode_buf);
        let result = codec.decode(&mut decode_buf).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_multiple_frames_in_buffer() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        // Encode three messages
        codec.encode(Message::HealthCheck, &mut buf).unwrap();
        codec.encode(Message::Shutdown, &mut buf).unwrap();
        codec.encode(Message::ShutdownAck, &mut buf).unwrap();

        // Decode all three
        let msg1 = codec.decode(&mut buf).unwrap().unwrap();
        let msg2 = codec.decode(&mut buf).unwrap().unwrap();
        let msg3 = codec.decode(&mut buf).unwrap().unwrap();

        assert!(matches!(msg1, Message::HealthCheck));
        assert!(matches!(msg2, Message::Shutdown));
        assert!(matches!(msg3, Message::ShutdownAck));

        // Buffer should be empty
        assert_eq!(buf.len(), 0);
    }

    // ========== Category D: Edge Cases & Boundaries (12 tests) ==========

    #[test]
    fn test_max_frame_size_exact() {
        let max_size = 10000u32;
        let mut codec = SpliceCodec::new(max_size);
        let mut buf = BytesMut::new();

        // Create a message close to the limit
        let test_data_size = (max_size - 200) as usize;
        let msg = Message::StreamChunk {
            request_id: 1,
            sequence: 1,
            data: Bytes::from(vec![0u8; test_data_size]),
        };

        // Should succeed
        let result = codec.encode(msg, &mut buf);
        assert!(result.is_ok());
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

    #[test]
    fn test_zero_length_strings() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        let msg = Message::Invoke {
            request_id: 1,
            function_name: "".to_string(),
            params: Bytes::new(),
            deadline_ms: 1000,
            context: helpers::create_minimal_context(),
        };

        codec.encode(msg.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        match decoded {
            Message::Invoke { function_name, .. } => {
                assert_eq!(function_name, "");
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_zero_length_bytes() {
        let msg = Message::InvokeResult {
            request_id: 1,
            result: Bytes::new(),
            duration_us: 100,
        };

        let decoded = helpers::roundtrip(msg);

        match decoded {
            Message::InvokeResult { result, .. } => {
                assert_eq!(result.len(), 0);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_empty_vecs() {
        let msg = Message::Invoke {
            request_id: 1,
            function_name: "test".to_string(),
            params: Bytes::new(),
            deadline_ms: 1000,
            context: RequestContext {
                trace_id: 1,
                span_id: 1,
                headers: vec![],
                auth: None,
            },
        };

        let decoded = helpers::roundtrip(msg);

        match decoded {
            Message::Invoke { context, .. } => {
                assert_eq!(context.headers.len(), 0);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_maximum_u64_values() {
        let msg = Message::Invoke {
            request_id: u64::MAX,
            function_name: "test".to_string(),
            params: Bytes::new(),
            deadline_ms: 1000,
            context: RequestContext {
                trace_id: u64::MAX,
                span_id: u64::MAX,
                headers: vec![],
                auth: None,
            },
        };

        let decoded = helpers::roundtrip(msg);

        match decoded {
            Message::Invoke { request_id, context, .. } => {
                assert_eq!(request_id, u64::MAX);
                assert_eq!(context.trace_id, u64::MAX);
                assert_eq!(context.span_id, u64::MAX);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_maximum_u32_values() {
        let msg = Message::Handshake {
            protocol_version: u32::MAX,
            role: Role::Host,
            capabilities: u32::MAX,
            max_frame_size: u32::MAX,
        };

        let decoded = helpers::roundtrip(msg);

        match decoded {
            Message::Handshake { protocol_version, capabilities, max_frame_size, .. } => {
                assert_eq!(protocol_version, u32::MAX);
                assert_eq!(capabilities, u32::MAX);
                assert_eq!(max_frame_size, u32::MAX);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_server_id_all_zeros() {
        let msg = Message::HandshakeAck {
            protocol_version: PROTOCOL_VERSION,
            capabilities: 0,
            server_id: [0u8; 16],
            export_count: 0,
        };

        let decoded = helpers::roundtrip(msg);

        match decoded {
            Message::HandshakeAck { server_id, .. } => {
                assert_eq!(server_id, [0u8; 16]);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_server_id_all_ones() {
        let msg = Message::HandshakeAck {
            protocol_version: PROTOCOL_VERSION,
            capabilities: 0,
            server_id: [0xFF; 16],
            export_count: 0,
        };

        let decoded = helpers::roundtrip(msg);

        match decoded {
            Message::HandshakeAck { server_id, .. } => {
                assert_eq!(server_id, [0xFF; 16]);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_very_large_vec_fields() {
        let mut headers = vec![];
        for i in 0..1000 {
            headers.push((format!("header{}", i), format!("value{}", i)));
        }

        let msg = Message::Invoke {
            request_id: 1,
            function_name: "test".to_string(),
            params: Bytes::new(),
            deadline_ms: 1000,
            context: RequestContext {
                trace_id: 1,
                span_id: 1,
                headers: headers.clone(),
                auth: None,
            },
        };

        let decoded = helpers::roundtrip(msg);

        match decoded {
            Message::Invoke { context, .. } => {
                assert_eq!(context.headers.len(), 1000);
                assert_eq!(context.headers[0], headers[0]);
                assert_eq!(context.headers[999], headers[999]);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_unicode_strings() {
        let msg = Message::LogEvent {
            level: "INFO".to_string(),
            message: "Hello 世界 🌍 Привет".to_string(),
            fields: vec![
                ("emoji".to_string(), "🚀🎉".to_string()),
                ("chinese".to_string(), "你好".to_string()),
            ],
        };

        let decoded = helpers::roundtrip(msg.clone());

        match (msg, decoded) {
            (Message::LogEvent { message: m1, fields: f1, .. }, Message::LogEvent { message: m2, fields: f2, .. }) => {
                assert_eq!(m1, m2);
                assert_eq!(f1, f2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    #[test]
    fn test_special_characters() {
        let msg = Message::InvokeError {
            request_id: 1,
            code: ERR_EXECUTION_FAILED,
            kind: ErrorKind::System,
            message: "Error with\nnewline\tand\ttabs".to_string(),
            details: None,
        };

        let decoded = helpers::roundtrip(msg.clone());

        match (msg, decoded) {
            (Message::InvokeError { message: m1, .. }, Message::InvokeError { message: m2, .. }) => {
                assert_eq!(m1, m2);
            }
            _ => panic!("Message type mismatch"),
        }
    }

    // ========== Category E: Error Conditions (10 tests) ==========

    #[test]
    fn test_invalid_msgpack_data() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        // Manually construct invalid frame
        let invalid_payload = vec![0xFF, 0xFF, 0xFF];
        buf.put_u32(invalid_payload.len() as u32);
        buf.put_u8(MSG_HANDSHAKE);
        buf.put_slice(&invalid_payload);

        let result = codec.decode(&mut buf);
        assert!(matches!(result, Err(ProtocolError::Serialization(_))));
    }

    #[test]
    fn test_truncated_msgpack() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        // Encode a valid message
        codec.encode(Message::HealthCheck, &mut buf).unwrap();

        // Truncate the payload
        let total_len = buf.len();
        buf.truncate(total_len - 2);

        let result = codec.decode(&mut buf);
        // Should either return None (waiting for more data) or Serialization error
        assert!(result.is_err() || result.unwrap().is_none());
    }

    #[test]
    fn test_frame_size_zero() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        // Construct frame with zero length
        buf.put_u32(0);
        buf.put_u8(MSG_HEALTH_CHECK);

        let result = codec.decode(&mut buf);
        // Zero-length payload might cause deserialization error
        assert!(result.is_err() || result.unwrap().is_some());
    }

    #[test]
    fn test_decoder_with_garbage_prefix() {
        let mut codec = SpliceCodec::default();

        // Create a valid message
        let mut valid_buf = BytesMut::new();
        codec.encode(Message::HealthCheck, &mut valid_buf).unwrap();

        // Prepend garbage (less than 5 bytes)
        let mut buf = BytesMut::new();
        buf.put_slice(&[0x00, 0x01, 0x02]); // 3 garbage bytes
        buf.put_slice(&valid_buf);

        // First decode will fail or return None (not enough bytes for header)
        let result1 = codec.decode(&mut buf);
        // Either error or None
        assert!(result1.is_err() || result1.unwrap().is_none() || buf.len() > 0);
    }

    #[test]
    fn test_encoder_buffer_growth() {
        let mut codec = SpliceCodec::default();

        // Start with empty buffer
        let mut buf = BytesMut::new();
        assert_eq!(buf.capacity(), 0);

        // Encode a message
        codec.encode(Message::HealthCheck, &mut buf).unwrap();

        // Buffer should have grown
        assert!(buf.len() > 0);
        assert!(buf.capacity() >= buf.len());
    }

    #[test]
    fn test_decode_after_error() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        // Create invalid frame with complete payload but invalid msgpack
        let invalid_payload = vec![0xFF; 100];
        buf.put_u32(invalid_payload.len() as u32);
        buf.put_u8(MSG_HANDSHAKE);
        buf.put_slice(&invalid_payload);

        // First decode will error due to invalid msgpack
        let result1 = codec.decode(&mut buf);
        assert!(result1.is_err());

        // Clear buffer and try with valid message
        buf.clear();
        codec.encode(Message::HealthCheck, &mut buf).unwrap();

        // Should work fine
        let result2 = codec.decode(&mut buf).unwrap();
        assert!(result2.is_some());
    }

    #[test]
    fn test_encode_multiple_messages() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        // Encode multiple messages sequentially
        codec.encode(Message::HealthCheck, &mut buf).unwrap();
        let pos1 = buf.len();

        codec.encode(Message::Shutdown, &mut buf).unwrap();
        let pos2 = buf.len();

        codec.encode(Message::ShutdownAck, &mut buf).unwrap();
        let pos3 = buf.len();

        // All should succeed
        assert!(pos1 > 0);
        assert!(pos2 > pos1);
        assert!(pos3 > pos2);
    }

    #[test]
    fn test_length_mismatch_detection() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        // Create valid message first
        codec.encode(Message::HealthCheck, &mut buf).unwrap();

        // Corrupt the length prefix to be larger than actual data
        let corrupted_length = 9999u32;
        buf[0..4].copy_from_slice(&corrupted_length.to_be_bytes());

        let result = codec.decode(&mut buf);
        // Should return None (waiting for more data) since length indicates more bytes needed
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_sequential_encoding_different_sizes() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        // Small message
        codec.encode(Message::Shutdown, &mut buf).unwrap();

        // Medium message
        codec.encode(Message::Invoke {
            request_id: 1,
            function_name: "test_function".to_string(),
            params: Bytes::from_static(b"{\"data\":\"value\"}"),
            deadline_ms: 5000,
            context: helpers::create_minimal_context(),
        }, &mut buf).unwrap();

        // Large message
        let large_payload = vec![0u8; 10000];
        codec.encode(Message::InvokeResult {
            request_id: 2,
            result: Bytes::from(large_payload),
            duration_us: 1000,
        }, &mut buf).unwrap();

        // Decode all three
        let msg1 = codec.decode(&mut buf).unwrap().unwrap();
        let msg2 = codec.decode(&mut buf).unwrap().unwrap();
        let msg3 = codec.decode(&mut buf).unwrap().unwrap();

        assert!(matches!(msg1, Message::Shutdown));
        assert!(matches!(msg2, Message::Invoke { .. }));
        assert!(matches!(msg3, Message::InvokeResult { .. }));
    }

    #[test]
    fn test_decoder_reserve_called() {
        let mut codec = SpliceCodec::default();
        let mut buf = BytesMut::new();

        // Encode a large message
        let large_data = vec![0u8; 50000];
        codec.encode(Message::StreamChunk {
            request_id: 1,
            sequence: 1,
            data: Bytes::from(large_data),
        }, &mut buf).unwrap();

        // Split into partial frame
        let total_len = buf.len();
        let partial = buf.split_to(20); // Just the header

        // Decode partial - should call reserve
        let mut decode_buf = partial;
        let result = codec.decode(&mut decode_buf).unwrap();
        assert!(result.is_none());

        // Buffer should have reserved space
        assert!(decode_buf.capacity() > 20);
    }

    // ========== Category F: Enum Variant Tests (3 tests) ==========

    #[test]
    fn test_role_variants() {
        let mut codec = SpliceCodec::default();

        for role in [Role::Host, Role::Worker] {
            let mut buf = BytesMut::new();
            let msg = Message::Handshake {
                protocol_version: PROTOCOL_VERSION,
                role,
                capabilities: 0,
                max_frame_size: DEFAULT_MAX_FRAME_SIZE,
            };

            codec.encode(msg.clone(), &mut buf).unwrap();
            let decoded = codec.decode(&mut buf).unwrap().unwrap();

            match decoded {
                Message::Handshake { role: decoded_role, .. } => {
                    assert_eq!(role, decoded_role);
                }
                _ => panic!("Wrong message type"),
            }
        }
    }

    #[test]
    fn test_error_kind_variants() {
        let mut codec = SpliceCodec::default();

        for (idx, kind) in [ErrorKind::User, ErrorKind::System, ErrorKind::Timeout, ErrorKind::Cancelled]
            .iter()
            .enumerate()
        {
            let mut buf = BytesMut::new();
            let msg = Message::InvokeError {
                request_id: idx as u64,
                code: ERR_EXECUTION_FAILED,
                kind: *kind,
                message: "Test error".to_string(),
                details: None,
            };

            codec.encode(msg.clone(), &mut buf).unwrap();
            let decoded = codec.decode(&mut buf).unwrap().unwrap();

            match decoded {
                Message::InvokeError { kind: decoded_kind, .. } => {
                    assert_eq!(*kind, decoded_kind);
                }
                _ => panic!("Wrong message type"),
            }
        }
    }

    #[test]
    fn test_all_error_codes() {
        let error_codes = [
            ERR_INVALID_REQUEST,
            ERR_INVALID_PARAMS,
            ERR_FUNCTION_NOT_FOUND,
            ERR_UNAUTHORIZED,
            ERR_FRAME_TOO_LARGE,
            ERR_EXECUTION_FAILED,
            ERR_TIMEOUT,
            ERR_CANCELLED,
            ERR_PANIC,
            ERR_INTERNAL_ERROR,
            ERR_UNAVAILABLE,
            ERR_OVERLOADED,
        ];

        let mut codec = SpliceCodec::default();

        for code in error_codes {
            let mut buf = BytesMut::new();
            let msg = Message::InvokeError {
                request_id: code as u64,
                code,
                kind: ErrorKind::System,
                message: format!("Error {}", code),
                details: None,
            };

            codec.encode(msg, &mut buf).unwrap();
            let decoded = codec.decode(&mut buf).unwrap().unwrap();

            match decoded {
                Message::InvokeError { code: decoded_code, .. } => {
                    assert_eq!(code, decoded_code);
                }
                _ => panic!("Wrong message type"),
            }
        }
    }
}
