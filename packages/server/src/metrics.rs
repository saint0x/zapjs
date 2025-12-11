//! Prometheus metrics for ZapJS observability
//!
//! Provides:
//! - HTTP request counters, histograms, gauges
//! - IPC handler metrics
//! - Thread-safe global metrics registry

use lazy_static::lazy_static;
use prometheus::{
    CounterVec, Encoder, Gauge, HistogramOpts, HistogramVec, Opts, Registry,
    TextEncoder,
};
use std::sync::Once;

static INIT: Once = Once::new();

lazy_static! {
    /// Global Prometheus metrics registry
    pub static ref REGISTRY: Registry = Registry::new();

    // ========================================================================
    // HTTP Metrics
    // ========================================================================

    /// Total number of HTTP requests
    pub static ref HTTP_REQUESTS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("zap_http_requests_total", "Total number of HTTP requests"),
        &["method", "path", "status"]
    ).expect("metric can be created");

    /// HTTP request duration in seconds
    pub static ref HTTP_REQUEST_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "zap_http_request_duration_seconds",
            "HTTP request duration in seconds"
        ).buckets(vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0
        ]),
        &["method", "path"]
    ).expect("metric can be created");

    /// Number of HTTP requests currently being processed
    pub static ref HTTP_REQUESTS_IN_FLIGHT: Gauge = Gauge::new(
        "zap_http_requests_in_flight",
        "Number of HTTP requests currently being processed"
    ).expect("metric can be created");

    // ========================================================================
    // IPC Metrics
    // ========================================================================

    /// IPC handler invocation duration in seconds
    pub static ref IPC_INVOKE_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "zap_ipc_invoke_duration_seconds",
            "IPC handler invocation duration in seconds"
        ).buckets(vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0
        ]),
        &["handler_id"]
    ).expect("metric can be created");

    /// Total number of IPC handler errors
    pub static ref IPC_HANDLER_ERRORS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("zap_ipc_handler_errors_total", "Total number of IPC handler errors"),
        &["handler_id", "error_code"]
    ).expect("metric can be created");

    /// Total number of IPC handler invocations
    pub static ref IPC_INVOCATIONS_TOTAL: CounterVec = CounterVec::new(
        Opts::new("zap_ipc_invocations_total", "Total number of IPC handler invocations"),
        &["handler_id"]
    ).expect("metric can be created");

    // ========================================================================
    // Server Info Metrics
    // ========================================================================

    /// Server info gauge (always 1, labels contain version info)
    pub static ref SERVER_INFO: CounterVec = CounterVec::new(
        Opts::new("zap_server_info", "Server information"),
        &["version"]
    ).expect("metric can be created");

    /// Server start time (unix timestamp)
    pub static ref SERVER_START_TIME: Gauge = Gauge::new(
        "zap_server_start_time_seconds",
        "Unix timestamp when the server started"
    ).expect("metric can be created");
}

/// Initialize and register all metrics with the global registry
pub fn init_metrics() {
    INIT.call_once(|| {
        // HTTP metrics
        REGISTRY
            .register(Box::new(HTTP_REQUESTS_TOTAL.clone()))
            .expect("HTTP_REQUESTS_TOTAL can be registered");
        REGISTRY
            .register(Box::new(HTTP_REQUEST_DURATION_SECONDS.clone()))
            .expect("HTTP_REQUEST_DURATION_SECONDS can be registered");
        REGISTRY
            .register(Box::new(HTTP_REQUESTS_IN_FLIGHT.clone()))
            .expect("HTTP_REQUESTS_IN_FLIGHT can be registered");

        // IPC metrics
        REGISTRY
            .register(Box::new(IPC_INVOKE_DURATION_SECONDS.clone()))
            .expect("IPC_INVOKE_DURATION_SECONDS can be registered");
        REGISTRY
            .register(Box::new(IPC_HANDLER_ERRORS_TOTAL.clone()))
            .expect("IPC_HANDLER_ERRORS_TOTAL can be registered");
        REGISTRY
            .register(Box::new(IPC_INVOCATIONS_TOTAL.clone()))
            .expect("IPC_INVOCATIONS_TOTAL can be registered");

        // Server info
        REGISTRY
            .register(Box::new(SERVER_INFO.clone()))
            .expect("SERVER_INFO can be registered");
        REGISTRY
            .register(Box::new(SERVER_START_TIME.clone()))
            .expect("SERVER_START_TIME can be registered");

        // Set server start time
        SERVER_START_TIME.set(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        );

        // Set server info (version from Cargo.toml)
        SERVER_INFO
            .with_label_values(&[env!("CARGO_PKG_VERSION")])
            .inc();

        tracing::debug!("Prometheus metrics initialized");
    });
}

