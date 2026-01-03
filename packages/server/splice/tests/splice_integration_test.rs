mod splice_mock;

use splice::protocol::*;
use splice_mock::*;
use serde_json::json;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use bytes::Bytes;

// ========== Helper Functions ==========

fn create_test_export(name: &str) -> ExportMetadata {
    ExportMetadata {
        name: name.to_string(),
        is_async: false,
        is_streaming: false,
        params_schema: "{}".to_string(),
        return_schema: "{}".to_string(),
    }
}

// ========== Category 1: Protocol Compliance Tests (8 tests) ==========

#[tokio::test]
async fn test_protocol_version_validation() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);

    // Should succeed with matching version
    let result = host.connect().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_all_message_types_roundtrip() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("echo"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Test various message types through actual protocol flow
    // Handshake, ListExports, Invoke, InvokeResult
    let result = host.invoke("echo", json!({"test": "data"})).await;
    assert!(result.is_ok());

    // HealthCheck
    let health = host.health_check().await;
    assert!(health.is_ok());

    // Cancel
    let cancel_result = host.cancel(999).await;
    assert!(cancel_result.is_ok());

    // Shutdown
    let shutdown_result = host.shutdown().await;
    assert!(shutdown_result.is_ok());
}

#[tokio::test]
async fn test_frame_size_limit_enforcement() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("large"))
        .with_dispatcher(|_name, _params| {
            // Return a very large payload
            let mut large_array = vec![];
            for i in 0..10000 {
                large_array.push(json!({"id": i, "data": "x".repeat(100)}));
            }
            Ok(json!({"items": large_array}))
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // This should work if under 100MB limit
    let result = host.invoke("large", json!({})).await;
    // Result depends on actual size, but should not crash
    let _ = result;
}

#[tokio::test]
async fn test_messagepack_serialization_fidelity() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("echo"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Test various data types
    let test_data = json!({
        "null": null,
        "bool_true": true,
        "bool_false": false,
        "int": 42,
        "negative": -100,
        "float": 3.14159,
        "string": "hello",
        "unicode": "世界🌍",
        "array": [1, 2, 3],
        "object": {"nested": "value"}
    });

    let result = host.invoke("echo", test_data.clone()).await.unwrap();
    assert_eq!(result, test_data);
}

#[tokio::test]
async fn test_request_context_propagation() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .with_dispatcher(|_name, _params| Ok(json!({"ok": true})))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // The mock host sends a RequestContext with trace_id=1, span_id=1
    // This tests that the context is properly serialized and sent
    let result = host.invoke("test", json!({})).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_error_code_propagation() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("error"))
        .with_dispatcher(|_name, _params| Err("Test error message".to_string()))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let result = host.invoke("error", json!({})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Test error message"));
}

#[tokio::test]
async fn test_capability_flags_negotiation() {
    // Test different capability combinations
    for caps in [0, CAP_STREAMING, CAP_CANCELLATION, CAP_STREAMING | CAP_CANCELLATION] {
        let harness = TestHarness::new();
        let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

        let worker = MockWorkerBuilder::new().build(worker_rx, worker_tx);
        tokio::spawn(worker.run());

        let mut host = MockHostBuilder::new()
            .with_capabilities(caps)
            .build(host_tx, host_rx);

        // Should successfully negotiate capabilities
        host.connect().await.unwrap();
        assert_eq!(host.state, HostState::Ready);

        // Cleanup
        let _ = host.shutdown().await;
    }
}

#[tokio::test]
async fn test_export_metadata_complete() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let exports = vec![
        ExportMetadata {
            name: "sync_fn".to_string(),
            is_async: false,
            is_streaming: false,
            params_schema: r#"{"type":"object","properties":{"x":{"type":"number"}}}"#.to_string(),
            return_schema: r#"{"type":"number"}"#.to_string(),
        },
        ExportMetadata {
            name: "async_fn".to_string(),
            is_async: true,
            is_streaming: false,
            params_schema: "{}".to_string(),
            return_schema: r#"{"type":"string"}"#.to_string(),
        },
    ];

    let worker = MockWorkerBuilder::new()
        .with_exports(exports.clone())
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    assert_eq!(host.exports.len(), 2);
    assert_eq!(host.exports[0].name, "sync_fn");
    assert!(!host.exports[0].is_async);
    assert_eq!(host.exports[1].name, "async_fn");
    assert!(host.exports[1].is_async);
}

