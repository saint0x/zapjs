//! Router Performance Benchmarks
//!
//! Validates the documented performance claims:
//! - Static route lookup: ~9ns (target < 15ns)
//! - Dynamic route lookup: ~80ns (target < 120ns)

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use zap_core::{Router, Method};

/// Benchmark static route lookups with various route counts
///
/// Tests router performance as it scales from 10 to 10,000 routes.
/// Expected: O(log n) lookup time, ~9ns for typical applications
fn bench_router_static_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("router_static_lookup");

    for route_count in [10, 100, 1000, 10_000].iter() {
        let mut router = Router::new();

        // Insert static routes
        for i in 0..*route_count {
            router
                .insert(Method::GET, &format!("/route{}", i), i)
                .expect("Failed to insert route");
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_routes", route_count)),
            route_count,
            |b, _| {
                b.iter(|| {
                    // Lookup first route (best case)
                    router.at(black_box(Method::GET), black_box("/route0"))
                });
            },
        );
    }
    group.finish();
}

/// Benchmark static route lookup at different positions
///
/// Tests if lookup performance varies by route position in the tree
fn bench_router_static_position(c: &mut Criterion) {
    let mut router = Router::new();

    // Insert 1000 routes
    for i in 0..1000 {
        router
            .insert(Method::GET, &format!("/route{}", i), i)
            .expect("Failed to insert route");
    }

    let mut group = c.benchmark_group("router_static_position");

    // First route (best case)
    group.bench_function("first", |b| {
        b.iter(|| router.at(black_box(Method::GET), black_box("/route0")))
    });

    // Middle route
    group.bench_function("middle", |b| {
        b.iter(|| router.at(black_box(Method::GET), black_box("/route500")))
    });

    // Last route (worst case)
    group.bench_function("last", |b| {
        b.iter(|| router.at(black_box(Method::GET), black_box("/route999")))
    });

    group.finish();
}

/// Benchmark dynamic route lookups with parameters
///
/// Tests parameter extraction performance
/// Expected: ~80ns for single param, ~100-120ns for multiple params
fn bench_router_dynamic_lookup(c: &mut Criterion) {
    let mut router = Router::new();
    router.insert(Method::GET, "/users/:id", "get_user").unwrap();
    router.insert(Method::GET, "/users/:id/posts/:post_id", "get_post").unwrap();
    router.insert(Method::GET, "/api/:version/users/:id/posts/:post_id", "get_post_v").unwrap();

    let mut group = c.benchmark_group("router_dynamic_lookup");

    group.bench_function("single_param", |b| {
        b.iter(|| {
            router.at(black_box(Method::GET), black_box("/users/12345"))
        })
    });

    group.bench_function("two_params", |b| {
        b.iter(|| {
            router.at(black_box(Method::GET), black_box("/users/123/posts/456"))
        })
    });

    group.bench_function("three_params", |b| {
        b.iter(|| {
            router.at(black_box(Method::GET), black_box("/api/v2/users/789/posts/101"))
        })
    });

    group.finish();
}

/// Benchmark wildcard route matching
///
/// Tests performance of catch-all routes
fn bench_router_wildcard(c: &mut Criterion) {
    let mut router = Router::new();
    router.insert(Method::GET, "/files/*filepath", "serve_file").unwrap();
    router.insert(Method::GET, "/assets/**catchall", "serve_asset").unwrap();

    let mut group = c.benchmark_group("router_wildcard");

    // Single wildcard
    group.bench_function("single_wildcard_short", |b| {
        b.iter(|| {
            router.at(
                black_box(Method::GET),
                black_box("/files/test.txt")
            )
        })
    });

    group.bench_function("single_wildcard_long", |b| {
        b.iter(|| {
            router.at(
                black_box(Method::GET),
                black_box("/files/docs/api/reference/v2/spec.md")
            )
        })
    });

    // Double wildcard (catch-all)
    group.bench_function("catch_all", |b| {
        b.iter(|| {
            router.at(
                black_box(Method::GET),
                black_box("/assets/css/components/button/primary.css")
            )
        })
    });

    group.finish();
}

