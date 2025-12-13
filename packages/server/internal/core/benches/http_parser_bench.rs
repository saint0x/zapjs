//! HTTP Parser Performance Benchmarks
//!
//! Validates the documented performance claims:
//! - Simple GET: ~125ns (target < 200ns)
//! - With headers: ~312ns (target < 400ns)

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use zap_core::HttpParser;

/// Benchmark simple GET request parsing
///
/// Expected: ~125ns for minimal GET request
fn bench_http_simple_get(c: &mut Criterion) {
    let parser = HttpParser::new();
    let request = b"GET /hello HTTP/1.1\r\n\r\n";

    let mut group = c.benchmark_group("http_parser_simple");
    group.throughput(Throughput::Bytes(request.len() as u64));

    group.bench_function("minimal_get", |b| {
        b.iter(|| parser.parse_request(black_box(request)))
    });

    group.finish();
}

/// Benchmark GET request with common headers
///
/// Expected: ~312ns with typical headers
fn bench_http_with_headers(c: &mut Criterion) {
    let parser = HttpParser::new();

    // Typical browser request
    let request = b"GET /api/users HTTP/1.1\r\n\
Host: example.com\r\n\
User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64)\r\n\
Accept: application/json\r\n\
Accept-Encoding: gzip, deflate\r\n\
Connection: keep-alive\r\n\
\r\n";

    let mut group = c.benchmark_group("http_parser_headers");
    group.throughput(Throughput::Bytes(request.len() as u64));

    group.bench_function("typical_headers", |b| {
        b.iter(|| parser.parse_request(black_box(request)))
    });

    group.finish();
}

/// Benchmark POST request with body
///
/// Tests parsing performance with JSON payload
fn bench_http_post_with_body(c: &mut Criterion) {
    let parser = HttpParser::new();

    let request = b"POST /api/users HTTP/1.1\r\n\
Host: api.example.com\r\n\
Content-Type: application/json\r\n\
Content-Length: 46\r\n\
\r\n\
{\"name\":\"John Doe\",\"email\":\"john@example.com\"}";

    let mut group = c.benchmark_group("http_parser_post");
    group.throughput(Throughput::Bytes(request.len() as u64));

    group.bench_function("json_body", |b| {
        b.iter(|| parser.parse_request(black_box(request)))
    });

    group.finish();
}

/// Benchmark request with many headers
///
/// Tests parser performance under header-heavy scenarios
fn bench_http_large_headers(c: &mut Criterion) {
    let parser = HttpParser::new();

    let mut request = String::from("GET / HTTP/1.1\r\n");
    for i in 0..50 {
        request.push_str(&format!("X-Custom-Header-{}: value-{}\r\n", i, i));
    }
    request.push_str("\r\n");

    let request_bytes = request.as_bytes();

    let mut group = c.benchmark_group("http_parser_large_headers");
    group.throughput(Throughput::Bytes(request_bytes.len() as u64));

    group.bench_function("50_headers", |b| {
        b.iter(|| parser.parse_request(black_box(request_bytes)))
    });

    group.finish();
}

/// Benchmark different HTTP methods
///
/// Tests if method parsing affects performance
fn bench_http_methods(c: &mut Criterion) {
    let parser = HttpParser::new();
    let mut group = c.benchmark_group("http_parser_methods");

    let methods = [
        ("GET", b"GET /api HTTP/1.1\r\n\r\n" as &[u8]),
        ("POST", b"POST /api HTTP/1.1\r\n\r\n"),
        ("PUT", b"PUT /api HTTP/1.1\r\n\r\n"),
        ("DELETE", b"DELETE /api HTTP/1.1\r\n\r\n"),
        ("PATCH", b"PATCH /api HTTP/1.1\r\n\r\n"),
        ("OPTIONS", b"OPTIONS /api HTTP/1.1\r\n\r\n"),
        ("HEAD", b"HEAD /api HTTP/1.1\r\n\r\n"),
    ];

    for (method, request) in methods.iter() {
        group.throughput(Throughput::Bytes(request.len() as u64));
        group.bench_function(*method, |b| {
            b.iter(|| parser.parse_request(black_box(*request)))
        });
    }

    group.finish();
}

