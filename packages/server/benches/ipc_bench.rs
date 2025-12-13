//! IPC Protocol Performance Benchmarks
//!
//! Validates the documented performance claims:
//! - IPC round-trip: ~100μs (target < 150μs)
//! - Serialization overhead: MessagePack vs JSON

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use zap_server::ipc::{IpcMessage, IpcRequest, IpcEncoding, serialize_message, deserialize_message};
use std::collections::HashMap;

/// Benchmark IPC message serialization
///
/// Tests JSON vs MessagePack encoding performance
fn bench_ipc_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipc_serialization");

    // Small message (health check)
    let small_msg = IpcMessage::HealthCheck;

    // Medium message (invoke handler)
    let medium_msg = IpcMessage::InvokeHandler {
        handler_id: "handler_123".to_string(),
        request: IpcRequest {
            request_id: "req-456".to_string(),
            method: "GET".to_string(),
            path: "/api/users/123".to_string(),
            path_only: "/api/users/123".to_string(),
            query: HashMap::new(),
            params: {
                let mut m = HashMap::new();
                m.insert("id".to_string(), "123".to_string());
                m
            },
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "application/json".to_string());
                h
            },
            body: String::new(),
            cookies: HashMap::new(),
        },
    };

    // Large message (handler response with data)
    let large_msg = IpcMessage::HandlerResponse {
        handler_id: "handler_789".to_string(),
        status: 200,
        headers: {
            let mut h = HashMap::new();
            h.insert("content-type".to_string(), "application/json".to_string());
            h
        },
        body: serde_json::json!({
            "items": (0..100).map(|i| serde_json::json!({
                "id": i,
                "name": format!("Item {}", i),
                "price": i as f64 * 9.99
            })).collect::<Vec<_>>(),
            "total": 100
        }).to_string(),
    };

    // JSON serialization - small
    group.throughput(Throughput::Elements(1));
    group.bench_function("json_small", |b| {
        b.iter(|| {
            let bytes = serialize_message(black_box(&small_msg), IpcEncoding::Json).unwrap();
            black_box(bytes)
        })
    });

    // JSON deserialization - small
    let small_json = serialize_message(&small_msg, IpcEncoding::Json).unwrap();
    group.bench_function("json_small_decode", |b| {
        b.iter(|| {
            let msg = deserialize_message(black_box(&small_json)).unwrap();
            black_box(msg)
        })
    });

    // MessagePack serialization - small
    group.bench_function("msgpack_small", |b| {
        b.iter(|| {
            let bytes = serialize_message(black_box(&small_msg), IpcEncoding::MessagePack).unwrap();
            black_box(bytes)
        })
    });

    // MessagePack deserialization - small
    let small_msgpack = serialize_message(&small_msg, IpcEncoding::MessagePack).unwrap();
    group.bench_function("msgpack_small_decode", |b| {
        b.iter(|| {
            let msg = deserialize_message(black_box(&small_msgpack)).unwrap();
            black_box(msg)
        })
    });

    // JSON - medium
    group.bench_function("json_medium", |b| {
        b.iter(|| {
            let bytes = serialize_message(black_box(&medium_msg), IpcEncoding::Json).unwrap();
            black_box(bytes)
        })
    });

    // MessagePack - medium
    group.bench_function("msgpack_medium", |b| {
        b.iter(|| {
            let bytes = serialize_message(black_box(&medium_msg), IpcEncoding::MessagePack).unwrap();
            black_box(bytes)
        })
    });

    // JSON - large
    group.bench_function("json_large", |b| {
        b.iter(|| {
            let bytes = serialize_message(black_box(&large_msg), IpcEncoding::Json).unwrap();
            black_box(bytes)
        })
    });

    // MessagePack - large
    group.bench_function("msgpack_large", |b| {
        b.iter(|| {
            let bytes = serialize_message(black_box(&large_msg), IpcEncoding::MessagePack).unwrap();
            black_box(bytes)
        })
    });

    group.finish();
}