/// Benchmark route not found (404) scenarios
///
/// Tests performance when no route matches
fn bench_router_not_found(c: &mut Criterion) {
    let mut router = Router::new();
    router.insert(Method::GET, "/users/:id", "handler").unwrap();
    router.insert(Method::POST, "/users", "handler").unwrap();
    router.insert(Method::GET, "/posts/:id", "handler").unwrap();

    let mut group = c.benchmark_group("router_not_found");

    // Completely different path
    group.bench_function("different_path", |b| {
        b.iter(|| {
            router.at(black_box(Method::GET), black_box("/nonexistent/path"))
        })
    });

    // Similar but wrong method
    group.bench_function("wrong_method", |b| {
        b.iter(|| {
            router.at(black_box(Method::DELETE), black_box("/users/123"))
        })
    });

    // Almost matching path
    group.bench_function("almost_matching", |b| {
        b.iter(|| {
            router.at(black_box(Method::GET), black_box("/users/123/photos"))
        })
    });

    group.finish();
}

/// Benchmark HTTP method lookup performance
///
/// Tests if different HTTP methods have different lookup performance
fn bench_router_http_methods(c: &mut Criterion) {
    let mut router = Router::new();

    for method in [Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH].iter() {
        router.insert(*method, "/api/resource", format!("{:?}", method)).unwrap();
    }

    let mut group = c.benchmark_group("router_http_methods");

    group.bench_function("GET", |b| {
        b.iter(|| router.at(black_box(Method::GET), black_box("/api/resource")))
    });

    group.bench_function("POST", |b| {
        b.iter(|| router.at(black_box(Method::POST), black_box("/api/resource")))
    });

    group.bench_function("PUT", |b| {
        b.iter(|| router.at(black_box(Method::PUT), black_box("/api/resource")))
    });

    group.bench_function("DELETE", |b| {
        b.iter(|| router.at(black_box(Method::DELETE), black_box("/api/resource")))
    });

    group.bench_function("PATCH", |b| {
        b.iter(|| router.at(black_box(Method::PATCH), black_box("/api/resource")))
    });

    group.finish();
}

/// Benchmark route insertion performance
///
/// Tests how fast routes can be added to the router
fn bench_router_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("router_insertion");

    group.bench_function("static_route", |b| {
        b.iter_batched(
            || Router::new(),
            |mut router| {
                router.insert(black_box(Method::GET), black_box("/test"), "handler").unwrap();
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("dynamic_route", |b| {
        b.iter_batched(
            || Router::new(),
            |mut router| {
                router.insert(black_box(Method::GET), black_box("/users/:id"), "handler").unwrap();
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("wildcard_route", |b| {
        b.iter_batched(
            || Router::new(),
            |mut router| {
                router.insert(black_box(Method::GET), black_box("/files/*path"), "handler").unwrap();
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// Benchmark complex routing patterns
///
/// Tests realistic application routing patterns
fn bench_router_realistic_patterns(c: &mut Criterion) {
    let mut router = Router::new();

    // Typical REST API routes
    router.insert(Method::GET, "/api/v1/users", "list_users").unwrap();
    router.insert(Method::POST, "/api/v1/users", "create_user").unwrap();
    router.insert(Method::GET, "/api/v1/users/:id", "get_user").unwrap();
    router.insert(Method::PUT, "/api/v1/users/:id", "update_user").unwrap();
    router.insert(Method::DELETE, "/api/v1/users/:id", "delete_user").unwrap();
    router.insert(Method::GET, "/api/v1/users/:id/posts", "list_user_posts").unwrap();
    router.insert(Method::GET, "/api/v1/users/:id/posts/:post_id", "get_post").unwrap();
    router.insert(Method::GET, "/api/v1/posts/:id/comments", "list_comments").unwrap();
    router.insert(Method::POST, "/api/v1/posts/:id/comments", "create_comment").unwrap();
    router.insert(Method::GET, "/health", "health_check").unwrap();
    router.insert(Method::GET, "/metrics", "metrics").unwrap();
    router.insert(Method::GET, "/", "index").unwrap();

    let mut group = c.benchmark_group("router_realistic");

    // Common patterns
    group.bench_function("health_check", |b| {
        b.iter(|| router.at(black_box(Method::GET), black_box("/health")))
    });

    group.bench_function("list_api", |b| {
        b.iter(|| router.at(black_box(Method::GET), black_box("/api/v1/users")))
    });

    group.bench_function("get_resource", |b| {
        b.iter(|| router.at(black_box(Method::GET), black_box("/api/v1/users/12345")))
    });

    group.bench_function("nested_resource", |b| {
        b.iter(|| router.at(black_box(Method::GET), black_box("/api/v1/users/123/posts/456")))
    });

    group.finish();
}

criterion_group!(
    router_benches,
    bench_router_static_lookup,
    bench_router_static_position,
    bench_router_dynamic_lookup,
    bench_router_wildcard,
    bench_router_not_found,
    bench_router_http_methods,
    bench_router_insertion,
    bench_router_realistic_patterns
);
criterion_main!(router_benches);