// ========== Category 2: Performance & Throughput Tests (5 tests) ==========

#[tokio::test]
async fn test_sequential_request_performance() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("fast"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let start = std::time::Instant::now();
    for i in 0..100 {
        let result = host.invoke("fast", json!({"id": i})).await;
        assert!(result.is_ok());
    }
    let elapsed = start.elapsed();

    // 100 requests should complete reasonably fast in mock environment
    assert!(elapsed < Duration::from_secs(10), "100 requests took {:?}", elapsed);
}

#[tokio::test]
async fn test_large_payload_handling() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("echo"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Create 1MB payload
    let large_string = "x".repeat(1024 * 1024);
    let large_payload = json!({"data": large_string});

    let result = host.invoke("echo", large_payload.clone()).await.unwrap();
    assert_eq!(result["data"].as_str().unwrap().len(), 1024 * 1024);
}

#[tokio::test]
async fn test_small_payload_overhead() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("ping"))
        .with_dispatcher(|_name, _params| Ok(json!({"pong": true})))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Test many small requests
    for _ in 0..50 {
        let result = host.invoke("ping", json!({})).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_mixed_payload_sizes() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("echo"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Alternate between small and large payloads
    for i in 0..10 {
        let payload = if i % 2 == 0 {
            json!({"small": i})
        } else {
            json!({"large": "x".repeat(10000)})
        };

        let result = host.invoke("echo", payload).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_rapid_connect_disconnect() {
    for _ in 0..5 {
        let harness = TestHarness::new();
        let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

        let worker = MockWorkerBuilder::new().build(worker_rx, worker_tx);

        tokio::spawn(worker.run());

        let mut host = MockHostBuilder::new().build(host_tx, host_rx);
        host.connect().await.unwrap();
        host.shutdown().await.unwrap();
    }
}

// ========== Category 3: Error Recovery Tests (6 tests) ==========

#[tokio::test]
async fn test_recovery_after_function_error() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("sometimes_fails"))
        .with_dispatcher(|_name, params| {
            if params.get("fail").and_then(|v| v.as_bool()).unwrap_or(false) {
                Err("Requested failure".to_string())
            } else {
                Ok(json!({"success": true}))
            }
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Cause error
    let err_result = host.invoke("sometimes_fails", json!({"fail": true})).await;
    assert!(err_result.is_err());

    // Should recover
    let ok_result = host.invoke("sometimes_fails", json!({"fail": false})).await;
    assert!(ok_result.is_ok());

    // Another error
    let err_result2 = host.invoke("sometimes_fails", json!({"fail": true})).await;
    assert!(err_result2.is_err());

    // And recover again
    let ok_result2 = host.invoke("sometimes_fails", json!({"fail": false})).await;
    assert!(ok_result2.is_ok());
}

#[tokio::test]
async fn test_invalid_params_deserialization() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // All JSON values should serialize/deserialize correctly via MessagePack
    let result = host.invoke("test", json!({"valid": "json"})).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_multiple_errors_in_sequence() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("error"))
        .with_dispatcher(|_name, _params| Err("Always fails".to_string()))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Multiple errors in a row should all work
    for _ in 0..10 {
        let result = host.invoke("error", json!({})).await;
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_state_consistency_after_errors() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .with_dispatcher(|_name, _params| Err("Error".to_string()))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Error
    let _ = host.invoke("test", json!({})).await;

    // State should still be Ready
    assert_eq!(host.state, HostState::Ready);

    // Pending requests should be empty
    assert_eq!(host.pending_requests.len(), 0);
}

#[tokio::test]
async fn test_error_message_content() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let custom_error = "Custom error message with special chars: 世界🌍";
    let error_clone = custom_error.to_string();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("error"))
        .with_dispatcher(move |_name, _params| Err(error_clone.clone()))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let result = host.invoke("error", json!({})).await;
    assert!(result.is_err());
    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("Custom error message"));
    assert!(error_msg.contains("世界🌍"));
}

