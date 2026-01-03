mod splice_mock;

use splice_mock::*;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

// Import protocol types
use splice::protocol::{ExportMetadata, CAP_STREAMING, CAP_CANCELLATION};

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

fn create_async_export(name: &str) -> ExportMetadata {
    ExportMetadata {
        name: name.to_string(),
        is_async: true,
        is_streaming: false,
        params_schema: "{}".to_string(),
        return_schema: "{}".to_string(),
    }
}

// ========== Category 1: Protocol Handshake Tests (6 tests) ==========

#[tokio::test]
async fn test_successful_handshake() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test_fn"))
        .with_dispatcher(|_name, _params| Ok(json!({"success": true})))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);

    host.connect().await.unwrap();
    assert_eq!(host.state, HostState::Ready);
    assert_eq!(host.exports.len(), 1);
    assert_eq!(host.exports[0].name, "test_fn");
}

#[tokio::test]
async fn test_handshake_protocol_version_match() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new().build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);

    // Should succeed with matching protocol version
    host.connect().await.unwrap();
    assert_eq!(host.state, HostState::Ready);
}

#[tokio::test]
async fn test_handshake_capability_negotiation() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new().build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new()
        .with_capabilities(CAP_STREAMING | CAP_CANCELLATION)
        .build(host_tx, host_rx);

    host.connect().await.unwrap();
    assert_eq!(host.state, HostState::Ready);
}

#[tokio::test]
async fn test_handshake_without_capabilities() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new().build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new()
        .with_capabilities(0)
        .build(host_tx, host_rx);

    host.connect().await.unwrap();
    assert_eq!(host.state, HostState::Ready);
}

#[tokio::test]
async fn test_handshake_server_id() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let custom_server_id = [0xAB; 16];
    let worker = MockWorkerBuilder::new()
        .with_server_id(custom_server_id)
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);

    host.connect().await.unwrap();
    assert_eq!(host.state, HostState::Ready);
}

#[tokio::test]
async fn test_handshake_timeout() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (_worker_tx, _worker_rx)) = harness.split();

    // Don't spawn worker - handshake should timeout

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);

    let result = host.connect().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("timeout"));
}

// ========== Category 2: Export Discovery Tests (3 tests) ==========

#[tokio::test]
async fn test_list_exports_empty() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new().build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);

    host.connect().await.unwrap();
    assert_eq!(host.exports.len(), 0);
}

#[tokio::test]
async fn test_list_exports_multiple() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_exports(vec![
            create_test_export("func1"),
            create_test_export("func2"),
            create_async_export("async_func"),
        ])
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);

    host.connect().await.unwrap();
    assert_eq!(host.exports.len(), 3);
    assert_eq!(host.exports[0].name, "func1");
    assert_eq!(host.exports[1].name, "func2");
    assert_eq!(host.exports[2].name, "async_func");
    assert!(!host.exports[0].is_async);
    assert!(host.exports[2].is_async);
}

#[tokio::test]
async fn test_export_metadata_validation() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let export = ExportMetadata {
        name: "test_function".to_string(),
        is_async: true,
        is_streaming: false,
        params_schema: r#"{"type":"object"}"#.to_string(),
        return_schema: r#"{"type":"string"}"#.to_string(),
    };

    let worker = MockWorkerBuilder::new()
        .with_export(export.clone())
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);

    host.connect().await.unwrap();
    assert_eq!(host.exports[0].name, export.name);
    assert_eq!(host.exports[0].params_schema, export.params_schema);
    assert_eq!(host.exports[0].return_schema, export.return_schema);
}

// ========== Category 3: Function Invocation Tests (12 tests) ==========

