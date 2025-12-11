//! Static file serving functionality for ZapServer
//!
//! Provides high-performance static file serving with:
//! - ETag generation (weak or strong)
//! - Last-Modified headers
//! - Conditional request handling (304 Not Modified)
//! - Cache-Control configuration
//! - Content-Type detection
//! - Directory traversal protection

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use zap_core::{Response, StatusCode};
use crate::error::ZapError;
use crate::response::ZapResponse;

/// ETag generation strategy
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ETagStrategy {
    /// Weak ETag from mtime + size (fast, no hashing)
    /// Format: W/"size-mtime_hex"
    #[default]
    Weak,
    /// Strong ETag using SHA256 hash (slower but precise)
    /// Format: "sha256_hex"
    Strong,
    /// Disable ETag generation
    None,
}

/// Static file handler configuration
#[derive(Debug, Clone)]
pub struct StaticHandler {
    /// URL prefix (e.g., "/assets")
    pub prefix: String,
    /// Local directory path
    pub directory: PathBuf,
    /// Options for static serving
    pub options: StaticOptions,
}

/// Static file serving options
#[derive(Debug, Clone)]
pub struct StaticOptions {
    /// Enable directory listing
    pub directory_listing: bool,
    /// Set Cache-Control header
    pub cache_control: Option<String>,
    /// Custom headers
    pub headers: HashMap<String, String>,
    /// Enable compression
    pub compress: bool,
    /// ETag generation strategy (default: Weak)
    pub etag_strategy: ETagStrategy,
    /// Enable Last-Modified header (default: true)
    pub enable_last_modified: bool,
}

impl Default for StaticOptions {
    fn default() -> Self {
        Self {
            directory_listing: false,
            cache_control: Some("public, max-age=3600".to_string()),
            headers: HashMap::new(),
            compress: true,
            etag_strategy: ETagStrategy::default(),
            enable_last_modified: true,
        }
    }
}

/// File metadata for caching headers
#[derive(Debug, Clone)]
struct FileMetadata {
    size: u64,
    modified: SystemTime,
}

impl StaticHandler {
    /// Create a new static handler
    pub fn new<P: Into<PathBuf>>(prefix: &str, directory: P) -> Self {
        Self {
            prefix: prefix.to_string(),
            directory: directory.into(),
            options: StaticOptions::default(),
        }
    }

    /// Create a new static handler with options
    pub fn new_with_options<P: Into<PathBuf>>(
        prefix: &str,
        directory: P,
        options: StaticOptions,
    ) -> Self {
        Self {
            prefix: prefix.to_string(),
            directory: directory.into(),
            options,
        }
    }

    /// Handle a static file request with conditional request support
    pub async fn handle(&self, path: &str) -> Result<Option<ZapResponse>, ZapError> {
        self.handle_with_headers(path, &HashMap::new()).await
    }

    /// Handle a static file request with request headers for conditional handling
    pub async fn handle_with_headers(
        &self,
        path: &str,
        request_headers: &HashMap<String, String>,
    ) -> Result<Option<ZapResponse>, ZapError> {
        if !path.starts_with(&self.prefix) {
            return Ok(None);
        }

        let file_path = path.strip_prefix(&self.prefix).unwrap_or("");
        // Handle empty path or root
        let file_path = if file_path.is_empty() || file_path == "/" {
            "index.html"
        } else {
            file_path.trim_start_matches('/')
        };
        let full_path = self.directory.join(file_path);

        // Security check: ensure path doesn't escape the directory
        let canonical_dir = self.directory.canonicalize().unwrap_or_else(|_| self.directory.clone());
        let canonical_path = full_path.canonicalize();

        if let Ok(canonical) = &canonical_path {
            if !canonical.starts_with(&canonical_dir) {
                return Ok(Some(ZapResponse::Custom(Response::forbidden("Access denied"))));
            }
        }

        // Get file metadata
        let metadata = match tokio::fs::metadata(&full_path).await {
            Ok(m) if m.is_file() => m,
            _ => return Ok(None),
        };

        let file_meta = FileMetadata {
            size: metadata.len(),
            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        };

        // Generate ETag if enabled
        let etag = self.generate_etag(&file_meta, &full_path).await;

        // Generate Last-Modified header value
        let last_modified = if self.options.enable_last_modified {
            Some(format_http_date(file_meta.modified))
        } else {
            None
        };

        // Check conditional request headers
        if let Some(ref etag_value) = etag {
            // Check If-None-Match
            if let Some(if_none_match) = request_headers.get("if-none-match")
                .or_else(|| request_headers.get("If-None-Match"))
            {
                if etags_match(if_none_match, etag_value) {
                    return Ok(Some(self.not_modified_response(&etag, &last_modified)));
                }
            }
        }

        // Check If-Modified-Since
        if let Some(ref last_mod) = last_modified {
            if let Some(if_modified_since) = request_headers.get("if-modified-since")
                .or_else(|| request_headers.get("If-Modified-Since"))
            {
                if let Some(since_time) = parse_http_date(if_modified_since) {
                    // File not modified since the specified time
                    if file_meta.modified <= since_time {
                        return Ok(Some(self.not_modified_response(&etag, &Some(last_mod.clone()))));
                    }
                }
            }
        }

        // Read file and serve
        match tokio::fs::read(&full_path).await {
            Ok(contents) => {
                let content_type = mime_guess::from_path(&full_path)
                    .first_or_octet_stream()
                    .to_string();

                let mut response = Response::new()
                    .status(StatusCode::OK)
                    .content_type(content_type)
                    .body(contents);

                // Add cache control if specified
                if let Some(cache_control) = &self.options.cache_control {
                    response = response.cache_control(cache_control);
                }

                // Add ETag header
                if let Some(etag_value) = etag {
                    response = response.header("ETag", etag_value);
                }

                // Add Last-Modified header
                if let Some(last_mod) = last_modified {
                    response = response.header("Last-Modified", last_mod);
                }

                // Add custom headers
                for (key, value) in &self.options.headers {
                    response = response.header(key, value);
                }

                Ok(Some(ZapResponse::Custom(response)))
            }
            Err(_) => Ok(Some(ZapResponse::Custom(
                Response::internal_server_error("Failed to read file"),
            ))),
        }
    }