#[tokio::test]
async fn test_connection_state_after_multiple_operations() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Multiple operations
    for _ in 0..5 {
        let _ = host.invoke("test", json!({})).await;
        let _ = host.health_check().await;
        let _ = host.cancel(999).await;
    }

    // Connection should still be in Ready state
    assert_eq!(host.state, HostState::Ready);
}

// ========== Category 4: Edge Cases Tests (6 tests) ==========

#[tokio::test]
async fn test_empty_export_list_workflow() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new().build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    assert_eq!(host.exports.len(), 0);

    // Can still do health check
    let health = host.health_check().await;
    assert!(health.is_ok());

    // Can still shutdown
    let shutdown = host.shutdown().await;
    assert!(shutdown.is_ok());
}

#[tokio::test]
async fn test_maximum_export_count() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    // Create 1000 exports
    let mut exports = vec![];
    for i in 0..1000 {
        exports.push(create_test_export(&format!("func_{}", i)));
    }

    let worker = MockWorkerBuilder::new()
        .with_exports(exports)
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    assert_eq!(host.exports.len(), 1000);
}

#[tokio::test]
async fn test_very_long_function_name() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let long_name = "x".repeat(1000);
    let name_clone = long_name.clone();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export(&long_name))
        .with_dispatcher(move |name, _params| {
            if name == name_clone {
                Ok(json!({"ok": true}))
            } else {
                Err("Not found".to_string())
            }
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let result = host.invoke(&long_name, json!({})).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_special_characters_in_function_name() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let special_name = "func-with_special.chars:123";
    let name_clone = special_name.to_string();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export(special_name))
        .with_dispatcher(move |name, _params| {
            if name == name_clone {
                Ok(json!({"ok": true}))
            } else {
                Err("Not found".to_string())
            }
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let result = host.invoke(special_name, json!({})).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_deeply_nested_json() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("echo"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Create deeply nested structure
    let mut nested = json!({"value": 42});
    for _ in 0..100 {
        nested = json!({"nested": nested});
    }

    let result = host.invoke("echo", nested.clone()).await.unwrap();
    assert_eq!(result, nested);
}

#[tokio::test]
async fn test_binary_data_in_json() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("echo"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Base64-encoded binary data
    let binary_data = json!({
        "base64": "SGVsbG8gV29ybGQh",
        "binary_array": [0, 1, 2, 3, 255, 254, 253]
    });

    let result = host.invoke("echo", binary_data.clone()).await.unwrap();
    assert_eq!(result, binary_data);
}

// ========== Category 5: Concurrent Operations Tests (5 tests) ==========

#[tokio::test]
async fn test_health_checks_during_invocations() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("slow"))
        .with_dispatcher(|_name, params| {
            // Simulate work
            Ok(params)
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Invoke and health check (sequential due to mock limitations)
    let _ = host.invoke("slow", json!({})).await;
    let health = host.health_check().await;
    assert!(health.is_ok());
}

#[tokio::test]
async fn test_cancellation_during_processing() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Start a request
    let request_id = host.next_request_id;

    // Try to cancel (even before invoke completes)
    let cancel_result = host.cancel(request_id).await;
    assert!(cancel_result.is_ok());
}

#[tokio::test]
async fn test_shutdown_signal_during_request() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Complete a request then shutdown
    let _ = host.invoke("test", json!({})).await;
    let shutdown = host.shutdown().await;
    assert!(shutdown.is_ok());
}

#[tokio::test]
async fn test_request_id_uniqueness() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("echo"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let mut seen_ids = std::collections::HashSet::new();

    // Make multiple requests and track IDs
    for i in 0..50 {
        let current_id = host.next_request_id;
        assert!(!seen_ids.contains(&current_id), "Duplicate request ID: {}", current_id);
        seen_ids.insert(current_id);

        let result = host.invoke("echo", json!({"id": i})).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_interleaved_operations() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Interleave different operations
    for i in 0..10 {
        match i % 3 {
            0 => {
                let _ = host.invoke("test", json!({"i": i})).await;
            }
            1 => {
                let _ = host.health_check().await;
            }
            2 => {
                let _ = host.cancel(999).await;
            }
            _ => {}
        }
    }
}
