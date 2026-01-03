# Splice: Distributed Rust Functions Runtime

Production-ready Rust function execution for ZapJS via process isolation and stable protocol.

**Status:** Protocol complete. Context support ✅. CLI integration pending.

---

## The Problem

ZapJS uses `inventory::collect!` to discover `#[zap::export]` functions at startup. This fails when the runtime is pre-built and distributed via npm—user code compiled separately cannot register into a frozen binary's inventory.

```
Pre-built zap binary              User's Rust code
┌─────────────────────┐           ┌─────────────────────┐
│ inventory::collect! │     ✗     │ inventory::submit!  │
│ (frozen at build)   │ ←───────→ │ (separate compile)  │
└─────────────────────┘           └─────────────────────┘
```

**Solution:** Splice supervisor runs user code in a separate process. Protocol bridges the gap. Replace `inventory` with `linkme` distributed slices.

---

## Architecture

```
zap (HTTP server)
   │ connects via Unix socket
   ▼
splice (supervisor)  ← .zap/splice.sock
   │ spawns & monitors
   ▼
user-server (worker) ← ZAP_SOCKET env var
```

**Crash isolation:** User code crashes only kill worker. Splice restarts. Zap stays up.

---

## ✅ Completed

### Protocol Implementation
- **Location:** `packages/server/splice/src/protocol.rs`
- **Tests:** 145 passing (codec, state machines, concurrency, error recovery)
- **Features:** 18 message types, MessagePack framing, role-based protocol (Host/Worker)
- **RequestContext:** Already includes trace_id, span_id, headers, auth (lines 350-360)

### Splice Supervisor Binary
- **Binary:** `splice` (1.8MB) - packaged in all platform releases
- **Location:** `packages/server/splice-bin/src/main.rs`
- **Features:** Crash recovery (exponential backoff), circuit breaker, concurrency limits (1024 global, 100/function), health checks, hot reload support

### Zap Server Integration
- **Splice Client:** `packages/server/src/splice_client.rs` - handshake, export discovery, invocation, request ID correlation
- **Auto-connection:** Server detects `config.splice_socket_path` and connects automatically (server.rs:658-692)
- **Config:** `splice_socket_path: Option<String>` field in ZapConfig

### Worker Runtime
- **Location:** `packages/server/src/splice_worker.rs`
- **Features:** Connects to supervisor, MessagePack codec, uses existing `build_rpc_dispatcher()` from registry
- **Protocol:** MessagePack → JSON → dispatcher → JSON → MessagePack (lines 155-183)

---

## 📋 Implementation Checklist

### Phase 1: Remove Inventory & Add Context Support ✅

**Goal:** Enable user functions to access request context and remove inventory dependency.

- [x] **Add Context struct** (`packages/server/src/context.rs`)
  - ✅ Created Context wrapper with: trace_id, span_id, headers, auth (line 8-43)
  - ✅ Methods: trace_id(), span_id(), header(), headers(), auth(), user_id(), has_role()
  - ✅ Constructed from protocol's existing RequestContext via Context::new()
  - Note: Cancellation token deferred to Phase 3.5

- [x] **Switch from inventory to linkme** (`packages/server/src/registry.rs`)
  - ✅ Replaced `inventory::collect!` with `linkme::distributed_slice!` (line 78)
  - ✅ Defined `EXPORTS: [ExportedFunction]` distributed slice
  - ✅ Updated `build_rpc_dispatcher()` to iterate `EXPORTS` (line 118)
  - ✅ Removed inventory dependency from Cargo.toml

- [x] **Update export macro for linkme** (`packages/server/internal/macros/src/lib.rs`)
  - ✅ Replaced `inventory::submit!` with `#[linkme::distributed_slice(...)]` (line 389)
  - ✅ Added `is_context_type()` helper for Context parameter detection (line 78)
  - ✅ Modified wrapper to conditionally accept Context parameter (line 164-303)
  - ✅ Generates unique static names to avoid collisions: `__ZAP_EXPORT_{FUNCTION}` (line 383)
  - ✅ Full backward compatibility: functions without Context use Sync/Async variants

