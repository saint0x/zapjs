//! Rate Limiting Middleware
//!
//! IP-based rate limiting with pluggable storage backends.
//! Supports in-memory storage for single-instance deployments
//! and Redis for distributed deployments.

use crate::middleware::{Context, Middleware, MiddlewareFuture, MiddlewareResult, ResponseBuilder};
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per window (default: 100)
    #[serde(default = "default_max_requests")]
    pub max_requests: u32,

    /// Window duration in seconds (default: 60)
    #[serde(default = "default_window_secs")]
    pub window_secs: u64,

    /// Storage backend type
    #[serde(default)]
    pub storage: RateLimitStorage,

    /// Redis URL (for redis storage)
    pub redis_url: Option<String>,

    /// Paths to skip rate limiting (supports wildcards like "/health*")
    #[serde(default)]
    pub skip_paths: Vec<String>,

    /// Custom error message
    #[serde(default = "default_error_message")]
    pub message: String,
}

/// Storage backend type
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RateLimitStorage {
    #[default]
    Memory,
    Redis,
}

fn default_max_requests() -> u32 {
    100
}
fn default_window_secs() -> u64 {
    60
}
fn default_error_message() -> String {
    "Too Many Requests".to_string()
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: default_max_requests(),
            window_secs: default_window_secs(),
            storage: RateLimitStorage::Memory,
            redis_url: None,
            skip_paths: Vec::new(),
            message: default_error_message(),
        }
    }
}

/// Rate limit storage trait for pluggable backends
#[async_trait]
pub trait RateLimitStore: Send + Sync {
    /// Increment the counter for a key and return (current_count, remaining_ttl_secs)
    async fn increment(&self, key: &str, window_secs: u64) -> Result<(u32, u64), RateLimitError>;

    /// Get current count for a key
    async fn get(&self, key: &str) -> Result<Option<u32>, RateLimitError>;

    /// Reset the counter for a key
    async fn reset(&self, key: &str) -> Result<(), RateLimitError>;
}

/// Rate limiting errors
#[derive(Debug)]
pub enum RateLimitError {
    /// Storage backend error
    StorageError(String),
    /// Connection error (for Redis)
    ConnectionError(String),
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            RateLimitError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
        }
    }
}

impl std::error::Error for RateLimitError {}

/// Entry in the in-memory rate limit store
struct RateLimitEntry {
    count: u32,
    window_start: Instant,
}

/// In-memory rate limit storage (single instance only)
///
/// Uses a sliding window algorithm for rate limiting.
/// Not suitable for distributed deployments - use Redis for that.
pub struct InMemoryStore {
    /// Map of key -> (count, window_start)
    entries: RwLock<HashMap<String, RateLimitEntry>>,
    window_duration: Duration,
}

impl InMemoryStore {
    /// Create new in-memory store with the given window duration
    pub fn new(window_secs: u64) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            window_duration: Duration::from_secs(window_secs),
        }
    }

    /// Clean up expired entries (call periodically to prevent memory bloat)
    pub fn cleanup(&self) {
        let mut entries = self.entries.write();
        let now = Instant::now();
        entries.retain(|_, entry| now.duration_since(entry.window_start) < self.window_duration);
    }
}

#[async_trait]
impl RateLimitStore for InMemoryStore {
    async fn increment(&self, key: &str, window_secs: u64) -> Result<(u32, u64), RateLimitError> {
        let mut entries = self.entries.write();
        let now = Instant::now();
        let window_duration = Duration::from_secs(window_secs);

        let entry = entries.entry(key.to_string()).or_insert(RateLimitEntry {
            count: 0,
            window_start: now,
        });

        // Check if window has expired
        if now.duration_since(entry.window_start) >= window_duration {
            entry.count = 1;
            entry.window_start = now;
            return Ok((1, window_secs));
        }

        // Increment count
        entry.count += 1;
        let elapsed = now.duration_since(entry.window_start).as_secs();
        let remaining = window_secs.saturating_sub(elapsed);

        Ok((entry.count, remaining))
    }

    async fn get(&self, key: &str) -> Result<Option<u32>, RateLimitError> {
        let entries = self.entries.read();
        Ok(entries.get(key).map(|e| e.count))
    }

