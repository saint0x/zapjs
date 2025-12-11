//! Security Headers Middleware
//!
//! Adds common security headers to all responses by default.
//! These headers help protect against common web vulnerabilities.

use crate::middleware::{Context, Middleware, MiddlewareFuture, MiddlewareResult};
use serde::{Deserialize, Serialize};

/// Security headers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHeadersConfig {
    /// X-Frame-Options value (default: "DENY")
    #[serde(default = "default_frame_options")]
    pub frame_options: Option<String>,

    /// X-Content-Type-Options (default: "nosniff")
    #[serde(default = "default_content_type_options")]
    pub content_type_options: Option<String>,

    /// X-XSS-Protection (default: "1; mode=block")
    #[serde(default = "default_xss_protection")]
    pub xss_protection: Option<String>,

    /// HSTS configuration
    #[serde(default)]
    pub hsts: Option<HstsConfig>,

    /// Content-Security-Policy (no default - must be explicitly configured)
    pub content_security_policy: Option<String>,

    /// Referrer-Policy (default: "strict-origin-when-cross-origin")
    #[serde(default = "default_referrer_policy")]
    pub referrer_policy: Option<String>,

    /// X-Permitted-Cross-Domain-Policies (default: "none")
    #[serde(default = "default_cross_domain_policies")]
    pub cross_domain_policies: Option<String>,

    /// X-Download-Options (default: "noopen")
    #[serde(default = "default_download_options")]
    pub download_options: Option<String>,
}

/// HTTP Strict Transport Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HstsConfig {
    /// max-age in seconds (default: 31536000 = 1 year)
    #[serde(default = "default_hsts_max_age")]
    pub max_age: u64,

    /// Include subdomains (default: true)
    #[serde(default = "default_true")]
    pub include_sub_domains: bool,

    /// Preload flag (default: false)
    #[serde(default)]
    pub preload: bool,
}

// Default value functions
fn default_frame_options() -> Option<String> {
    Some("DENY".to_string())
}
fn default_content_type_options() -> Option<String> {
    Some("nosniff".to_string())
}
fn default_xss_protection() -> Option<String> {
    Some("1; mode=block".to_string())
}
fn default_referrer_policy() -> Option<String> {
    Some("strict-origin-when-cross-origin".to_string())
}
fn default_cross_domain_policies() -> Option<String> {
    Some("none".to_string())
}
fn default_download_options() -> Option<String> {
    Some("noopen".to_string())
}
fn default_hsts_max_age() -> u64 {
    31536000 // 1 year
}
fn default_true() -> bool {
    true
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            frame_options: default_frame_options(),
            content_type_options: default_content_type_options(),
            xss_protection: default_xss_protection(),
            hsts: Some(HstsConfig::default()),
            content_security_policy: None, // Must be explicitly configured
            referrer_policy: default_referrer_policy(),
            cross_domain_policies: default_cross_domain_policies(),
            download_options: default_download_options(),
        }
    }
}

impl Default for HstsConfig {
    fn default() -> Self {
        Self {
            max_age: default_hsts_max_age(),
            include_sub_domains: true,
            preload: false,
        }
    }
}

/// Security Headers Middleware
///
/// Adds security headers to all responses. Configurable via `SecurityHeadersConfig`.
///
/// # Default Headers
/// - `X-Frame-Options: DENY`
/// - `X-Content-Type-Options: nosniff`
/// - `X-XSS-Protection: 1; mode=block`
/// - `Strict-Transport-Security: max-age=31536000; includeSubDomains`
/// - `Referrer-Policy: strict-origin-when-cross-origin`
/// - `X-Permitted-Cross-Domain-Policies: none`
/// - `X-Download-Options: noopen`
pub struct SecurityHeadersMiddleware {
    config: SecurityHeadersConfig,
}

impl SecurityHeadersMiddleware {
    /// Create new security headers middleware with default configuration
    pub fn new() -> Self {
        Self {
            config: SecurityHeadersConfig::default(),
        }
    }

    /// Create security headers middleware with custom configuration
    pub fn with_config(config: SecurityHeadersConfig) -> Self {
        Self { config }
    }

    /// Builder: Set X-Frame-Options
    pub fn frame_options(mut self, value: impl Into<String>) -> Self {
        self.config.frame_options = Some(value.into());
        self
    }

    /// Builder: Disable X-Frame-Options
    pub fn no_frame_options(mut self) -> Self {
        self.config.frame_options = None;
        self
    }

    /// Builder: Set Content-Security-Policy
    pub fn content_security_policy(mut self, value: impl Into<String>) -> Self {
        self.config.content_security_policy = Some(value.into());
        self
    }

    /// Builder: Configure HSTS
    pub fn hsts(mut self, max_age: u64, include_sub_domains: bool, preload: bool) -> Self {
        self.config.hsts = Some(HstsConfig {
            max_age,
            include_sub_domains,
            preload,
        });
        self
    }

    /// Builder: Disable HSTS
    pub fn no_hsts(mut self) -> Self {
        self.config.hsts = None;
        self
    }