/// Encode metrics in Prometheus text format
pub fn encode_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();

    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        tracing::error!("Failed to encode metrics: {}", e);
        return format!("# Error encoding metrics: {}\n", e);
    }

    String::from_utf8(buffer).unwrap_or_else(|e| format!("# Error converting metrics to UTF-8: {}\n", e))
}

/// Normalize path for metrics by replacing dynamic segments with placeholders
///
/// This prevents high cardinality in metrics labels.
/// E.g., `/users/123` -> `/users/:id` if route_pattern is provided,
/// otherwise attempts basic normalization.
pub fn normalize_path(path: &str, route_pattern: Option<&str>) -> String {
    // If we have the route pattern, use it directly
    if let Some(pattern) = route_pattern {
        return pattern.to_string();
    }

    // Basic normalization: replace UUIDs and numeric IDs
    let mut result = path.to_string();

    // Replace UUIDs
    let uuid_regex =
        regex_lite::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
            .unwrap();
    result = uuid_regex.replace_all(&result, ":id").to_string();

    // Replace numeric segments using simple pattern (split and reconstruct)
    // We handle /123/ and /123 patterns without look-ahead
    let parts: Vec<&str> = result.split('/').collect();
    let normalized_parts: Vec<String> = parts
        .iter()
        .map(|part| {
            if part.chars().all(|c| c.is_ascii_digit()) && !part.is_empty() {
                ":id".to_string()
            } else {
                part.to_string()
            }
        })
        .collect();
    result = normalized_parts.join("/");

    result
}

/// Record an HTTP request completion
pub fn record_request(method: &str, path: &str, status: u16, duration_secs: f64) {
    let status_str = status.to_string();
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, path, &status_str])
        .inc();
    HTTP_REQUEST_DURATION_SECONDS
        .with_label_values(&[method, path])
        .observe(duration_secs);
}

/// Record an IPC handler invocation
pub fn record_ipc_invoke(handler_id: &str, duration_secs: f64, error_code: Option<&str>) {
    IPC_INVOCATIONS_TOTAL
        .with_label_values(&[handler_id])
        .inc();
    IPC_INVOKE_DURATION_SECONDS
        .with_label_values(&[handler_id])
        .observe(duration_secs);

    if let Some(code) = error_code {
        IPC_HANDLER_ERRORS_TOTAL
            .with_label_values(&[handler_id, code])
            .inc();
    }
}

/// Increment in-flight request counter
pub fn inc_in_flight() {
    HTTP_REQUESTS_IN_FLIGHT.inc();
}

/// Decrement in-flight request counter
pub fn dec_in_flight() {
    HTTP_REQUESTS_IN_FLIGHT.dec();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_with_pattern() {
        assert_eq!(
            normalize_path("/users/123", Some("/users/:id")),
            "/users/:id"
        );
    }

    #[test]
    fn test_normalize_path_numeric() {
        let result = normalize_path("/users/123/posts/456", None);
        assert!(result.contains(":id"));
    }

    #[test]
    fn test_normalize_path_uuid() {
        let result = normalize_path(
            "/users/550e8400-e29b-41d4-a716-446655440000",
            None,
        );
        assert!(result.contains(":id"));
    }

    #[test]
    fn test_encode_metrics() {
        init_metrics();
        let output = encode_metrics();
        assert!(output.contains("zap_"));
    }
}
