# ZapJS Implementation Plan

> Fullstack web framework: React frontend + Rust backend compiled into a single deployable binary.

**Status:** Phase 10.1-10.3 Complete (Security, Observability, Error Handling) | **Updated:** December 2024

---

## Completed (Phases 1-7 + Type Safety + Phase 10.1-10.3)

| Phase | Summary |
|-------|---------|
| 1. Monorepo | pnpm + Cargo workspaces, `/packages/` structure, 68+ tests |
| 2. Proc Macros | `#[zap::export]`, syn parser, zap-codegen for TS bindings |
| 3. CLI | `zap new/dev/build/serve/codegen/routes` commands |
| 4. Dev Server | Hot reload (Rust + TS), file watching, WebSocket HMR |
| 5. Production | LTO builds, Docker, cross-compilation, graceful shutdown |
| 6. App Router | TanStack-style file routing, API routes, route tree codegen |
| 7. create-zap-app | `npx create-zap-app`, templates, package manager selection |
| **Type Safety** | Full bidirectional Rust↔TypeScript type safety with union types |
| **10.1 Security** | Security headers, rate limiting, strict CORS middleware |
| **10.2 Observability** | Prometheus metrics, X-Request-ID correlation, structured logging |
| **10.3 Error Handling** | React ErrorBoundary, useRouteError hook, TanStack-style errorComponent |

**All packages complete:** `@zapjs/runtime`, `@zapjs/cli`, `@zapjs/dev-server`, `@zapjs/router`, `create-zap-app`, `zap-core`, `zap-server`, `zap-macros`, `zap-codegen`

**Performance achieved:** 9ns static routes, 80-200ns dynamic, ~100μs IPC, ~4MB binary

---

## Roadmap

### Phase 8: Enhanced RPC
- [ ] MessagePack serialization (replace JSON)
- [ ] Streaming responses
- [ ] WebSocket mode option

### Phase 9: Edge/WASM Runtime
- [ ] Compile Rust to WASM
- [ ] Vercel/Cloudflare Workers support
- [ ] Deno Deploy support

### Phase 10: Production Hardening

#### 10.1 Security ✅ COMPLETE
- [x] Security headers middleware (X-Frame-Options, HSTS, CSP, X-Content-Type-Options)
- [x] Rate limiting middleware (100 req/min default, pluggable storage: memory/Redis)
- [x] Strict CORS by default (require explicit origin list)
- [ ] Request validation framework (zod-style schema validation) - *deferred*

#### 10.2 Observability ✅ COMPLETE
- [x] Prometheus metrics endpoint (`/metrics`)
  - `http_requests_total{method, path, status}`
  - `http_request_duration_seconds{method, path}`
  - `http_requests_in_flight`
- [x] Request ID correlation (X-Request-ID header, passed through IPC)
- [x] Structured JSON logging with trace context

#### 10.3 Error Handling ✅ COMPLETE
- [x] React ErrorBoundary with TanStack-style `errorComponent` prop
- [x] `useRouteError()` hook for error pages
- [x] DefaultErrorComponent fallback UI
- [x] Route scanner detects `errorComponent` exports
- [x] Codegen wires error components automatically

#### 10.4 Caching & Performance
- [ ] ETag generation for static files
- [ ] If-None-Match → 304 Not Modified support
- [ ] Last-Modified headers

#### 10.5 Reliability
- [ ] IPC retry logic (3 retries, exponential backoff)
- [ ] Circuit breaker for persistent handler failures
- [ ] Enhanced health checks (`/health/live`, `/health/ready`)

#### 10.6 Type Safety ✅ COMPLETE
- [x] Full bidirectional Rust↔TypeScript type safety
- [x] `Result<T, ApiError>` return types generate `T | ApiError` union types
- [x] Automatic codegen from Rust source (no manual type definitions)
- [x] 19 typed response interfaces generated automatically
- [x] Discriminated union pattern for error handling

### Phase 11: Platform Support
- [ ] Windows support (named pipes instead of Unix sockets)

