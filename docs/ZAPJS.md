# ZapJS Implementation Plan

> Fullstack web framework: React frontend + Rust backend compiled into a single deployable binary.

**Status:** Core Features Complete | **Engineering Audit Score:** 7,200/10,000 | **Updated:** December 2024

---

## Completed Features (Phases 1-11)

**Core Infrastructure:** Monorepo, CLI (`zap dev/build/serve/new/codegen/routes`), file-based routing, hot reload, production builds

**Type Safety:** Bidirectional Rust↔TypeScript codegen, `#[export]` macro, automatic type bindings, `Result<T, E>` → union types

**Performance:** MessagePack IPC, connection pooling, 9ns static routing, streaming responses, WebSocket support

**Security:** Security headers, rate limiting, strict CORS, path traversal protection

**Observability:** Prometheus metrics, request ID correlation, structured logging

**Reliability:** Circuit breaker, exponential backoff, Kubernetes health probes, IPC retry logic

**Client Features:** React router (useRouter, Link, NavLink), SSG (generateStaticParams), error boundaries

**Caching:** ETag/Last-Modified support, conditional requests (304)

---

## Production Readiness Roadmap

### 🔴 P0 - Critical (Blocking Production Use)

#### 1. Comprehensive Test Suite for Client Package ⚠️ CRITICAL
**Status:** Zero client-side tests currently - framework has 0 test files for TypeScript code
**Why Critical:** Untested IPC deserialization, route matching, and error boundaries are production risks

**Tasks:**
- [ ] Set up test framework (Vitest recommended for speed)
- [ ] Unit tests for IPC message serialization/deserialization
- [ ] Unit tests for route matching algorithm
- [ ] Integration tests for RouterProvider and navigation
- [ ] Tests for error boundary behavior
- [ ] Tests for hooks (useRouter, useParams, useSearchParams)
- [ ] Tests for Link/NavLink components
- [ ] Mock IPC client for handler testing
- [ ] Achieve minimum 80% coverage on client package
- [ ] Add coverage reporting to CI/CD
- [ ] Document testing patterns for users

**Estimated Effort:** 2-3 weeks
**Acceptance Criteria:**
- All critical paths tested
- Coverage >80% on packages/client
- CI fails on coverage regression

---

#### 2. Enable Strict TypeScript Mode ⚠️ CRITICAL
**Status:** `"strict": false` in client tsconfig - disables null checks and implicit any
**Why Critical:** Type safety is undermined without strict mode; framework handles HTTP requests

**Tasks:**
- [ ] Set `"strict": true` in packages/client/tsconfig.json
- [ ] Fix all null/undefined type errors
- [ ] Remove all implicit `any` types
- [ ] Add proper type annotations to function parameters
- [ ] Enable `strictNullChecks` and fix issues
- [ ] Enable `noImplicitAny` and fix issues
- [ ] Enable `strictFunctionTypes` and fix issues
- [ ] Update any loose type definitions
- [ ] Add ESLint rule to enforce strict types
- [ ] Document type safety guidelines

**Estimated Effort:** 1-2 weeks
**Acceptance Criteria:**
- `strict: true` in all tsconfig files
- Zero TypeScript errors with strict mode
- ESLint enforces strict typing

---

#### 3. CSRF Protection ⚠️ CRITICAL
**Status:** No CSRF protection - forms vulnerable to cross-site attacks
**Why Critical:** Security vulnerability for any app with forms/mutations

**Tasks:**
- [ ] Implement CSRF token generation (double-submit cookie pattern)
- [ ] Add CSRF middleware for Rust server
- [ ] Auto-inject CSRF tokens in form components
- [ ] Validate CSRF tokens on POST/PUT/DELETE/PATCH
- [ ] Add `<CsrfToken>` component for forms
- [ ] SameSite cookie configuration (Strict/Lax)
- [ ] CSRF exemption for API routes (optional)
- [ ] Document CSRF setup and best practices
- [ ] Add tests for CSRF validation
- [ ] Enable by default with opt-out

**Estimated Effort:** 1 week
**Acceptance Criteria:**
- All state-changing requests protected
- Documented opt-out mechanism
- Tests verify CSRF validation
- Default configuration is secure

---

### 🟡 P1 - High Priority (Important for Adoption)