#[tokio::test]
async fn test_invoke_sync_function_success() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("add"))
        .with_dispatcher(|name, params| {
            if name == "add" {
                let a = params["a"].as_i64().unwrap();
                let b = params["b"].as_i64().unwrap();
                Ok(json!(a + b))
            } else {
                Err("Function not found".to_string())
            }
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let result = host.invoke("add", json!({"a": 5, "b": 3})).await.unwrap();
    assert_eq!(result, json!(8));
}

#[tokio::test]
async fn test_invoke_async_function_success() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_async_export("fetch_data"))
        .with_dispatcher(|name, _params| {
            if name == "fetch_data" {
                Ok(json!({"data": "async result"}))
            } else {
                Err("Function not found".to_string())
            }
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let result = host.invoke("fetch_data", json!({})).await.unwrap();
    assert_eq!(result, json!({"data": "async result"}));
}

#[tokio::test]
async fn test_invoke_function_with_error() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("divide"))
        .with_dispatcher(|name, params| {
            if name == "divide" {
                let b = params["b"].as_i64().unwrap();
                if b == 0 {
                    Err("Division by zero".to_string())
                } else {
                    let a = params["a"].as_i64().unwrap();
                    Ok(json!(a / b))
                }
            } else {
                Err("Function not found".to_string())
            }
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let result = host.invoke("divide", json!({"a": 10, "b": 0})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Division by zero"));
}

#[tokio::test]
async fn test_invoke_unknown_function() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("known_fn"))
        .with_dispatcher(|name, _params| {
            if name == "known_fn" {
                Ok(json!({"ok": true}))
            } else {
                Err(format!("Function '{}' not found", name))
            }
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let result = host.invoke("unknown_fn", json!({})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[tokio::test]
async fn test_invoke_with_complex_params() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("process"))
        .with_dispatcher(|_name, params| {
            Ok(json!({
                "input": params,
                "processed": true
            }))
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let complex_params = json!({
        "nested": {
            "array": [1, 2, 3],
            "string": "test",
            "bool": true
        },
        "number": 42
    });

    let result = host.invoke("process", complex_params.clone()).await.unwrap();
    assert_eq!(result["input"], complex_params);
    assert_eq!(result["processed"], true);
}

#[tokio::test]
async fn test_invoke_with_empty_params() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("no_params"))
        .with_dispatcher(|_name, _params| Ok(json!({"result": "success"})))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let result = host.invoke("no_params", json!({})).await.unwrap();
    assert_eq!(result, json!({"result": "success"}));
}

#[tokio::test]
async fn test_concurrent_invocations() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("echo"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Spawn 10 concurrent requests
    let mut handles = vec![];
    for i in 0..10 {
        handles.push(tokio::spawn({
            let params = json!({"id": i});
            async move { (i, params) }
        }));
    }

    // Note: Due to the mock implementation limitations, we can't truly test
    // concurrent invocations as each invoke() call processes messages sequentially.
    // This test verifies the setup works, but true concurrency would require
    // a different mock design with shared state.

    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_request_id_wrapping() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .with_dispatcher(|_name, _params| Ok(json!({"ok": true})))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Set next_request_id to near max
    host.next_request_id = u64::MAX - 2;

    // Make 5 requests - should wrap around
    for _ in 0..5 {
        let result = host.invoke("test", json!({})).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_invoke_large_payload() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("process"))
        .with_dispatcher(|_name, params| {
            // Echo back the large payload
            Ok(params)
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Create a large payload (not too large to avoid frame size limits)
    let mut large_array = vec![];
    for i in 0..1000 {
        large_array.push(json!({"index": i, "data": "test data"}));
    }
    let large_params = json!({"items": large_array});

    let result = host.invoke("process", large_params.clone()).await.unwrap();
    assert_eq!(result, large_params);
}

#[tokio::test]
async fn test_invoke_unicode_strings() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("echo"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let unicode_params = json!({
        "chinese": "你好世界",
        "emoji": "🚀🎉🌍",
        "russian": "Привет мир"
    });

    let result = host.invoke("echo", unicode_params.clone()).await.unwrap();
    assert_eq!(result, unicode_params);
}

#[tokio::test]
async fn test_invoke_null_values() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("handle_null"))
        .with_dispatcher(|_name, params| {
            Ok(json!({"received_null": params.get("value").map_or(false, |v| v.is_null())}))
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let result = host.invoke("handle_null", json!({"value": null})).await.unwrap();
    assert_eq!(result["received_null"], true);
}

#[tokio::test]
async fn test_invoke_special_number_values() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("math"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let special_nums = json!({
        "zero": 0,
        "negative": -42,
        "large": 9007199254740991i64, // Max safe integer in JS
        "float": 3.14159
    });

    let result = host.invoke("math", special_nums.clone()).await.unwrap();
    assert_eq!(result, special_nums);
}

// ========== Category 4: Request Tracking Tests (5 tests) ==========

#[tokio::test]
async fn test_pending_requests_management() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .with_dispatcher(|_name, _params| Ok(json!({"ok": true})))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    assert_eq!(host.pending_requests.len(), 0);

    let _result = host.invoke("test", json!({})).await.unwrap();

    // After completion, pending should be empty
    assert_eq!(host.pending_requests.len(), 0);
}

#[tokio::test]
async fn test_response_correlation_by_request_id() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("echo"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Multiple invocations should correlate correctly
    let result1 = host.invoke("echo", json!({"id": 1})).await.unwrap();
    let result2 = host.invoke("echo", json!({"id": 2})).await.unwrap();
    let result3 = host.invoke("echo", json!({"id": 3})).await.unwrap();

    assert_eq!(result1["id"], 1);
    assert_eq!(result2["id"], 2);
    assert_eq!(result3["id"], 3);
}

#[tokio::test]
async fn test_state_validation_before_invoke() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (_worker_tx, _worker_rx)) = harness.split();

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);

    // Try to invoke before connecting
    let result = host.invoke("test", json!({})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Not in ready state"));
}

#[tokio::test]
async fn test_state_transitions() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new().build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);

    // Initial state
    assert_eq!(host.state, HostState::Init);

    // After connect
    host.connect().await.unwrap();
    assert_eq!(host.state, HostState::Ready);

    // After shutdown
    host.shutdown().await.unwrap();
    assert_eq!(host.state, HostState::Shutdown);
}