### Phase 12: Documentation
- [ ] OpenAPI/Swagger generation from routes
- [ ] Full API reference docs
- [ ] Tutorial guides

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

### Route Conventions
| Pattern | URL |
|---------|-----|
| `index.tsx` | `/` |
| `$param.tsx` | `/:param` |
| `posts.$id.tsx` | `/posts/:id` |
| `_layout.tsx` | Pathless layout |
| `__root.tsx` | Root layout |
| `(group)/` | Route group |
| `api/*.ts` | API routes |

### API Route Example
```typescript
// routes/api/users.$id.ts
export const GET = async ({ params }: { params: { id: string } }) => {
  return { id: params.id, name: `User ${params.id}` };
};
```

### Architecture
```
Zap Binary (Rust :3000)
├── Radix Router (9ns)
├── Middleware (CORS, logging)
├── Static Files
└── IPC Proxy → TS Handlers (Unix Socket)
```

### Production Bundle
```
dist/
├── bin/zap          # Rust binary
├── static/          # Frontend assets
├── config.json      # Server config
└── manifest.json    # Build metadata
```

---

## Production Features (Phase 10)

### Security (10.1)

**Security Headers** - Applied automatically to all responses:
```
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
X-XSS-Protection: 1; mode=block
Strict-Transport-Security: max-age=31536000; includeSubDomains
Referrer-Policy: strict-origin-when-cross-origin
Content-Security-Policy: <configurable>
```

**Rate Limiting** - Token bucket algorithm per IP:
- Default: 100 requests/minute
- Returns 429 with `Retry-After` header
- Pluggable storage (in-memory default, Redis optional)

**Strict CORS** - Explicit origin allowlist required:
```typescript
cors: {
  origins: ['https://app.example.com'],
  methods: ['GET', 'POST'],
  credentials: true,
}
```

### Observability (10.2)

**Prometheus Metrics** at `/metrics`:
```
http_requests_total{method="GET", path="/api/users", status="200"} 1234
http_request_duration_seconds{method="GET", path="/api/users"} 0.015
http_requests_in_flight 5
ipc_invoke_duration_seconds{handler_id="handler_0"} 0.008
```

**Request ID Correlation**:
- Incoming `X-Request-ID` header preserved
- Auto-generated UUID if not present
- Passed through IPC to TypeScript handlers
- Included in all log entries

**Structured Logging**:
```typescript
import { logger } from '@zapjs/runtime';

logger.info('User created', { request_id, userId: '123' });
// {"level":"info","message":"User created","request_id":"abc-123","userId":"123","timestamp":"..."}
```

### Error Handling (10.3)

**TanStack-style errorComponent** - Export from route files:
```typescript
// routes/users.$id.tsx
export default function UserPage({ params }) {
  return <UserProfile userId={params.id} />;
}

export function errorComponent({ error, reset }) {
  return (
    <div>
      <h1>Failed to load user</h1>
      <p>{error.message}</p>
      {error.digest && <small>Error ID: {error.digest}</small>}
      <button onClick={reset}>Try Again</button>
    </div>
  );
}
```

**useRouteError Hook**:
```typescript
import { useRouteError } from '@zapjs/runtime';

export function errorComponent() {
  const { error, reset } = useRouteError();
  return <MyErrorUI error={error} onRetry={reset} />;
}
```

**ZapRouteError Interface**:
```typescript
interface ZapRouteError {
  message: string;
  code?: string;      // "HANDLER_ERROR", "VALIDATION_ERROR", etc.
  status?: number;    // HTTP status code
  digest?: string;    // Server error correlation ID
  stack?: string;     // Stack trace (dev only)
  details?: Record<string, unknown>;
}
```

---

## Bidirectional Type Safety (Core Feature)

ZapJS provides **zero-cost bidirectional type safety** between Rust and TypeScript - the core differentiator of the framework.

### How It Works