    /// Generate ETag based on configured strategy
    async fn generate_etag(&self, meta: &FileMetadata, path: &PathBuf) -> Option<String> {
        match self.options.etag_strategy {
            ETagStrategy::Weak => {
                // Weak ETag from size + mtime
                let mtime_secs = meta
                    .modified
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                Some(format!("W/\"{:x}-{:x}\"", meta.size, mtime_secs))
            }
            ETagStrategy::Strong => {
                // Strong ETag using SHA256 hash of content
                match tokio::fs::read(path).await {
                    Ok(contents) => {
                        use sha2::{Digest, Sha256};
                        let mut hasher = Sha256::new();
                        hasher.update(&contents);
                        let hash = hasher.finalize();
                        // Use first 16 bytes (32 hex chars) for reasonable length
                        Some(format!("\"{}\"", hex::encode(&hash[..16])))
                    }
                    Err(_) => None,
                }
            }
            ETagStrategy::None => None,
        }
    }

    /// Generate a 304 Not Modified response
    fn not_modified_response(
        &self,
        etag: &Option<String>,
        last_modified: &Option<String>,
    ) -> ZapResponse {
        let mut response = Response::new().status(StatusCode::NOT_MODIFIED);

        // Add cache control if specified
        if let Some(cache_control) = &self.options.cache_control {
            response = response.cache_control(cache_control);
        }

        // Add ETag header
        if let Some(etag_value) = etag {
            response = response.header("ETag", etag_value);
        }

        // Add Last-Modified header
        if let Some(last_mod) = last_modified {
            response = response.header("Last-Modified", last_mod);
        }

        ZapResponse::Custom(response)
    }
}

/// Handle static file requests from a list of handlers
pub async fn handle_static_files(
    handlers: &[StaticHandler],
    path: &str,
) -> Result<Option<ZapResponse>, ZapError> {
    handle_static_files_with_headers(handlers, path, &HashMap::new()).await
}

/// Handle static file requests with request headers for conditional handling
pub async fn handle_static_files_with_headers(
    handlers: &[StaticHandler],
    path: &str,
    request_headers: &HashMap<String, String>,
) -> Result<Option<ZapResponse>, ZapError> {
    for handler in handlers {
        if let Some(response) = handler.handle_with_headers(path, request_headers).await? {
            return Ok(Some(response));
        }
    }
    Ok(None)
}

// ============================================================================
// HTTP Date Formatting (RFC 7231)
// ============================================================================

/// Format a SystemTime as an HTTP-date (RFC 7231)
/// Example: "Wed, 21 Oct 2015 07:28:00 GMT"
fn format_http_date(time: SystemTime) -> String {
    use std::time::UNIX_EPOCH;

    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs() as i64;

    // Calculate date components
    // Using a simplified algorithm for days since epoch
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;

    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calculate year, month, day using a simplified algorithm
    // This is accurate for dates after 1970
    let (year, month, day, weekday) = days_to_ymd(days_since_epoch);

    let weekday_name = match weekday {
        0 => "Thu", // Jan 1, 1970 was a Thursday
        1 => "Fri",
        2 => "Sat",
        3 => "Sun",
        4 => "Mon",
        5 => "Tue",
        6 => "Wed",
        _ => "???",
    };

    let month_name = match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    };

    format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        weekday_name, day, month_name, year, hours, minutes, seconds
    )
}

/// Convert days since epoch to year, month, day, weekday
fn days_to_ymd(days: i64) -> (i32, u32, u32, u32) {
    // Weekday: Thursday = 0 for Jan 1, 1970
    let weekday = ((days % 7) + 7) % 7;

    // Algorithm adapted from Howard Hinnant's date algorithms
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    (year as i32, m, d, weekday as u32)
}