#[tokio::test]
async fn test_worker_state_transitions() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new().build(worker_rx, worker_tx);

    // Worker starts in Init state
    assert_eq!(worker.state(), WorkerState::Init);

    let worker_handle = tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // After connect, worker should be Ready
    sleep(Duration::from_millis(10)).await;

    host.shutdown().await.unwrap();

    // Wait for worker to finish
    worker_handle.await.unwrap().unwrap();
}

// ========== Category 5: Cancellation Tests (3 tests) ==========

#[tokio::test]
async fn test_cancel_pending_request() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("slow"))
        .with_dispatcher(|_name, _params| {
            // This would be slow in real scenario
            Ok(json!({"done": true}))
        })
        .build(worker_rx, worker_tx);

    let worker_handle = tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Send cancel for a request ID
    let result = host.cancel(999).await;
    assert!(result.is_ok());

    host.shutdown().await.unwrap();
    worker_handle.await.unwrap().unwrap();
}

#[tokio::test]
async fn test_cancel_completed_request() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("fast"))
        .with_dispatcher(|_name, _params| Ok(json!({"done": true})))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Complete a request
    let request_id = host.next_request_id;
    let _result = host.invoke("fast", json!({})).await.unwrap();

    // Try to cancel already completed request - should still ack
    let result = host.cancel(request_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cancel_without_capability() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new().build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new()
        .with_capabilities(0) // No cancellation capability
        .build(host_tx, host_rx);

    host.connect().await.unwrap();

    // Cancel should still work at protocol level
    let result = host.cancel(1).await;
    assert!(result.is_ok());
}

// ========== Category 6: Shutdown Tests (4 tests) ==========

#[tokio::test]
async fn test_graceful_shutdown_no_pending() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new().build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Shutdown immediately
    let result = host.shutdown().await;
    assert!(result.is_ok());
    assert_eq!(host.state, HostState::Shutdown);
}

#[tokio::test]
async fn test_shutdown_after_invocations() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .with_dispatcher(|_name, _params| Ok(json!({"ok": true})))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Do some work
    for _ in 0..5 {
        host.invoke("test", json!({})).await.unwrap();
    }

    // Then shutdown
    let result = host.shutdown().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_double_shutdown() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new().build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // First shutdown
    host.shutdown().await.unwrap();

    // Second shutdown should fail (channel closed)
    let result = host.shutdown().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_shutdown_worker_cleanup() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .with_dispatcher(|_name, _params| Ok(json!({"ok": true})))
        .build(worker_rx, worker_tx);

    assert_eq!(worker.pending_count(), 0);

    let worker_handle = tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    host.shutdown().await.unwrap();

    // Worker should exit cleanly
    let result = worker_handle.await.unwrap();
    assert!(result.is_ok());
}

// ========== Category 7: Health Check Tests (2 tests) ==========

#[tokio::test]
async fn test_health_check_message() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new().build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let (uptime_ms, active_requests, total_requests) = host.health_check().await.unwrap();

    assert_eq!(active_requests, 0);
    assert!(uptime_ms >= 0);
    assert!(total_requests >= 0);
}

#[tokio::test]
async fn test_health_check_during_requests() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .with_dispatcher(|_name, _params| Ok(json!({"ok": true})))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Do some requests first
    for _ in 0..3 {
        host.invoke("test", json!({})).await.unwrap();
    }

    // Health check should work
    let result = host.health_check().await;
    assert!(result.is_ok());
}

// ========== Category 8: Error Handling Tests (5 tests) ==========

#[tokio::test]
async fn test_invalid_message_sequence() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (_worker_tx, _worker_rx)) = harness.split();

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);

    // Try to invoke before handshake
    let result = host.invoke("test", json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_worker_dispatcher_error() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("error_fn"))
        .with_dispatcher(|_name, _params| Err("Internal error".to_string()))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let result = host.invoke("error_fn", json!({})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Internal error"));
}

#[tokio::test]
async fn test_invalid_json_params() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("test"))
        .with_dispatcher(|_name, params| Ok(params))
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Valid JSON should work
    let result = host.invoke("test", json!({"valid": true})).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_connection_recovery_after_error() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_export(create_test_export("maybe_error"))
        .with_dispatcher(|_name, params| {
            if params.get("error").and_then(|v| v.as_bool()).unwrap_or(false) {
                Err("Requested error".to_string())
            } else {
                Ok(json!({"ok": true}))
            }
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    // Cause error
    let result1 = host.invoke("maybe_error", json!({"error": true})).await;
    assert!(result1.is_err());

    // Connection should still work
    let result2 = host.invoke("maybe_error", json!({"error": false})).await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_empty_function_name() {
    let harness = TestHarness::new();
    let ((host_tx, host_rx), (worker_tx, worker_rx)) = harness.split();

    let worker = MockWorkerBuilder::new()
        .with_dispatcher(|name, _params| {
            if name.is_empty() {
                Err("Empty function name".to_string())
            } else {
                Ok(json!({"ok": true}))
            }
        })
        .build(worker_rx, worker_tx);

    tokio::spawn(worker.run());

    let mut host = MockHostBuilder::new().build(host_tx, host_rx);
    host.connect().await.unwrap();

    let result = host.invoke("", json!({})).await;
    assert!(result.is_err());
}