    /// Builder: Set Referrer-Policy
    pub fn referrer_policy(mut self, value: impl Into<String>) -> Self {
        self.config.referrer_policy = Some(value.into());
        self
    }
}

impl Default for SecurityHeadersMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for SecurityHeadersMiddleware {
    fn call<'a>(&'a self, ctx: Context<'a>) -> MiddlewareFuture<'a> {
        Box::pin(async move {
            let mut new_ctx = ctx;

            // X-Frame-Options
            if let Some(ref value) = self.config.frame_options {
                new_ctx.response = new_ctx.response.header("X-Frame-Options", value);
            }

            // X-Content-Type-Options
            if let Some(ref value) = self.config.content_type_options {
                new_ctx.response = new_ctx.response.header("X-Content-Type-Options", value);
            }

            // X-XSS-Protection
            if let Some(ref value) = self.config.xss_protection {
                new_ctx.response = new_ctx.response.header("X-XSS-Protection", value);
            }

            // Strict-Transport-Security (HSTS)
            if let Some(ref hsts) = self.config.hsts {
                let mut hsts_value = format!("max-age={}", hsts.max_age);
                if hsts.include_sub_domains {
                    hsts_value.push_str("; includeSubDomains");
                }
                if hsts.preload {
                    hsts_value.push_str("; preload");
                }
                new_ctx.response = new_ctx
                    .response
                    .header("Strict-Transport-Security", &hsts_value);
            }

            // Content-Security-Policy
            if let Some(ref value) = self.config.content_security_policy {
                new_ctx.response = new_ctx.response.header("Content-Security-Policy", value);
            }

            // Referrer-Policy
            if let Some(ref value) = self.config.referrer_policy {
                new_ctx.response = new_ctx.response.header("Referrer-Policy", value);
            }

            // X-Permitted-Cross-Domain-Policies
            if let Some(ref value) = self.config.cross_domain_policies {
                new_ctx.response = new_ctx
                    .response
                    .header("X-Permitted-Cross-Domain-Policies", value);
            }

            // X-Download-Options
            if let Some(ref value) = self.config.download_options {
                new_ctx.response = new_ctx.response.header("X-Download-Options", value);
            }

            Ok((new_ctx, MiddlewareResult::Continue))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HttpParser;

    #[tokio::test]
    async fn test_default_security_headers() {
        let request_bytes = b"GET /test HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let parser = HttpParser::new();
        let parsed = parser.parse_request(request_bytes).unwrap();
        let body = &request_bytes[parsed.body_offset..];

        let ctx = Context::new(&parsed, body);
        let middleware = SecurityHeadersMiddleware::new();

        let (new_ctx, result) = middleware.call(ctx).await.unwrap();
        assert!(matches!(result, MiddlewareResult::Continue));

        // Check default headers were added
        let headers = &new_ctx.response.headers;
        assert!(headers.iter().any(|(k, v)| k == "X-Frame-Options" && v == "DENY"));
        assert!(headers
            .iter()
            .any(|(k, v)| k == "X-Content-Type-Options" && v == "nosniff"));
        assert!(headers
            .iter()
            .any(|(k, v)| k == "Strict-Transport-Security" && v.contains("max-age=")));
        assert!(headers
            .iter()
            .any(|(k, v)| k == "Referrer-Policy" && v == "strict-origin-when-cross-origin"));
    }

    #[tokio::test]
    async fn test_custom_csp() {
        let request_bytes = b"GET /test HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let parser = HttpParser::new();
        let parsed = parser.parse_request(request_bytes).unwrap();
        let body = &request_bytes[parsed.body_offset..];

        let ctx = Context::new(&parsed, body);
        let middleware =
            SecurityHeadersMiddleware::new().content_security_policy("default-src 'self'");

        let (new_ctx, _) = middleware.call(ctx).await.unwrap();

        let headers = &new_ctx.response.headers;
        assert!(headers
            .iter()
            .any(|(k, v)| k == "Content-Security-Policy" && v == "default-src 'self'"));
    }

    #[tokio::test]
    async fn test_disabled_headers() {
        let request_bytes = b"GET /test HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let parser = HttpParser::new();
        let parsed = parser.parse_request(request_bytes).unwrap();
        let body = &request_bytes[parsed.body_offset..];

        let ctx = Context::new(&parsed, body);
        let middleware = SecurityHeadersMiddleware::new().no_frame_options().no_hsts();

        let (new_ctx, _) = middleware.call(ctx).await.unwrap();

        let headers = &new_ctx.response.headers;
        assert!(!headers.iter().any(|(k, _)| k == "X-Frame-Options"));
        assert!(!headers.iter().any(|(k, _)| k == "Strict-Transport-Security"));
    }

    #[test]
    fn test_config_serialization() {
        let config = SecurityHeadersConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: SecurityHeadersConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.frame_options, decoded.frame_options);
        assert_eq!(config.hsts.is_some(), decoded.hsts.is_some());
    }
}