    async fn reset(&self, key: &str) -> Result<(), RateLimitError> {
        let mut entries = self.entries.write();
        entries.remove(key);
        Ok(())
    }
}

/// Rate Limiting Middleware
///
/// Limits requests based on client IP address.
/// Returns 429 Too Many Requests when limit is exceeded.
pub struct RateLimitMiddleware {
    config: RateLimitConfig,
    store: Arc<dyn RateLimitStore>,
}

impl RateLimitMiddleware {
    /// Create new rate limit middleware with in-memory storage
    pub fn new(config: RateLimitConfig) -> Self {
        let store: Arc<dyn RateLimitStore> =
            Arc::new(InMemoryStore::new(config.window_secs));
        Self { config, store }
    }

    /// Create rate limit middleware with custom storage backend
    pub fn with_store(config: RateLimitConfig, store: Arc<dyn RateLimitStore>) -> Self {
        Self { config, store }
    }

    /// Create with default configuration (100 req/min)
    pub fn default_config() -> Self {
        Self::new(RateLimitConfig::default())
    }

    /// Builder: Set max requests
    pub fn max_requests(mut self, max: u32) -> Self {
        self.config.max_requests = max;
        self
    }

    /// Builder: Set window duration in seconds
    pub fn window_secs(mut self, secs: u64) -> Self {
        self.config.window_secs = secs;
        self
    }

    /// Builder: Add path to skip list
    pub fn skip_path(mut self, path: impl Into<String>) -> Self {
        self.config.skip_paths.push(path.into());
        self
    }

    /// Builder: Set custom error message
    pub fn message(mut self, msg: impl Into<String>) -> Self {
        self.config.message = msg.into();
        self
    }

    /// Extract client IP from request context
    fn extract_client_ip(ctx: &Context) -> String {
        // Check X-Forwarded-For first (for proxied requests)
        if let Some(forwarded) = ctx.headers().get("X-Forwarded-For") {
            if let Some(first_ip) = forwarded.split(',').next() {
                return first_ip.trim().to_string();
            }
        }

        // Check X-Real-IP
        if let Some(real_ip) = ctx.headers().get("X-Real-IP") {
            return real_ip.to_string();
        }

        // Check CF-Connecting-IP (Cloudflare)
        if let Some(cf_ip) = ctx.headers().get("CF-Connecting-IP") {
            return cf_ip.to_string();
        }

        // Default to unknown
        "unknown".to_string()
    }

    /// Check if path should be skipped
    fn should_skip(&self, path: &str) -> bool {
        self.config.skip_paths.iter().any(|p| {
            if p.ends_with('*') {
                path.starts_with(&p[..p.len() - 1])
            } else {
                path == p
            }
        })
    }
}