#### 4. Comprehensive Documentation
**Status:** 97-line README only - no architectural docs, API reference, or guides
**Why Important:** Framework unusable without extensive documentation

**Tasks:**
- [ ] Create documentation site (Docusaurus/VitePress)
- [ ] Getting Started guide (installation, first app)
- [ ] Architecture overview (Rust/TS bridge, IPC protocol)
- [ ] API reference for all exported functions/types
- [ ] Routing guide (file conventions, dynamic routes, layouts)
- [ ] Data fetching patterns
- [ ] Deployment guide (Docker, systemd, cloud providers)
- [ ] Security best practices (CSRF, XSS, rate limiting)
- [ ] Performance tuning guide
- [ ] Migration guide from Next.js/Express
- [ ] Troubleshooting common issues
- [ ] Contributing guidelines (CONTRIBUTING.md)
- [ ] Example applications (blog, dashboard, real-time app)
- [ ] Video tutorials for key concepts
- [ ] ADRs (Architecture Decision Records)

**Estimated Effort:** 4-6 weeks
**Acceptance Criteria:**
- Documentation site deployed
- All major features documented
- 3+ example applications
- Community can contribute docs

---

#### 5. Performance Benchmarks
**Status:** Claims "10-100x faster than Express" but no published benchmarks
**Why Important:** Unverified performance claims hurt credibility

**Tasks:**
- [ ] Create benchmark suite comparing to:
  - Express.js (baseline)
  - Fastify (fast Node framework)
  - Next.js API routes
  - Bun + Elysia
  - Deno Fresh
- [ ] Benchmark scenarios:
  - Static route lookup (validate "9ns" claim)
  - Dynamic route with params
  - JSON serialization/deserialization
  - IPC roundtrip latency
  - Streaming responses
  - WebSocket message throughput
  - Concurrent request handling
  - Memory usage under load
- [ ] Use `criterion.rs` for Rust benchmarks
- [ ] Use `autocannon` for HTTP benchmarks
- [ ] Publish methodology and raw results
- [ ] Add performance regression tests to CI
- [ ] Create performance dashboard
- [ ] Document hardware specs for benchmarks

**Estimated Effort:** 2-3 weeks
**Acceptance Criteria:**
- Published benchmarks with methodology
- Verified performance claims (or updated claims)
- Automated benchmark runs in CI
- Performance regression detection

---

#### 6. Graceful Shutdown
**Status:** Signal handling exists but incomplete - no drain period for in-flight requests
**Why Important:** Lost requests and WebSocket connections on deployments

**Tasks:**
- [ ] Implement shutdown signal handling (SIGTERM, SIGINT)
- [ ] Configurable drain timeout (default 30s)
- [ ] Stop accepting new HTTP connections
- [ ] Wait for in-flight HTTP requests to complete
- [ ] Gracefully close WebSocket connections
- [ ] Send close frames with reason code
- [ ] Shutdown IPC connections cleanly
- [ ] Log shutdown progress
- [ ] Force shutdown after timeout
- [ ] Document graceful shutdown behavior
- [ ] Add tests for shutdown scenarios
- [ ] Kubernetes preStop hook example

**Estimated Effort:** 1 week
**Acceptance Criteria:**
- Zero dropped requests during normal shutdown
- WebSocket clients notified of shutdown
- Configurable timeout
- Works with systemd/Kubernetes

---

### 🟢 P2 - Nice to Have (Quality of Life)

#### 7. Distributed Tracing (OpenTelemetry)
**Status:** Request IDs exist but not propagated to all logs
**Why Useful:** Hard to debug issues across Rust/TypeScript boundary

**Tasks:**
- [ ] Integrate OpenTelemetry SDK (Rust + TypeScript)
- [ ] Trace context propagation across IPC
- [ ] Span creation for HTTP requests, handlers, DB calls
- [ ] Export to Jaeger/Zipkin/Honeycomb
- [ ] Correlation with existing request IDs
- [ ] Performance overhead measurement
- [ ] Configuration options (sampling rate)
- [ ] Documentation and examples

**Estimated Effort:** 2 weeks

---

#### 8. Optimize Route Matching
**Status:** O(n) linear search through routes on every navigation
**Why Useful:** Performance degrades with large route counts