1. **Rust functions** use `#[export]` macro with typed returns:
```rust
#[export]
pub fn list_users(limit: u32, offset: u32) -> Result<ListUsersResponse, ApiError> {
    // Implementation
}
```

2. **Codegen** scans Rust source and generates TypeScript:
```typescript
// Auto-generated
async listUsers(limit: number, offset: number): Promise<ListUsersResponse | ApiError>
```

3. **TypeScript** gets full type safety with discriminated unions:
```typescript
const result = await backend.listUsers(10, 0);
if ('error' in result) {
  // TypeScript KNOWS this is ApiError
  console.error(result.code);
} else {
  // TypeScript KNOWS this is ListUsersResponse
  console.log(result.users, result.total);
}
```

### Generated Files

| File | Purpose |
|------|---------|
| `types.ts` | All Rust structs as TypeScript interfaces |
| `backend.ts` | Flat function exports with full types |
| `server.ts` | Namespaced server client |
| `backend.d.ts` | Type declarations |

### Type Mappings

| Rust | TypeScript |
|------|------------|
| `String` | `string` |
| `u32`, `i32`, `usize` | `number` |
| `bool` | `boolean` |
| `Vec<T>` | `T[]` |
| `Option<T>` | `T \| null` |
| `HashMap<K, V>` | `Record<K, V>` |
| `Result<T, E>` | `T \| E` (union type) |
| Custom structs | Generated interfaces |

### Automatic Struct Detection

Any struct with `#[derive(Serialize)]` is automatically converted:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}
```

Generates:
```typescript
export interface User {
  id: string;
  name: string;
  createdAt: string;
}
```

---

## Production Readiness Assessment

### Current State: Type-Safe Rust Backend Library

ZapJS is production-ready as a **type-safe Rust RPC backend** - comparable to:
- tRPC (type-safe RPC)
- Remix loaders/actions (server functions)
- SvelteKit form actions

### What's Ready

| Feature | Status |
|---------|--------|
| Bidirectional type safety | ✅ Complete |
| Automatic codegen | ✅ Complete |
| Typed error handling | ✅ Complete |
| HTTP server | ✅ Complete |
| IPC to TypeScript | ✅ Complete |
| File-based routing | ✅ Basic |
| Hot reload | ✅ Complete |
| Production builds | ✅ Complete |
| Security headers | ✅ Complete |
| Rate limiting | ✅ Complete |
| Strict CORS | ✅ Complete |
| Prometheus metrics | ✅ Complete |
| Request ID correlation | ✅ Complete |
| Structured logging | ✅ Complete |
| React ErrorBoundary | ✅ Complete |
| useRouteError hook | ✅ Complete |

### Gaps vs Next.js

| Feature | Next.js | ZapJS |
|---------|---------|-------|
| SSR/SSG | Built-in streaming | Not implemented |
| React integration | First-class | Manual |
| Image optimization | Built-in | None |
| Middleware | Edge middleware | Security, rate limiting, CORS |
| Data fetching | fetch() caching, ISR | Manual |
| Layouts/templates | Nested layouts | None |
| Metadata API | SEO, OpenGraph | None |
| Deployment | Vercel, any Node host | Custom |
| Error boundaries | error.tsx convention | TanStack-style errorComponent |
| Observability | Manual | Prometheus, structured logging |

### Recommended Use Cases

**Good Fit:**
- APIs needing Rust performance (CPU-intensive, real-time)
- Type-safe backend for existing React/Vue/Svelte apps
- Microservices with strict type contracts
- Projects prioritizing type safety over ecosystem

**Not Yet Ready For:**
- Full-stack React apps (use Next.js + ZapJS backend)
- Static site generation
- Edge deployment
- Teams needing extensive documentation/ecosystem

### Path Forward

**Option A: Backend Library** (Current)
Position as a high-performance type-safe backend that complements Next.js/Remix

**Option B: Full Framework** (6-12 months)
Would require: React SSR, build tooling, static generation, edge runtime, docs

---

## License

MIT