/// Parse an HTTP-date (RFC 7231) to SystemTime
/// Supports: "Wed, 21 Oct 2015 07:28:00 GMT"
fn parse_http_date(date_str: &str) -> Option<SystemTime> {
    use std::time::{Duration, UNIX_EPOCH};

    // Parse format: "Wed, 21 Oct 2015 07:28:00 GMT"
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }

    // Parse day (skip weekday)
    let day: u32 = parts[1].trim_end_matches(',').parse().ok()?;

    // Parse month
    let month: u32 = match parts[2].to_lowercase().as_str() {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        _ => return None,
    };

    // Parse year
    let year: i32 = parts[3].parse().ok()?;

    // Parse time
    let time_parts: Vec<&str> = parts[4].split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hours: u32 = time_parts[0].parse().ok()?;
    let minutes: u32 = time_parts[1].parse().ok()?;
    let seconds: u32 = time_parts[2].parse().ok()?;

    // Convert to seconds since epoch
    let days = ymd_to_days(year, month, day)?;
    let total_secs = days as u64 * 86400 + hours as u64 * 3600 + minutes as u64 * 60 + seconds as u64;

    Some(UNIX_EPOCH + Duration::from_secs(total_secs))
}

/// Convert year, month, day to days since epoch
fn ymd_to_days(year: i32, month: u32, day: u32) -> Option<i64> {
    // Algorithm adapted from Howard Hinnant's date algorithms
    let y = if month <= 2 { year - 1 } else { year } as i64;
    let m = if month <= 2 { month + 12 } else { month } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m - 3) + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days)
}

// ============================================================================
// ETag Matching
// ============================================================================

/// Check if an If-None-Match header value matches an ETag
/// Handles multiple ETags separated by commas and weak/strong comparison
fn etags_match(if_none_match: &str, etag: &str) -> bool {
    // Handle wildcard
    if if_none_match.trim() == "*" {
        return true;
    }

    // Normalize for weak comparison
    let normalize = |s: &str| -> String {
        let s = s.trim();
        // Strip W/ prefix for weak ETags
        let s = s.strip_prefix("W/").unwrap_or(s);
        // Remove surrounding quotes
        s.trim_matches('"').to_string()
    };

    let etag_normalized = normalize(etag);

    // Check each ETag in the If-None-Match header
    for candidate in if_none_match.split(',') {
        if normalize(candidate) == etag_normalized {
            return true;
        }
    }

    false
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etag_strategy_default() {
        assert_eq!(ETagStrategy::default(), ETagStrategy::Weak);
    }

    #[test]
    fn test_static_options_default() {
        let opts = StaticOptions::default();
        assert!(!opts.directory_listing);
        assert!(opts.compress);
        assert!(opts.enable_last_modified);
        assert_eq!(opts.etag_strategy, ETagStrategy::Weak);
        assert_eq!(opts.cache_control, Some("public, max-age=3600".to_string()));
    }

    #[test]
    fn test_etags_match() {
        // Exact match
        assert!(etags_match("\"abc123\"", "\"abc123\""));

        // Weak ETag comparison
        assert!(etags_match("W/\"abc123\"", "\"abc123\""));
        assert!(etags_match("\"abc123\"", "W/\"abc123\""));
        assert!(etags_match("W/\"abc123\"", "W/\"abc123\""));

        // Multiple ETags
        assert!(etags_match("\"other\", \"abc123\"", "\"abc123\""));
        assert!(etags_match("\"abc123\", \"other\"", "\"abc123\""));

        // Wildcard
        assert!(etags_match("*", "\"anything\""));

        // No match
        assert!(!etags_match("\"different\"", "\"abc123\""));
    }

    #[test]
    fn test_format_http_date() {
        use std::time::{Duration, UNIX_EPOCH};

        // Test a known date: Jan 1, 1970 00:00:00 GMT (epoch)
        let epoch = UNIX_EPOCH;
        let date_str = format_http_date(epoch);
        assert!(date_str.contains("1970"));
        assert!(date_str.contains("Jan"));
        assert!(date_str.contains("GMT"));

        // Test a later date
        let later = UNIX_EPOCH + Duration::from_secs(1445412480); // Oct 21, 2015 07:28:00
        let date_str = format_http_date(later);
        assert!(date_str.contains("2015"));
        assert!(date_str.contains("Oct"));
    }

    #[test]
    fn test_parse_http_date() {
        // Test parsing
        let parsed = parse_http_date("Wed, 21 Oct 2015 07:28:00 GMT");
        assert!(parsed.is_some());

        // Invalid format
        let invalid = parse_http_date("invalid date");
        assert!(invalid.is_none());
    }

    #[test]
    fn test_static_handler_creation() {
        let handler = StaticHandler::new("/assets", "./public");
        assert_eq!(handler.prefix, "/assets");
        assert_eq!(handler.directory, PathBuf::from("./public"));
    }

    #[test]
    fn test_static_handler_with_options() {
        let options = StaticOptions {
            directory_listing: true,
            cache_control: Some("no-cache".to_string()),
            etag_strategy: ETagStrategy::Strong,
            enable_last_modified: false,
            ..Default::default()
        };

        let handler = StaticHandler::new_with_options("/downloads", "./files", options);
        assert!(handler.options.directory_listing);
        assert_eq!(handler.options.etag_strategy, ETagStrategy::Strong);
        assert!(!handler.options.enable_last_modified);
    }
} 