**Tasks:**
- [ ] Replace linear search with radix tree/trie
- [ ] Cache compiled route patterns
- [ ] Use Map lookup for exact matches
- [ ] Benchmark before/after performance
- [ ] Ensure no breaking changes to API
- [ ] Add tests for route matching edge cases

**Estimated Effort:** 1 week

---

#### 9. HTTP/2 Support
**Status:** HTTP/1.1 only - no multiplexing or header compression
**Why Useful:** Better performance for modern clients

**Tasks:**
- [ ] Enable HTTP/2 in Hyper configuration
- [ ] Test multiplexed requests
- [ ] Header compression verification
- [ ] Server push support (optional)
- [ ] Fallback to HTTP/1.1 for older clients
- [ ] Document HTTP/2 configuration
- [ ] Performance benchmarks vs HTTP/1.1

**Estimated Effort:** 1 week

---

### 📋 Deferred Roadmap

**Phase 9: Edge/WASM Runtime**
- [ ] Compile Rust to WASM
- [ ] Vercel/Cloudflare Workers support
- [ ] Deno Deploy support

**Phase 11: Platform Support**
- [ ] Windows support (named pipes instead of Unix sockets)

**Phase 12: Advanced Features**
- [ ] OpenAPI/Swagger generation from routes
- [ ] Request validation framework (zod-style)
- [ ] Server-Side Rendering (SSR)
- [ ] Image optimization
- [ ] Middleware composition patterns
- [ ] Built-in form validation
- [ ] Database connection pooling helpers

---

## Quick Reference

### CLI Commands
```bash
zap dev                     # Dev server with hot reload
zap build                   # Production build
zap serve                   # Run production server
zap new my-app              # Create project
zap routes                  # Show route tree
zap codegen                 # Generate TS bindings
npx create-zap-app my-app   # Standalone scaffolding
```

### Route Conventions (Next.js Style)
| Pattern | URL |
|---------|-----|
| `index.tsx` | `/` |
| `about.tsx` | `/about` |
| `[id].tsx` | `/:id` (dynamic) |
| `[...slug].tsx` | `/*slug` (catch-all) |
| `[[...slug]].tsx` | `/*slug?` (optional catch-all) |
| `posts.[id].tsx` | `/posts/:id` |
| `_layout.tsx` | Scoped layout |
| `__root.tsx` | Root layout |
| `(group)/` | Route group (no URL) |
| `_private/` | Excluded folder |
| `api/*.ts` | API routes |

### Architecture
```
Zap Binary (Rust :3000)
├── Radix Router (9ns claimed)
├── Middleware (CORS, logging, rate limiting)
├── Static Files (ETag, Last-Modified, 304)
└── IPC Proxy → TS Handlers (Unix Socket, MessagePack)
```

### Production Bundle
```
dist/
├── bin/zap          # Rust binary (~4MB)
├── static/          # Frontend assets
├── config.json      # Server config
└── manifest.json    # Build metadata
```

### Type Safety (Core Feature)

**Rust to TypeScript codegen:**
```rust
#[export]
pub fn list_users(limit: u32) -> Result<ListUsersResponse, ApiError> { /* ... */ }
```

**Auto-generated TypeScript:**
```typescript
async listUsers(limit: number): Promise<ListUsersResponse | ApiError>
```

**Type Mappings:**
- `Result<T, E>` → `T | E` (discriminated union)
- `Option<T>` → `T | null`
- `Vec<T>` → `T[]`
- `HashMap<K,V>` → `Record<K,V>`

---

## Current Status Summary

**Engineering Audit Score:** 7,200/10,000

**What's Excellent (8.5-10/10):**
- Rust implementation (memory-safe, concurrent, zero-allocation routing)
- IPC protocol design (innovative MessagePack bridge)
- Production patterns (circuit breaker, exponential backoff, health checks)
- Performance optimizations (SIMD, connection pooling, LTO builds)

**What's Good (7-8/10):**
- File-based routing conventions
- Client-side router (useRouter, Link, SSG)
- Security middleware (headers, rate limiting, CORS)
- Observability (Prometheus, structured logging)

**What's Blocking Production (0-3/10):**
- Zero client-side tests (unacceptable for framework)
- Loose TypeScript config (`strict: false`)
- No CSRF protection (security vulnerability)
- Minimal documentation (97-line README)

**Verdict:** Promising framework with solid Rust core, needs TypeScript hardening before production use.

---

## License

MIT