- [x] **Update splice_worker for Context** (`packages/server/src/splice_worker.rs`)
  - ✅ Updated dispatcher call to pass RequestContext (line 159)
  - ✅ Updated collect_exports() to use linkme EXPORTS (line 213)
  - Note: Context construction happens in registry dispatcher, not splice_worker

- [x] **Update registry for Context** (`packages/server/src/registry.rs`)
  - ✅ Expanded `FunctionWrapper` enum to 4 variants: Sync, Async, SyncCtx, AsyncCtx (line 18-27)
  - ✅ Added `has_context: bool` field to `ExportedFunction` (line 71)
  - ✅ Updated wrapper.call() to accept `Option<&Context>` (line 35-58)
  - ✅ Dispatcher constructs Context from RequestContext and passes to wrapper (line 150)

- [x] **Remove inventory entirely**
  - ✅ Deleted from packages/server/Cargo.toml
  - ✅ Deleted from packages/server/internal/macros/Cargo.toml
  - ✅ Removed `pub use inventory` from lib.rs (replaced with linkme)
  - ✅ Updated __private module exports (src/lib.rs:144)

**Tests:** All 83 tests passing. Registry functions work with and without Context parameter.

---

### Phase 2: CLI Integration

**Goal:** Wire Splice into dev/build/serve workflows.

#### TypeScript Utilities

- [ ] **Binary resolver** (`packages/client/src/cli/utils/binary-resolver.ts`)
  - Add `resolveSpliceBinary()` function
  - Resolve from `@zap-js/{platform}/bin/splice`
  - Return null if not found (graceful fallback)

- [ ] **Splice process manager** (`packages/client/src/runtime/splice-manager.ts`)
  - Class: `start()`, `stop()`, `waitForSocket()`
  - Spawn splice with `--socket` and `--worker` args
  - Handle stdout/stderr forwarding
  - Graceful shutdown: SIGTERM → wait → SIGKILL

- [ ] **User server builder** (`packages/client/src/cli/utils/user-server-builder.ts`)
  - `hasUserServer(projectDir)` - check for server/Cargo.toml
  - `buildUserServer({ projectDir, release })` - run cargo build
  - Return binary path: server/target/{debug|release}/server

#### CLI Commands

- [ ] **Dev command** (`packages/client/src/dev-server/server.ts`)
  - Check `hasUserServer(process.cwd())`
  - Build user server (debug mode for speed)
  - Spawn Splice supervisor before starting zap binary
  - Set `splice_socket_path` in ZapConfig
  - Cleanup Splice on shutdown

- [ ] **Build command** (`packages/client/src/cli/commands/build.ts`)
  - Check `hasUserServer(process.cwd())`
  - Run `cargo build --release` in server/ directory
  - Copy server/target/release/server to dist/bin/server
  - Copy splice binary to dist/bin/splice
  - Skip if no server/Cargo.toml (graceful)

- [ ] **Serve command** (`packages/client/src/cli/commands/serve.ts`)
  - Check for dist/bin/server and dist/bin/splice
  - Spawn Splice in production mode
  - Set splice_socket_path in config
  - Handle missing binaries gracefully (log warning, continue without Rust functions)

- [ ] **Type definitions** (`packages/client/src/runtime/types.ts`)
  - Add `splice_socket_path?: string` to ZapConfig interface

---

### Phase 3: Testing & Polish

**Goal:** Verify end-to-end and add developer experience improvements.

- [ ] **Create E2E test project**
  - Simple user-server with 2-3 exported functions
  - One sync function, one async function
  - Use Context to access headers and trace_id
  - Verify full workflow: dev → build → serve

- [ ] **Test crash recovery**
  - Function that panics
  - Verify supervisor restarts worker
  - Verify subsequent requests succeed

- [ ] **Test Context propagation**
  - Send request with custom headers
  - Verify function receives headers via ctx.header()
  - Verify trace_id propagates correctly

- [ ] **TypeScript codegen** (`packages/client/src/codegen/`)
  - Parse ListExportsResult from Splice
  - Generate TypeScript types from Rust signatures
  - Generate rpc.call() wrappers with proper types
  - Auto-import in project

- [ ] **Streaming support** (Phase 3.5 - Optional)
  - Verify StreamStart/StreamChunk/StreamEnd messages work
  - Test backpressure with StreamAck
  - Generate AsyncIterable wrappers in codegen