/// Benchmark different path lengths
///
/// Tests if path length affects parsing performance
fn bench_http_path_lengths(c: &mut Criterion) {
    let parser = HttpParser::new();
    let mut group = c.benchmark_group("http_parser_path_lengths");

    let paths = [
        ("short", b"GET / HTTP/1.1\r\n\r\n" as &[u8]),
        ("medium", b"GET /api/v1/users HTTP/1.1\r\n\r\n"),
        ("long", b"GET /api/v1/users/12345/posts/67890/comments HTTP/1.1\r\n\r\n"),
        ("very_long", b"GET /api/v1/organizations/org123/projects/proj456/repositories/repo789/branches/feature/new-feature/commits/abc123def456 HTTP/1.1\r\n\r\n"),
    ];

    for (name, request) in paths.iter() {
        group.throughput(Throughput::Bytes(request.len() as u64));
        group.bench_function(*name, |b| {
            b.iter(|| parser.parse_request(black_box(*request)))
        });
    }

    group.finish();
}

/// Benchmark query string parsing
///
/// Tests performance with URL parameters
fn bench_http_query_strings(c: &mut Criterion) {
    let parser = HttpParser::new();
    let mut group = c.benchmark_group("http_parser_query_strings");

    // No query string
    let no_query = b"GET /api/users HTTP/1.1\r\n\r\n";
    group.bench_function("no_query", |b| {
        b.iter(|| parser.parse_request(black_box(no_query)))
    });

    // Single param
    let single_param = b"GET /api/users?page=1 HTTP/1.1\r\n\r\n";
    group.bench_function("single_param", |b| {
        b.iter(|| parser.parse_request(black_box(single_param)))
    });

    // Multiple params
    let multi_params = b"GET /api/users?page=1&limit=50&sort=name&order=asc HTTP/1.1\r\n\r\n";
    group.bench_function("multi_params", |b| {
        b.iter(|| parser.parse_request(black_box(multi_params)))
    });

    group.finish();
}

/// Benchmark realistic HTTP requests
///
/// Tests common real-world scenarios
fn bench_http_realistic(c: &mut Criterion) {
    let parser = HttpParser::new();
    let mut group = c.benchmark_group("http_parser_realistic");

    // API GET request
    let api_get = b"GET /api/v1/users/12345 HTTP/1.1\r\n\
Host: api.example.com\r\n\
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9\r\n\
Accept: application/json\r\n\
User-Agent: MyApp/1.0\r\n\
\r\n";

    group.throughput(Throughput::Bytes(api_get.len() as u64));
    group.bench_function("api_get", |b| {
        b.iter(|| parser.parse_request(black_box(api_get)))
    });

    // API POST request
    let api_post = b"POST /api/v1/users HTTP/1.1\r\n\
Host: api.example.com\r\n\
Content-Type: application/json\r\n\
Content-Length: 87\r\n\
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9\r\n\
\r\n\
{\"name\":\"Jane Smith\",\"email\":\"jane@example.com\",\"role\":\"admin\",\"active\":true}";

    group.throughput(Throughput::Bytes(api_post.len() as u64));
    group.bench_function("api_post", |b| {
        b.iter(|| parser.parse_request(black_box(api_post)))
    });

    // Health check
    let health = b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n";
    group.throughput(Throughput::Bytes(health.len() as u64));
    group.bench_function("health_check", |b| {
        b.iter(|| parser.parse_request(black_box(health)))
    });

    group.finish();
}

criterion_group!(
    http_parser_benches,
    bench_http_simple_get,
    bench_http_with_headers,
    bench_http_post_with_body,
    bench_http_large_headers,
    bench_http_methods,
    bench_http_path_lengths,
    bench_http_query_strings,
    bench_http_realistic
);
criterion_main!(http_parser_benches);