impl Middleware for RateLimitMiddleware {
    fn call<'a>(&'a self, ctx: Context<'a>) -> MiddlewareFuture<'a> {
        Box::pin(async move {
            // Check if path should be skipped
            if self.should_skip(ctx.path()) {
                return Ok((ctx, MiddlewareResult::Continue));
            }

            let client_ip = Self::extract_client_ip(&ctx);
            let key = format!("{}:{}", ctx.path(), client_ip);

            match self.store.increment(&key, self.config.window_secs).await {
                Ok((count, remaining_secs)) => {
                    let mut new_ctx = ctx;

                    // Add rate limit headers to response
                    new_ctx.response = new_ctx
                        .response
                        .header("X-RateLimit-Limit", &self.config.max_requests.to_string())
                        .header(
                            "X-RateLimit-Remaining",
                            &self.config.max_requests.saturating_sub(count).to_string(),
                        )
                        .header("X-RateLimit-Reset", &remaining_secs.to_string());

                    if count > self.config.max_requests {
                        // Rate limit exceeded - return 429
                        let response = ResponseBuilder::new()
                            .status(429)
                            .header("Retry-After", &remaining_secs.to_string())
                            .header("X-RateLimit-Limit", &self.config.max_requests.to_string())
                            .header("X-RateLimit-Remaining", "0")
                            .header("X-RateLimit-Reset", &remaining_secs.to_string())
                            .header("Content-Type", "application/json")
                            .body(
                                format!(
                                    r#"{{"error":"{}","retry_after":{}}}"#,
                                    self.config.message, remaining_secs
                                )
                                .into_bytes(),
                            )
                            .finish();

                        return Ok((new_ctx, MiddlewareResult::Response(response)));
                    }

                    Ok((new_ctx, MiddlewareResult::Continue))
                }
                Err(e) => {
                    // Log error but don't block request on storage failure
                    eprintln!("Rate limit storage error: {}", e);
                    Ok((ctx, MiddlewareResult::Continue))
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HttpParser;

    #[tokio::test]
    async fn test_in_memory_store() {
        let store = InMemoryStore::new(60);

        // First request
        let (count, _) = store.increment("test-key", 60).await.unwrap();
        assert_eq!(count, 1);

        // Second request
        let (count, _) = store.increment("test-key", 60).await.unwrap();
        assert_eq!(count, 2);

        // Different key
        let (count, _) = store.increment("other-key", 60).await.unwrap();
        assert_eq!(count, 1);

        // Get current count
        let count = store.get("test-key").await.unwrap();
        assert_eq!(count, Some(2));

        // Reset
        store.reset("test-key").await.unwrap();
        let count = store.get("test-key").await.unwrap();
        assert_eq!(count, None);
    }

    #[tokio::test]
    async fn test_rate_limit_middleware() {
        let request_bytes = b"GET /api/test HTTP/1.1\r\nHost: example.com\r\nX-Forwarded-For: 192.168.1.1\r\n\r\n";
        let parser = HttpParser::new();
        let parsed = parser.parse_request(request_bytes).unwrap();
        let body = &request_bytes[parsed.body_offset..];

        let middleware = RateLimitMiddleware::new(RateLimitConfig {
            max_requests: 2,
            window_secs: 60,
            ..Default::default()
        });

        // First request - should pass
        let ctx = Context::new(&parsed, body);
        let (new_ctx, result) = middleware.call(ctx).await.unwrap();
        assert!(matches!(result, MiddlewareResult::Continue));
        assert!(new_ctx
            .response
            .headers
            .iter()
            .any(|(k, v)| k == "X-RateLimit-Remaining" && v == "1"));

        // Second request - should pass
        let ctx = Context::new(&parsed, body);
        let (_, result) = middleware.call(ctx).await.unwrap();
        assert!(matches!(result, MiddlewareResult::Continue));

        // Third request - should be rate limited
        let ctx = Context::new(&parsed, body);
        let (_, result) = middleware.call(ctx).await.unwrap();
        match result {
            MiddlewareResult::Response(response) => {
                assert_eq!(response.status, 429);
                assert!(response
                    .headers
                    .iter()
                    .any(|(k, _)| k == "Retry-After"));
            }
            _ => panic!("Expected rate limit response"),
        }
    }

    #[tokio::test]
    async fn test_skip_paths() {
        let request_bytes = b"GET /health HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let parser = HttpParser::new();
        let parsed = parser.parse_request(request_bytes).unwrap();
        let body = &request_bytes[parsed.body_offset..];

        let middleware = RateLimitMiddleware::new(RateLimitConfig {
            max_requests: 1,
            window_secs: 60,
            skip_paths: vec!["/health".to_string(), "/metrics*".to_string()],
            ..Default::default()
        });

        // Health path should be skipped
        let ctx = Context::new(&parsed, body);
        let (_, result) = middleware.call(ctx).await.unwrap();
        assert!(matches!(result, MiddlewareResult::Continue));

        // Second request should also pass (skipped)
        let ctx = Context::new(&parsed, body);
        let (_, result) = middleware.call(ctx).await.unwrap();
        assert!(matches!(result, MiddlewareResult::Continue));
    }

    #[test]
    fn test_config_serialization() {
        let config = RateLimitConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: RateLimitConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.max_requests, decoded.max_requests);
        assert_eq!(config.window_secs, decoded.window_secs);
        assert_eq!(config.storage, decoded.storage);
    }
}