/// Benchmark IPC message sizes
///
/// Tests how message size affects serialization performance
fn bench_ipc_message_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipc_message_sizes");

    let sizes = [
        ("tiny", 10),
        ("small", 100),
        ("medium", 1000),
        ("large", 10000),
    ];

    for (name, item_count) in sizes.iter() {
        let body = serde_json::json!({
            "items": (0..*item_count).map(|i| serde_json::json!({"id": i})).collect::<Vec<_>>()
        }).to_string();

        let msg = IpcMessage::HandlerResponse {
            handler_id: format!("handler_{}", name),
            status: 200,
            headers: HashMap::new(),
            body,
        };

        group.bench_with_input(
            BenchmarkId::new("json", name),
            &msg,
            |b, msg| {
                b.iter(|| {
                    let bytes = serialize_message(black_box(msg), IpcEncoding::Json).unwrap();
                    black_box(bytes)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("msgpack", name),
            &msg,
            |b, msg| {
                b.iter(|| {
                    let bytes = serialize_message(black_box(msg), IpcEncoding::MessagePack).unwrap();
                    black_box(bytes)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark IPC request creation
///
/// Tests the overhead of creating IPC requests
fn bench_ipc_request_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipc_request_creation");

    // Simple GET request
    group.bench_function("simple_get", |b| {
        b.iter(|| {
            let req = IpcRequest {
                request_id: black_box("req-123".to_string()),
                method: black_box("GET".to_string()),
                path: black_box("/api/users".to_string()),
                path_only: black_box("/api/users".to_string()),
                query: black_box(HashMap::new()),
                params: black_box(HashMap::new()),
                headers: black_box(HashMap::new()),
                body: black_box(String::new()),
                cookies: black_box(HashMap::new()),
            };
            black_box(req)
        })
    });

    // POST with body
    group.bench_function("post_with_body", |b| {
        b.iter(|| {
            let req = IpcRequest {
                request_id: black_box("req-456".to_string()),
                method: black_box("POST".to_string()),
                path: black_box("/api/users".to_string()),
                path_only: black_box("/api/users".to_string()),
                query: black_box(HashMap::new()),
                params: black_box(HashMap::new()),
                headers: black_box({
                    let mut h = HashMap::new();
                    h.insert("content-type".to_string(), "application/json".to_string());
                    h
                }),
                body: black_box(r#"{"name":"John Doe","email":"john@example.com"}"#.to_string()),
                cookies: black_box(HashMap::new()),
            };
            black_box(req)
        })
    });

    // Request with many headers
    group.bench_function("many_headers", |b| {
        b.iter(|| {
            let req = IpcRequest {
                request_id: black_box("req-789".to_string()),
                method: black_box("GET".to_string()),
                path: black_box("/api/users".to_string()),
                path_only: black_box("/api/users".to_string()),
                query: black_box(HashMap::new()),
                params: black_box(HashMap::new()),
                headers: black_box({
                    let mut h = HashMap::new();
                    for i in 0..20 {
                        h.insert(format!("x-custom-{}", i), format!("value-{}", i));
                    }
                    h
                }),
                body: black_box(String::new()),
                cookies: black_box(HashMap::new()),
            };
            black_box(req)
        })
    });

    group.finish();
}

/// Benchmark full serialization round-trip
///
/// Tests encode -> decode cycle for different message types
fn bench_ipc_round_trip(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipc_round_trip");

    let messages = vec![
        ("health_check", IpcMessage::HealthCheck),
        ("handler_response", IpcMessage::HandlerResponse {
            handler_id: "handler_1".to_string(),
            status: 200,
            headers: HashMap::new(),
            body: r#"{"message":"Success"}"#.to_string(),
        }),
        ("error_response", IpcMessage::Error {
            code: "NOT_FOUND".to_string(),
            message: "Resource not found".to_string(),
            status: 404,
            digest: "err-123".to_string(),
            details: None,
        }),
        ("invoke_handler", IpcMessage::InvokeHandler {
            handler_id: "handler_2".to_string(),
            request: IpcRequest {
                request_id: "req-1".to_string(),
                method: "GET".to_string(),
                path: "/api/users".to_string(),
                path_only: "/api/users".to_string(),
                query: HashMap::new(),
                params: HashMap::new(),
                headers: HashMap::new(),
                body: String::new(),
                cookies: HashMap::new(),
            },
        }),
    ];

    for (name, msg) in messages.iter() {
        // JSON round-trip
        group.bench_function(format!("json_{}", name), |b| {
            b.iter(|| {
                let encoded = serialize_message(black_box(msg), IpcEncoding::Json).unwrap();
                let decoded = deserialize_message(black_box(&encoded)).unwrap();
                black_box(decoded)
            })
        });

        // MessagePack round-trip
        group.bench_function(format!("msgpack_{}", name), |b| {
            b.iter(|| {
                let encoded = serialize_message(black_box(msg), IpcEncoding::MessagePack).unwrap();
                let decoded = deserialize_message(black_box(&encoded)).unwrap();
                black_box(decoded)
            })
        });
    }

    group.finish();
}

/// Benchmark IPC frame protocol overhead
///
/// Tests the framing protocol (length prefix + data) overhead
fn bench_ipc_framing(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipc_framing");

    let msg = IpcMessage::HealthCheck;

    // Encode with frame header
    group.bench_function("encode_with_frame", |b| {
        b.iter(|| {
            let payload = serialize_message(black_box(&msg), IpcEncoding::Json).unwrap();
            let len = (payload.len() as u32).to_be_bytes();
            let mut frame = Vec::with_capacity(4 + payload.len());
            frame.extend_from_slice(&len);
            frame.extend_from_slice(&payload);
            black_box(frame)
        })
    });

    // Parse frame header
    let payload = serialize_message(&msg, IpcEncoding::Json).unwrap();
    let len = (payload.len() as u32).to_be_bytes();
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&len);
    frame.extend_from_slice(&payload);

    group.bench_function("parse_frame_header", |b| {
        b.iter(|| {
            let frame_ref = black_box(&frame);
            let len_bytes: [u8; 4] = frame_ref[0..4].try_into().unwrap();
            let len = u32::from_be_bytes(len_bytes);
            black_box(len)
        })
    });

    group.finish();
}

/// Benchmark different IPC message types
///
/// Tests serialization performance across various message types
fn bench_ipc_message_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("ipc_message_types");

    // Health check message
    let health_check = IpcMessage::HealthCheck;
    group.bench_function("health_check", |b| {
        b.iter(|| {
            let bytes = serialize_message(black_box(&health_check), IpcEncoding::Json).unwrap();
            black_box(bytes)
        })
    });

    // Handler response message
    let handler_response = IpcMessage::HandlerResponse {
        handler_id: "handler_1".to_string(),
        status: 200,
        headers: HashMap::new(),
        body: r#"{"result":"ok"}"#.to_string(),
    };
    group.bench_function("handler_response", |b| {
        b.iter(|| {
            let bytes = serialize_message(black_box(&handler_response), IpcEncoding::Json).unwrap();
            black_box(bytes)
        })
    });

    // Error message
    let error = IpcMessage::Error {
        code: "INTERNAL_ERROR".to_string(),
        message: "Internal server error".to_string(),
        status: 500,
        digest: "err-456".to_string(),
        details: None,
    };
    group.bench_function("error", |b| {
        b.iter(|| {
            let bytes = serialize_message(black_box(&error), IpcEncoding::Json).unwrap();
            black_box(bytes)
        })
    });

    // Invoke handler message
    let invoke = IpcMessage::InvokeHandler {
        handler_id: "handler_2".to_string(),
        request: IpcRequest {
            request_id: "req-1".to_string(),
            method: "GET".to_string(),
            path: "/api".to_string(),
            path_only: "/api".to_string(),
            query: HashMap::new(),
            params: HashMap::new(),
            headers: HashMap::new(),
            body: String::new(),
            cookies: HashMap::new(),
        },
    };
    group.bench_function("invoke_handler", |b| {
        b.iter(|| {
            let bytes = serialize_message(black_box(&invoke), IpcEncoding::Json).unwrap();
            black_box(bytes)
        })
    });

    // Stream start message
    let stream_start = IpcMessage::StreamStart {
        stream_id: "stream-123".to_string(),
        status: 200,
        headers: HashMap::new(),
    };
    group.bench_function("stream_start", |b| {
        b.iter(|| {
            let bytes = serialize_message(black_box(&stream_start), IpcEncoding::Json).unwrap();
            black_box(bytes)
        })
    });

    // WebSocket message
    let ws_message = IpcMessage::WsMessage {
        connection_id: "ws-456".to_string(),
        handler_id: "ws_handler_1".to_string(),
        data: "Hello".to_string(),
        binary: false,
    };
    group.bench_function("ws_message", |b| {
        b.iter(|| {
            let bytes = serialize_message(black_box(&ws_message), IpcEncoding::Json).unwrap();
            black_box(bytes)
        })
    });

    group.finish();
}

criterion_group!(
    ipc_benches,
    bench_ipc_serialization,
    bench_ipc_message_sizes,
    bench_ipc_request_creation,
    bench_ipc_round_trip,
    bench_ipc_framing,
    bench_ipc_message_types
);
criterion_main!(ipc_benches);