- [ ] **Hot reload E2E**
  - Modify Rust function while dev server running
  - Verify file watcher triggers cargo build
  - Verify Splice detects new binary and hot swaps
  - Verify zero downtime

---

## Protocol Reference (Existing)

### Invoke Message (Already Implemented)

```rust
struct Invoke {
    request_id: u64,
    function_name: String,
    params: Bytes,               // msgpack-encoded
    deadline_ms: u32,
    context: RequestContext,     // ← THIS IS ALREADY HERE
}

struct RequestContext {
    trace_id: u64,
    span_id: u64,
    headers: Vec<(String, String)>,
    auth: Option<AuthContext>,
}

struct AuthContext {
    user_id: String,
    roles: Vec<String>,
}
```

**Key insight:** We don't need a new SDK. RequestContext already exists in the protocol. We just need to expose it to user functions via a simple Context wrapper.

---

## User-Facing API (After Implementation)

```rust
use zap_server::{export, Context};

#[export]
pub async fn get_user(id: i64, ctx: Context) -> Result<User, String> {
    // Access trace ID for logging
    println!("trace_id: {}", ctx.trace_id());

    // Access headers
    if let Some(api_key) = ctx.header("x-api-key") {
        // Authenticate
    }

    // Access auth context
    if let Some(user_id) = ctx.user_id() {
        println!("Request from user: {}", user_id);
    }

    // Check cancellation
    if ctx.is_cancelled() {
        return Err("Request cancelled".to_string());
    }

    // Function logic
    Ok(User { id, name: "John".to_string() })
}

// Functions without Context still work (backward compatible)
#[export]
pub fn health_check() -> String {
    "OK".to_string()
}
```

---

## Key Files to Modify

### Phase 1: Context & Linkme
- `packages/server/src/context.rs` (NEW)
- `packages/server/src/registry.rs` (MODIFY: linkme, Context support)
- `packages/server/internal/macros/src/lib.rs` (MODIFY: linkme, Context param detection)
- `packages/server/src/splice_worker.rs` (MODIFY: construct Context, pass to dispatcher)
- `packages/server/Cargo.toml` (MODIFY: remove inventory, add linkme)
- `packages/server/internal/macros/Cargo.toml` (MODIFY: remove inventory, add linkme)

### Phase 2: CLI Integration
- `packages/client/src/cli/utils/binary-resolver.ts` (MODIFY: add resolveSpliceBinary)
- `packages/client/src/runtime/splice-manager.ts` (NEW)
- `packages/client/src/cli/utils/user-server-builder.ts` (NEW)
- `packages/client/src/dev-server/server.ts` (MODIFY: Splice integration)
- `packages/client/src/cli/commands/build.ts` (MODIFY: cargo build, copy binaries)
- `packages/client/src/cli/commands/serve.ts` (MODIFY: spawn Splice)
- `packages/client/src/runtime/types.ts` (MODIFY: add splice_socket_path)

### Phase 3: Testing & Codegen
- `tests/e2e/splice-integration/` (NEW: E2E test project)
- `packages/client/src/codegen/splice.ts` (NEW: TypeScript generation)

---

## Success Criteria

- [ ] Users can write `#[zap::export]` functions in server/ directory
- [x] Functions work with pre-built npm binaries (no compilation of zap needed) ✅ linkme migration
- [x] Context parameter provides access to trace_id, headers, auth ✅ Context wrapper API
- [ ] `zap dev` automatically builds and runs user server via Splice
- [ ] `zap build` packages user server and splice binaries
- [ ] `zap serve` runs Splice in production
- [x] Zero inventory dependency anywhere in codebase ✅ Removed from all packages
- [ ] Full E2E test coverage (Phase 1: ✅ 83/83 tests passing, Phase 2/3: pending)

---

## Notes

- **No separate SDK crate needed** - everything lives in `zap_server`
- **RequestContext already exists** - just need to expose it
- **Linkme works across compilation boundaries** - solves inventory problem
- **Backward compatible** - functions without Context still work
- **Graceful fallback** - if no server/Cargo.toml, everything still works (just no Rust functions)
