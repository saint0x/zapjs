# Splice: Rust↔TypeScript Runtime Bridge

**Production-grade protocol enabling TypeScript to seamlessly call Rust functions across runtime boundaries.**

Splice bridges the gap between TypeScript's V8/JSC runtime and Rust's Tokio runtime, allowing ZapJS applications to execute high-performance Rust functions from TypeScript code. Instead of complex FFI bindings (node-gyp, napi-rs), Splice uses a lightweight MessagePack protocol over Unix sockets, enabling:

**The Core Problem:**
TypeScript applications need to call Rust functions without:
- Platform-specific native compilation (node-gyp, napi-rs, wasm)
- Tight coupling between Node.js and Rust versions
- Recompiling on every dependency update
- Complex build toolchains for every platform

**The Splice Solution:**
```
TypeScript Runtime          Protocol Bridge          Rust Runtime
┌─────────────────┐        ┌──────────┐        ┌─────────────────┐
│ Node.js / Bun   │        │  Splice  │        │  Tokio Async    │
│  (V8 / JSC)     │◄──────►│  Socket  │◄──────►│   Runtime       │
│                 │  JSON  │          │  Rust  │                 │
│ await rpc.call( │        │ Message  │        │ #[export]       │
│  "users.create",│        │  Pack    │        │ pub fn create() │
│  {name: "..."}  │        │ Protocol │        │                 │
│ )               │        │          │        │                 │
└─────────────────┘        └──────────┘        └─────────────────┘
```

**Key Benefits:**
- **Zero Native Compilation**: Distribute pre-built Rust binaries via npm
- **Runtime Isolation**: TypeScript and Rust run in separate processes
- **Type Safety**: Full TypeScript types auto-generated from Rust signatures
- **Hot Reload**: Update Rust functions without restarting Node.js
- **Crash Isolation**: Rust panics don't crash the TypeScript server

```
┌────────────────────────────────────────────────────────────┐
│  Problem: TypeScript ↔ Rust Integration                   │
│                                                            │
│  Option 1: FFI (node-gyp, napi-rs)                        │
│  ┌────────────┐                    ┌─────────────┐        │
│  │ TypeScript │─── C bindings ────►│ Rust (NAPI) │        │
│  └────────────┘                    └─────────────┘        │
│  ✗ Platform-specific compilation                          │
│  ✗ Node.js version coupling                               │
│  ✗ Complex build toolchain                                │
│                                                            │
│  Option 2: WebAssembly                                    │
│  ┌────────────┐                    ┌─────────────┐        │
│  │ TypeScript │──── WASM ABI ─────►│ Rust (WASM) │        │
│  └────────────┘                    └─────────────┘        │
│  ✗ No async/await support                                 │
│  ✗ No direct I/O access                                   │
│  ✗ Limited ecosystem                                      │
└────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────┐
│  Solution: Splice Protocol Bridge                         │
│                                                            │
│  ┌────────────┐     Unix Socket     ┌─────────────┐       │
│  │ TypeScript │◄─── MessagePack ───►│ Rust Tokio  │       │
│  │  Runtime   │     Protocol        │  Runtime    │       │
│  └────────────┘                     └─────────────┘       │
│  ✓ Zero compilation (pre-built binaries)                  │
│  ✓ Runtime isolation (crash safety)                       │
│  ✓ Full async/await support                               │
│  ✓ Type-safe RPC with codegen                             │
└────────────────────────────────────────────────────────────┘
```

---

## Architecture Overview

### Runtime Bridge Architecture

Splice bridges two completely separate runtime environments:

```
┌───────────────────────────────────────────────────────────────────────┐
│                    TypeScript Runtime (Node.js/Bun)                   │
│  ┌────────────────────────────────────────────────────────┐           │
│  │ JavaScript Event Loop (libuv / Bun event loop)         │           │
│  │  • HTTP request handling                               │           │
│  │  • Express/Hono route processing                       │           │
│  │  • await rpc.call("users.create", {name: "Alice"})     │           │
│  └─────────────────────┬──────────────────────────────────┘           │
│                        │                                              │
│  ┌─────────────────────▼──────────────────────────────────┐           │
│  │ SpliceClient (TypeScript)                              │           │
│  │  • Serializes TypeScript objects to MessagePack        │           │
│  │  • Sends RPC over Unix socket                          │           │
│  │  • Deserializes MessagePack responses to TypeScript    │           │
│  └─────────────────────┬──────────────────────────────────┘           │
└────────────────────────┼──────────────────────────────────────────────┘
                         │
                         │ Unix Socket (.zap/splice.sock)
                         │ Protocol: MessagePack-encoded RPC
                         │
┌────────────────────────▼──────────────────────────────────────────────┐
│                    Splice Supervisor (Rust)                           │
│  ┌────────────────────────────────────────────────────────┐           │
│  │ Protocol Bridge                                        │           │
│  │  • Receives MessagePack from TypeScript                │           │
│  │  • Routes to appropriate Rust worker                   │           │
│  │  • Handles concurrency limits & timeouts               │           │
│  │  • Returns MessagePack responses to TypeScript         │           │
│  └─────────────────────┬──────────────────────────────────┘           │
└────────────────────────┼──────────────────────────────────────────────┘
                         │
                         │ Unix Socket (worker.sock)
                         │ Protocol: MessagePack RPC + Context
                         │
┌────────────────────────▼──────────────────────────────────────────────┐
│                    Rust Runtime (Tokio)                               │
│  ┌────────────────────────────────────────────────────────┐           │
│  │ User-Server Worker Process                             │           │
│  │  • Tokio async runtime                                 │           │
│  │  • #[export] function registry (linkme)                │           │
│  │  • pub async fn users_create(ctx: &Context, ...) {}    │           │
│  │  • Full access to Rust ecosystem (sqlx, reqwest, etc.) │           │
│  └────────────────────────────────────────────────────────┘           │
└───────────────────────────────────────────────────────────────────────┘

Flow: TypeScript → JSON → MessagePack → Supervisor → Worker → Rust Function
      Rust Result → MessagePack → Supervisor → TypeScript → Typed Object
```

### Component Responsibilities

| Component | File | Responsibilities |
|-----------|------|------------------|
| **Host Client** | `packages/server/src/splice_client.rs` | • RPC function dispatcher<br>• Export metadata caching<br>• Connection management<br>• Response correlation |
| **Supervisor** | `packages/server/splice-bin/src/main.rs` | • Worker process spawning<br>• Crash detection and restart<br>• Request routing and load balancing<br>• Health monitoring |
| **Worker Runtime** | `packages/server/src/splice_worker.rs` | • Protocol message handling<br>• Function dispatch via linkme<br>• Concurrent request execution<br>• Cooperative cancellation |
| **Protocol Library** | `packages/server/splice/src/protocol.rs` | • Message type definitions<br>• MessagePack codec implementation<br>• Error taxonomy |
| **Router** | `packages/server/splice/src/router.rs` | • Concurrency limit enforcement<br>• Request ID allocation<br>• Timeout handling<br>• Response correlation |
| **Supervisor Logic** | `packages/server/splice/src/supervisor.rs` | • Worker state machine<br>• Exponential backoff calculation<br>• Circuit breaker logic<br>• Graceful shutdown |

### Cross-Runtime Call Flow

**TypeScript → Rust (Request Path):**
```
TypeScript Code:
  const user = await rpc.call("users.create", {name: "Alice", age: 30})
    │
    │ [TypeScript Runtime - V8/JSC]
    ▼
  SpliceClient.invoke()
    • Converts TypeScript object {name: "Alice", age: 30} to JSON
    • Serializes JSON to MessagePack bytes
    • Allocates request_id = 42
    │
    │ [Runtime Boundary - Unix Socket]
    ▼
  Supervisor receives MessagePack bytes
    • Deserializes to Message::Invoke { request_id: 42, ... }
    • Router checks concurrency limits
    • Forwards to worker process
    │
    │ [Runtime Boundary - Unix Socket]
    ▼
  Worker receives in Rust Tokio runtime
    • Deserializes MessagePack to serde_json::Value
    • Dispatcher finds "users.create" in linkme registry
    • Calls: users_create(ctx, {name: "Alice", age: 30})
    │
    │ [Rust Runtime - Tokio]
    ▼
  User function executes:
    pub async fn users_create(ctx: &Context, params: UserParams) -> Result<User> {
        let db = ctx.get_db();
        db.users.insert(params.name, params.age).await
    }
```

**Rust → TypeScript (Response Path):**
```
  [Rust Runtime - Tokio]
  Function returns: Ok(User { id: 123, name: "Alice", age: 30 })
    │
    ▼
  Worker serializes result
    • Converts Rust struct to serde_json::Value
    • Serializes to MessagePack bytes
    • Sends Message::InvokeResult { request_id: 42, result: [...] }
    │
    │ [Runtime Boundary - Unix Socket]
    ▼
  Supervisor Router
    • Receives MessagePack bytes
    • Matches request_id = 42 to pending request
    • Forwards to SpliceClient via socket
    │
    │ [Runtime Boundary - Unix Socket]
    ▼
  SpliceClient receives in TypeScript runtime
    • Deserializes MessagePack to Uint8Array
    • Parses to JavaScript object
    • Returns typed result: {id: 123, name: "Alice", age: 30}
    │
    │ [TypeScript Runtime - V8/JSC]
    ▼
  TypeScript code receives:
    const user: User = {id: 123, name: "Alice", age: 30}
    // Full type safety with auto-generated TypeScript types!
```

**The Bridge in Action:**
1. **TypeScript objects** → JSON → MessagePack → **Rust structs**
2. **Async/await in TypeScript** transparently waits for **async Rust execution**
3. **Rust errors** (`Result::Err`) → Protocol errors → **TypeScript exceptions**
4. **Rust types** → JSON Schema → **TypeScript interfaces** (auto-generated)

---

## Core Modules

### Protocol Module

**File:** `packages/server/splice/src/protocol.rs` (1747 lines)

The protocol module defines the complete MessagePack-based wire protocol for Splice communication.

**Message Categories (18 total):**

1. **Connection Lifecycle**
   - `Handshake`: Initial connection with protocol version and capabilities
   - `HandshakeAck`: Connection acceptance with negotiated capabilities
   - `Shutdown`: Graceful termination request
   - `ShutdownAck`: Confirmation of shutdown

2. **Function Registry**
   - `ListExports`: Request all available functions
   - `ListExportsResult`: Function metadata list

3. **Invocation**
   - `Invoke`: Execute function with params and context
   - `InvokeResult`: Successful function response
   - `InvokeError`: Function execution error

4. **Streaming** (Future - Phase 3.5)
   - `StreamStart`, `StreamChunk`, `StreamEnd`, `StreamError`, `StreamAck`

5. **Cancellation**
   - `Cancel`: Request cancellation of in-flight request
   - `CancelAck`: Cancellation signal acknowledged

6. **Observability**
   - `LogEvent`: Worker log forwarding
   - `HealthCheck`, `HealthStatus`: Health monitoring

**Frame Format:**
```
[4 bytes: length (big-endian)] [1 byte: message type] [MessagePack payload]
```

**Error Taxonomy:**

| Range | Category | Examples |
|-------|----------|----------|
| 1000-1999 | Client Errors | Invalid request (1000), Invalid params (1001), Function not found (1002), Unauthorized (1003), Frame too large (1004) |
| 2000-2999 | Execution Errors | Execution failed (2000), Timeout (2001), Cancelled (2002), Panic (2003) |
| 3000-3999 | System Errors | Internal error (3000), Unavailable (3001), Overloaded (3002) |

**RequestContext Structure:**
```rust
struct RequestContext {
    trace_id: u64,      // Distributed tracing ID
    span_id: u64,       // Span identifier for this operation
    headers: Vec<(String, String)>,  // HTTP headers
    auth: Option<AuthContext>,       // User authentication
}

struct AuthContext {
    user_id: String,
    roles: Vec<String>,
}
```

**Codec Implementation:**

The `SpliceCodec` implements Tokio's `Encoder` and `Decoder` traits for efficient framing:
- Max frame size: 100MB (configurable via `DEFAULT_MAX_FRAME_SIZE`)
- Uses `rmp_serde` for MessagePack serialization
- Handles partial frames and backpressure

### Supervisor Module

**File:** `packages/server/splice/src/supervisor.rs` (247 lines)

Manages worker process lifecycle with automatic crash recovery.

**Worker State Machine:**
```
Starting ──[process spawned]──> Ready
  │                               │
  │                               │ [drain requested]
  │                               ▼
  │                            Draining
  │                               │
  │                               │ [graceful shutdown]
  │                               ▼
  └─────[spawn failed]─────> Failed ──[backoff]──> Starting
                                │
                                │ [max restarts exceeded]
                                ▼
                          CircuitBreaker (30s cooldown)
```

**Crash Recovery Strategy:**

- **Exponential Backoff Schedule:** `[0ms, 100ms, 500ms, 2s, 5s]`
- **Max Restart Attempts:** 10 within sliding window
- **Circuit Breaker:** After max failures, 30-second cooldown before allowing retries
- **Graceful Shutdown:** SIGTERM with 30s drain timeout, then SIGKILL if unresponsive

**Configuration Defaults:**
```rust
SupervisorConfig {
    max_restarts: 10,
    restart_backoff: [0ms, 100ms, 500ms, 2s, 5s],
    health_check_interval: 5s,
    drain_timeout: 30s,
    connect_timeout: 10s,
}
```

### Router Module

**File:** `packages/server/splice/src/router.rs` (261 lines)

Routes invocation requests with concurrency limits and timeout handling.

**Concurrency Management:**

- **Global Limit:** 1024 concurrent requests (prevents memory exhaustion)
- **Per-Function Limit:** 256 concurrent requests (prevents monopolization)
- **Request Tracking:** `HashMap<u64, PendingRequest>` with oneshot channels
- **Automatic Cleanup:** Removes entries on completion or timeout

**Router.invoke() Flow:**

```rust
async fn invoke(
    function_name: String,
    params: Bytes,
    deadline_ms: u64,
    context: RequestContext,
) -> Result<Bytes, RouterError> {
    // 1. Check global concurrency limit
    if pending.len() >= max_concurrent_requests {
        return Err(RouterError::Overloaded);
    }

    // 2. Check per-function concurrency limit
    if function_counts[&function_name] >= max_concurrent_per_function {
        return Err(RouterError::Overloaded);
    }

    // 3. Allocate request ID and create oneshot channel
    let request_id = self.next_request_id;
    let (result_tx, result_rx) = oneshot::channel();

    // 4. Track pending request
    pending.insert(request_id, PendingRequest { result_tx, ... });

    // 5. Send Invoke message to worker via mpsc
    worker_tx.send(Message::Invoke { request_id, ... }).await?;

    // 6. Wait for response with timeout
    let timeout = Duration::from_millis(deadline_ms or default_timeout);
    match tokio::time::timeout(timeout, result_rx).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(_)) => Err(RouterError::Cancelled),
        Err(_) => {
            // Timeout: Send Cancel message
            worker_tx.send(Message::Cancel { request_id }).await;
            Err(RouterError::Timeout)
        }
    }
}
```

**Bidirectional Communication (Phase 4):**

- **Supervisor → Worker:** mpsc channel (`supervisor_tx` → Bridge Task 1 → worker socket)
- **Worker → Supervisor:** Message handler (worker socket → Bridge Task 2 → `Router.handle_worker_message()`)
- **Response Correlation:** Request ID matching via `pending` HashMap

### Reload Module

**File:** `packages/server/splice/src/reload.rs` (90 lines)

Enables zero-downtime updates through binary change detection.

**Hot Reload Sequence:**

1. **Change Detection:** SHA256 hash of worker binary
2. **Drain Requests:** Wait for in-flight requests (configurable timeout)
3. **Graceful Shutdown:** SIGTERM to worker with 30s grace period
4. **Supervisor Restarts:** Automatic spawn of new worker process
5. **Export Refresh:** New `ListExports` request to update function registry
6. **TypeScript Codegen:** `runSpliceCodegen()` regenerates bindings
7. **Browser Reload:** HotReloadServer notifies frontend

**Integration Points:**

- **DevServer:** File watcher detects `server/**/*.rs` changes
- **Build Trigger:** `cargo build` in response to file change
- **Codegen:** Connects to Splice, fetches exports, generates TypeScript types

### Metrics Module

**File:** `packages/server/splice/src/metrics.rs` (105 lines)

Lock-free observability counters using atomic operations.

**Tracked Metrics:**

- `total_requests`: Cumulative request count since startup
- `successful_requests`: Completed without error
- `failed_requests`: Returned error (user or system)
- `timeout_requests`: Exceeded deadline
- `cancelled_requests`: Explicitly cancelled
- `active_requests`: Currently executing
- `uptime_ms`: Server uptime in milliseconds

**Implementation:** `AtomicU64` with `Ordering::Relaxed` for performance (metrics don't require strict ordering)

---

## Protocol Specification

### Handshake Sequence

```
Worker                           Supervisor                         Host
  │                                   │                               │
  │ Handshake                         │                               │
  │ (protocol_version: 0x00010000)    │                               │
  │ (role: Worker)                    │                               │
  │ (capabilities: 0b11)              │                               │
  ├──────────────────────────────────>│                               │
  │                                   │                               │
  │                 HandshakeAck      │                               │
  │          (negotiated caps: 0b11)  │                               │
  │<──────────────────────────────────┤                               │
  │                                   │                               │
  │ ListExportsResult                 │                               │
  │ (exports: [...])                  │                               │
  ├──────────────────────────────────>│                               │
  │                                   │                               │
  │                                   │    Handshake                  │
  │                                   │    (role: Host)               │
  │                                   │<──────────────────────────────┤
  │                                   │                               │
  │                                   │    HandshakeAck               │
  │                                   │    (export_count: 9)          │
  │                                   ├──────────────────────────────>│
  │                                   │                               │
  │                                   │    ListExports                │
  │                                   │<──────────────────────────────┤
  │                                   │                               │
  │                                   │    ListExportsResult          │
  │                                   ├──────────────────────────────>│
```

**Capability Negotiation:** Bitwise AND of capabilities
- `CAP_STREAMING` (0x01): Supports streaming requests/responses
- `CAP_CANCELLATION` (0x02): Supports request cancellation
- `CAP_COMPRESSION` (0x04): Supports payload compression (future)

### Function Discovery

**ListExports Request:**
- No payload required
- Sent by host after handshake

**ListExportsResult Response:**
```rust
struct ExportMetadata {
    name: String,              // Function name (e.g., "users.create")
    is_async: bool,            // Async function
    is_streaming: bool,        // Returns AsyncIterable (future)
    params_schema: String,     // JSON Schema for parameters
    return_schema: String,     // JSON Schema for return type
}
```

### Invocation Protocol

**Invoke Message:**
```rust
Message::Invoke {
    request_id: u64,                    // Unique request identifier
    function_name: String,              // e.g., "users.create"
    params: Bytes,                      // MessagePack-serialized params
    deadline_ms: u64,                   // 0 = use default timeout
    context: RequestContext,            // Trace ID, headers, auth
}
```

**InvokeResult Response:**
```rust
Message::InvokeResult {
    request_id: u64,                    // Matches Invoke request_id
    result: Bytes,                      // MessagePack-serialized result
    duration_us: u64,                   // Execution time in microseconds
}
```

**InvokeError Response:**
```rust
Message::InvokeError {
    request_id: u64,
    code: u32,                          // Error code (see taxonomy)
    kind: ErrorKind,                    // User/System/Timeout/Cancelled
    message: String,                    // Human-readable error
    details: Option<String>,            // Additional context (stack trace, etc.)
}
```

### Cancellation Protocol

**Cancel Message:**
- Sent by supervisor on timeout or explicit cancellation request
- Triggers `CancellationToken` in worker for corresponding request_id

**Cooperative Cancellation:**
- Functions must check `ctx.is_cancelled()` or use `tokio::select!` with `ctx.cancelled()`
- Not preemptive - long-running functions can ignore cancellation

**CancelAck Response:**
- Confirms cancellation signal was received
- Does **not** guarantee function stopped (depends on cooperation)

### Streaming Protocol (Future - Phase 3.5)

**StreamStart Message:**
- Indicates beginning of streaming response
- Includes window size for flow control

**StreamChunk Message:**
- Contains data chunk with sequence number
- Client sends `StreamAck` to advance window

**StreamEnd Message:**
- Signals completion of stream
- Includes total chunk count for verification

### Message Type Reference

| Code | Message | Direction | Payload | Purpose |
|------|---------|-----------|---------|---------|
| 0x01 | Handshake | Worker→Supervisor, Host→Supervisor | protocol_version, role, capabilities | Connection initialization |
| 0x02 | HandshakeAck | Supervisor→Worker, Supervisor→Host | capabilities, server_id, export_count | Connection accepted |
| 0x03 | Shutdown | Supervisor→Worker, Host→Supervisor | - | Graceful termination |
| 0x04 | ShutdownAck | Worker→Supervisor, Supervisor→Host | - | Shutdown confirmed |
| 0x05 | ListExports | Host→Supervisor | - | Request function list |
| 0x06 | ListExportsResult | Supervisor→Host, Worker→Supervisor | exports: Vec<ExportMetadata> | Function metadata |
| 0x07 | Invoke | Supervisor→Worker | request_id, function_name, params, deadline_ms, context | Execute function |
| 0x08 | InvokeResult | Worker→Supervisor | request_id, result, duration_us | Success response |
| 0x09 | InvokeError | Worker→Supervisor | request_id, code, kind, message, details | Error response |
| 0x0A | StreamStart | Worker→Supervisor | request_id, window_size | Begin stream |
| 0x0B | StreamChunk | Worker→Supervisor | request_id, sequence, data | Stream data chunk |
| 0x0C | StreamEnd | Worker→Supervisor | request_id, total_chunks | Stream completion |
| 0x0D | StreamError | Worker→Supervisor | request_id, code, message | Stream error |
| 0x0E | StreamAck | Supervisor→Worker | request_id, acknowledged_sequence | Flow control ack |
| 0x0F | Cancel | Supervisor→Worker | request_id | Cancel request |
| 0x10 | CancelAck | Worker→Supervisor | request_id | Cancellation received |
| 0x11 | LogEvent | Worker→Supervisor | level, target, message, fields | Log forwarding |
| 0x12 | HealthCheck | Supervisor→Worker | - | Health probe |
| 0x13 | HealthStatus | Worker→Supervisor | healthy, metrics | Health response |

---

## Design Decisions and Trade-offs

### Why Process Isolation for Runtime Bridging?

**The FFI Alternative:**
Traditionally, calling Rust from TypeScript requires FFI via node-gyp or napi-rs. This tightly couples the two runtimes:
- TypeScript code loads a native `.node` module (shared library)
- Rust code runs in the same process as Node.js
- Memory is shared across the JavaScript/Rust boundary
- Node.js version changes can break Rust bindings

**Problems with FFI:**
1. **Platform-Specific Compilation:** Users must have C/C++ toolchain installed
2. **Tight Version Coupling:** Rust code must be recompiled for each Node.js version
3. **No Crash Isolation:** Rust panic = Node.js crash = HTTP server down
4. **Distribution Complexity:** Can't ship pre-built binaries that work across Node.js versions

**The Splice Solution: Runtime Isolation**
```
FFI Approach (Shared Process):
┌──────────────────────────────────┐
│   Single Process                 │
│  ┌────────────┐  ┌────────────┐  │
│  │ Node.js    │  │ Rust .node │  │
│  │ V8 Runtime │──│ (FFI)      │  │  ← Shared memory, same process
│  └────────────┘  └────────────┘  │
│         │              │         │
│         └──── crash ───┘         │
│           entire process dies    │
└──────────────────────────────────┘

Splice Approach (Isolated Processes):
┌────────────────┐      ┌────────────────┐
│ Node.js        │      │ Rust Worker    │
│ V8 Runtime     │◄────►│ Tokio Runtime  │  ← Separate processes
│ (TypeScript)   │ RPC  │ (Rust)         │     Independent memory
└────────────────┘      └────────────────┘
     │ survives              │ panic
     └───────────────────────┘ isolated
```

**Benefits of Process Isolation:**
1. **Zero Compilation:** Ship pre-built Rust binaries via npm, no C++ toolchain needed
2. **Runtime Independence:** TypeScript and Rust versions decoupled
3. **Crash Isolation:** Rust panic doesn't crash Node.js (supervisor auto-restarts worker)
4. **Hot Reload:** Update Rust code without restarting the Node.js HTTP server
5. **Resource Isolation:** Separate memory spaces prevent memory leaks from crossing runtimes

**Trade-off:** ~15-25μs RPC overhead vs FFI ~100ns, but enables zero-compilation distribution and crash safety

### Why MessagePack for TypeScript↔Rust Serialization?

**The Serialization Challenge:**
TypeScript and Rust have fundamentally different type systems:
- TypeScript: Dynamic typing, JSON-oriented, prototype-based objects
- Rust: Static typing, strongly-typed structs, zero-cost abstractions

The protocol needs to bridge these efficiently while preserving type information.

**Why Not JSON?**
```typescript
// TypeScript side:
const data = {name: "Alice", age: 30, metadata: {...}}
JSON.stringify(data)  // "{"name":"Alice","age":30,...}"  ← Text encoding

// Rust side:
serde_json::from_str(json_str)  // Parse text → struct
```
- **Text overhead:** ~30% larger than binary encoding
- **Parsing cost:** Text parsing is slower than binary deserialization
- **No binary data:** Base64 encoding required for binary (e.g., file uploads)

**Why MessagePack?**
```typescript
// TypeScript side:
msgpack.encode({name: "Alice", age: 30})
// Binary: [0x82, 0xa4, "name", 0xa5, "Alice", ...]  ← 40% smaller

// Rust side:
rmp_serde::from_slice(&bytes)  // Direct binary → struct (faster)
```

**Advantages for Runtime Bridge:**
- **Binary Efficiency:** 30-50% smaller than JSON, faster over Unix sockets
- **Type Preservation:** Numbers stay numbers (not strings), booleans stay booleans
- **Binary Support:** Native Uint8Array/Bytes support (no Base64 needed)
- **Serde Compatibility:** Seamless integration with Rust's `serde` ecosystem
- **Bounded Frames:** Length-prefix prevents unbounded memory allocation

**Advantages over Protocol Buffers:**
- **No Schema Files:** TypeScript objects map directly to MessagePack (no `.proto` files)
- **Dynamic Typing:** Better for JavaScript's dynamic nature
- **Simpler Tooling:** No protoc compiler, no code generation step
- **Smaller npm Package:** Lighter distribution for frontend/backend bridge

**Trade-off:** Less compile-time schema validation than Protobuf, but:
- Runtime validation via JSON Schema
- Auto-generated TypeScript types provide compile-time safety
- Faster iteration: change Rust function signature → regenerate types → TypeScript sees changes

### Why Unix Sockets for Inter-Runtime Communication?

**The Communication Options:**
When bridging TypeScript and Rust runtimes, several IPC mechanisms are possible:

1. **TCP Sockets (localhost:port):**
   ```
   TypeScript ──TCP 127.0.0.1:9000──> Rust
   ```
   - ✗ Network stack overhead (~50-100μs latency)
   - ✗ Port conflicts with other services
   - ✗ Network-level security concerns (firewall rules, localhost hijacking)
   - ✗ Extra configuration (port selection, binding)

2. **Unix Domain Sockets (file-based):**
   ```
   TypeScript ──/tmp/splice.sock──> Rust
   ```
   - ✓ Lower latency (~5-10μs, bypasses network stack)
   - ✓ File system permissions (chmod/chown for access control)
   - ✓ Automatic cleanup (socket file removed on process exit)
   - ✓ No port conflicts (filesystem namespace)

**Advantages for Runtime Bridge:**
- **Performance:** ~10x faster than TCP localhost (critical for RPC latency)
- **Local-Only:** Socket file ensures TypeScript and Rust must be on same machine
- **File Permissions:** Standard Unix permissions control which processes can connect
- **Process Discovery:** Socket file path passed via environment variable (`ZAP_SOCKET`)

**Platform Consideration:**
- macOS/Linux: Native Unix socket support
- Windows: Requires Named Pipes (different API, not yet implemented)
- **Deployment Reality:** 95%+ of Node.js production runs on Unix (Docker, AWS, GCP, Azure Linux)

**Trade-off:** Platform-specific vs universal, but aligns with Node.js deployment reality

### Why Supervisor Pattern?

**Centralized Management:**
- **Crash Recovery:** Supervisor detects worker exit and restarts automatically
- **Health Monitoring:** Supervisor polls worker without requiring host involvement
- **Hot Reload Coordination:** Supervisor drains requests and restarts worker
- **Future Scaling:** Supervisor can manage worker pools

**Alternative Considered:** Direct host↔worker communication would be simpler but:
- No automatic restart (host has to implement)
- No centralized health monitoring
- No coordination point for hot reload

**Trade-off:** Additional process overhead (~5MB RSS) for operational simplicity

### Concurrency Limits Rationale

**Global Limit (1024):**
- **Prevents:** Memory exhaustion from unbounded request queuing
- **Calculation:** ~200 bytes per pending request → ~200KB max overhead
- **Assumption:** User functions are I/O bound, not CPU bound

**Per-Function Limit (256):**
- **Prevents:** Single hot function monopolizing all 1024 slots
- **Fairness:** Other functions get guaranteed capacity
- **Trade-off:** Lower per-function throughput for system stability

**Alternative Considered:** Unlimited queuing with backpressure would maximize throughput but:
- Risk of OOM under traffic spikes
- Harder to reason about resource usage
- Better to reject quickly than queue indefinitely

### Linkme vs Inventory: Function Discovery Across Runtimes

**The Discovery Problem:**
For the TypeScript→Rust bridge to work, the Rust worker needs to know which functions are available:

```typescript
// TypeScript wants to call:
await rpc.call("users.create", {name: "Alice"})

// But how does Rust know "users.create" exists?
```

**Inventory Approach (Doesn't Work for Pre-built Binaries):**
```rust
// In zap binary (pre-built, distributed via npm):
#[macro_use] extern crate inventory;
inventory::collect!(exported_functions);

// In user code (compiled separately):
#[export]
fn users_create() { ... }  // ✗ Not discovered! inventory fails across binaries
```

The `inventory` crate uses linker magic that only works when all code is compiled together. When ZapJS ships a pre-built binary via npm and users compile their functions separately, inventory can't discover them.

**Linkme Solution (Works Across Compilation Boundaries):**
```rust
// In splice worker runtime (links user code):
#[linkme::distributed_slice]
pub static EXPORTS: [ExportedFunction];

// In user code:
#[export]  // Macro generates:
#[linkme::distributed_slice(EXPORTS)]
static USERS_CREATE: ExportedFunction = ExportedFunction {
    name: "users.create",
    wrapper: |ctx, params| { ... },
};
```

**How Linkme Enables the Bridge:**
1. Each `#[export]` creates a static with `#[linkme::distributed_slice(EXPORTS)]`
2. **Linker** (not runtime) collects all statics into a single slice during binary linking
3. When user-server binary starts, `EXPORTS` contains all exported functions
4. Worker sends `ListExportsResult` to TypeScript via protocol
5. TypeScript codegen creates typed RPC bindings

**The Magic:**
```
User compiles: cargo build
  → Links user functions into user-server binary
  → Linkme collects EXPORTS at link time
  → Binary knows all functions

TypeScript calls: rpc.call("users.create", ...)
  → Worker looks up "users.create" in EXPORTS
  → Finds function and executes
  → Returns result to TypeScript
```

**Trade-off:** Requires static initialization (not dynamic discovery), but enables zero-compilation npm distribution with full function registry

---

## Component Interactions

### Startup Sequence

```
Step  │ Actor              │ Action
──────┼────────────────────┼───────────────────────────────────────────
  1   │ CLI                │ Runs: splice --socket /tmp/splice.sock --worker ./user-server
  2   │ Supervisor         │ Creates /tmp/splice.sock (host listener)
  3   │ Supervisor         │ Creates /tmp/worker.sock (worker listener)
  4   │ Supervisor         │ Spawns worker: ./user-server with ZAP_SOCKET=/tmp/worker.sock
  5   │ Worker             │ Connects to /tmp/worker.sock
  6   │ Worker             │ Sends Handshake (role: Worker, capabilities: 0b11)
  7   │ Supervisor         │ Sends HandshakeAck (negotiated capabilities: 0b11)
  8   │ Worker             │ Sends ListExportsResult (9 functions)
  9   │ Supervisor         │ Updates Router export cache
 10   │ Supervisor         │ Starts accepting host connections on /tmp/splice.sock
```

### Request Execution Flow

```
┌─────────────┐
│ Host (Zap)  │
│ RPC Handler │
└──────┬──────┘
       │ rpcCall("users.create", {name: "Alice"})
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ SpliceClient.invoke()                                                │
│  1. Allocate request_id = 42                                         │
│  2. Serialize params to MessagePack                                  │
│  3. Create oneshot channel for response                              │
│  4. Send Message::Invoke to supervisor socket                        │
└──────┬───────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Supervisor - Host Listener Task                                      │
│  1. Receive Message::Invoke                                          │
│  2. Call Router.invoke(function_name, params, deadline, context)     │
└──────┬───────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Router.invoke()                                                      │
│  1. Check: pending.len() < 1024? (global limit)                      │
│  2. Check: function_counts["users.create"] < 256? (per-func limit)   │
│  3. pending.insert(42, PendingRequest { result_tx, ... })            │
│  4. supervisor_tx.send(Message::Invoke { request_id: 42, ... })      │
│  5. tokio::time::timeout(30s, result_rx).await                       │
└──────┬───────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Supervisor - Bridge Task 1 (Supervisor → Worker)                     │
│  1. supervisor_rx.recv() → Message::Invoke { request_id: 42, ... }   │
│  2. worker_write.send(Message::Invoke).await                         │
└──────┬───────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Worker - Message Loop                                                │
│  1. worker_read.next() → Message::Invoke { request_id: 42, ... }     │
│  2. Create CancellationToken                                         │
│  3. in_flight.insert(42, InFlightRequest { cancel_token, ... })      │
│  4. tokio::spawn async task                                          │
└──────┬───────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Worker - Invocation Task                                             │
│  tokio::select! {                                                    │
│    result = dispatcher("users.create", params, context) => {         │
│      // Function executed successfully                               │
│      rmp_serde::to_vec(&result) → result_bytes                       │
│      response_tx.send(Message::InvokeResult {                        │
│        request_id: 42,                                               │
│        result: result_bytes,                                         │
│        duration_us: 1234                                             │
│      })                                                              │
│    }                                                                 │
│    _ = cancel_token.cancelled() => {                                 │
│      // Request was cancelled                                        │
│      response_tx.send(Message::InvokeError {                         │
│        request_id: 42, code: 2002, kind: Cancelled, ...              │
│      })                                                              │
│    }                                                                 │
│  }                                                                   │
└──────┬───────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Supervisor - Bridge Task 2 (Worker → Supervisor)                     │
│  1. worker_read.next() → Message::InvokeResult { request_id: 42 }    │
│  2. router.handle_worker_message(Message::InvokeResult)              │
└──────┬───────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Router.handle_worker_message()                                       │
│  1. pending.remove(42) → Some(PendingRequest { result_tx, ... })     │
│  2. result_tx.send(Ok(result_bytes))                                 │
│  3. function_counts["users.create"] -= 1                             │
└──────┬───────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Router.invoke() (unblocks)                                           │
│  1. result_rx receives Ok(result_bytes)                              │
│  2. Returns Ok(result_bytes) to caller                               │
└──────┬───────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ SpliceClient.invoke() (unblocks)                                     │
│  1. Deserialize result_bytes from MessagePack                        │
│  2. Returns Result<Value> to RPC handler                             │
└──────┬───────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────┐
│ RPC Handler │
│ Returns     │
│ HTTP 200    │
└─────────────┘
```

### Crash Recovery Flow

```
Time  │ Supervisor                │ Worker
──────┼───────────────────────────┼──────────────────────────────────
  0s  │ Worker healthy (Ready)    │ Processing requests
      │                           │
  5s  │ Health check poll         │ Responds: HealthStatus { healthy: true }
      │                           │
 10s  │                           │ !! PANIC: user code crashed !!
      │                           │ Process exits
      │                           │
 10s  │ Detects process exit      │ (dead)
      │ State: Ready → Failed     │
      │ restart_count = 1         │
      │                           │
 10s  │ Backoff: 0ms (first)      │
      │ Spawn new worker          │ Process starts
      │                           │
 11s  │ Worker connects           │ Sends Handshake
      │ State: Failed → Starting  │
      │                           │
 11s  │ Sends HandshakeAck        │ Sends ListExportsResult
      │ State: Starting → Ready   │
      │                           │
 16s  │ Health check poll         │ Responds: HealthStatus { healthy: true }
      │                           │
 20s  │                           │ !! PANIC again !!
      │                           │
 20s  │ restart_count = 2         │
      │ Backoff: 100ms            │
      │                           │
 20.1s│ Spawn new worker          │ Process starts
      │                           │
 ...  │ (repeat if crashes again) │
      │                           │
 45s  │ restart_count = 10        │ !! PANIC (10th time) !!
      │ Max restarts exceeded     │
      │ State: Failed → CB        │
      │ Circuit Breaker open      │
      │                           │
1m15s │ CB cooldown complete      │
      │ State: CB → Starting      │
      │ Attempt restart           │
```

### Hot Reload Flow

```
Event               │ Component         │ Action
────────────────────┼───────────────────┼────────────────────────────────────
File Change         │ DevServer Watcher │ Detects: server/lib.rs modified
                    │                   │
Build Trigger       │ DevServer         │ Runs: cargo build -p user-server
                    │                   │ Compiles new binary to target/debug/user-server
                    │                   │
Change Detection    │ ReloadManager     │ Reads binary file
                    │                   │ SHA256 hash: abc123 → def456
                    │                   │ Hash changed! Initiates reload
                    │                   │
Drain Requests      │ Router            │ Waits for in-flight requests to complete
                    │                   │ Timeout: 30s (configurable)
                    │                   │ 3 requests pending... waiting...
                    │                   │ All requests completed
                    │                   │
Graceful Shutdown   │ Supervisor        │ Sends Message::Shutdown to worker
                    │                   │ Worker cancels all in-flight
                    │                   │ Worker sends ShutdownAck
                    │                   │ Worker process exits (exit code 0)
                    │                   │
Worker Restart      │ Supervisor        │ Spawns new worker with updated binary
                    │                   │ Worker connects via /tmp/worker.sock
                    │                   │ Handshake sequence completes
                    │                   │
Export Refresh      │ Supervisor        │ Receives ListExportsResult
                    │                   │ Router updates export cache
                    │                   │ New functions: 10 (was 9)
                    │                   │
TypeScript Codegen  │ DevServer         │ runSpliceCodegen() connects to supervisor
                    │                   │ Fetches ListExports via protocol
                    │                   │ Generates packages/client/src/generated/backend.ts
                    │                   │ Generates packages/client/src/generated/server.ts
                    │                   │ Generates packages/client/src/generated/types.ts
                    │                   │
Browser Reload      │ HotReloadServer   │ Sends reload signal to connected browsers
                    │                   │ Frontend reloads with new function signatures
```

---

## Error Handling Strategy

### Error Categories

1. **Protocol Errors** (`ProtocolError`)
   - Serialization/deserialization failures
   - Frame size exceeded
   - Invalid message type
   - Protocol version mismatch

2. **Router Errors** (`RouterError`)
   - `Timeout`: Request exceeded deadline
   - `Overloaded`: Concurrency limit exceeded
   - `Cancelled`: Request explicitly cancelled
   - `WorkerUnavailable`: Worker process not connected
   - `ExecutionError(String)`: User function returned error

3. **Supervisor Errors** (`SupervisorError`)
   - `SpawnFailed`: Could not start worker process
   - `ConnectTimeout`: Worker didn't connect within timeout
   - `MaxRestartsExceeded`: Circuit breaker triggered
   - `WorkerCrashed`: Unexpected worker exit

4. **Reload Errors** (`ReloadError`)
   - `IoError`: Could not read binary file
   - `SpawnFailed`: Worker restart failed
   - `IncompatibleExports`: New exports don't match expected schema

### Error Propagation Paths

**User Function Error:**
```
User function returns Err("Invalid email")
  → Worker serializes to Message::InvokeError { code: 2000, kind: User, message: "Invalid email" }
  → Supervisor receives InvokeError
  → Router.handle_worker_message() extracts message
  → Returns RouterError::ExecutionError("Invalid email")
  → SpliceClient receives error
  → RPC handler returns error to TypeScript
  → HTTP 500 with error message
```

**Worker Crash:**
```
Worker process exits unexpectedly
  → Supervisor detects exit
  → State: Ready → Failed
  → Exponential backoff delay (100ms)
  → Spawn new worker
  → If successful: State: Failed → Starting → Ready
  → Pending requests return RouterError::WorkerUnavailable
  → RPC retries (upstream responsibility)
```

**Request Timeout:**
```
Router.invoke() deadline exceeded
  → tokio::time::timeout() returns Err(Elapsed)
  → Router sends Message::Cancel { request_id }
  → Worker triggers CancellationToken
  → Function receives cancellation (if cooperative)
  → Router returns RouterError::Timeout
  → HTTP 504 Gateway Timeout
```

**Concurrency Limit Exceeded:**
```
Router.invoke() checks pending.len() >= 1024
  → Returns RouterError::Overloaded immediately
  → No message sent to worker
  → RPC handler returns error
  → HTTP 503 Service Unavailable
```

### Recovery Strategies

**Transient Errors (Automatic Retry):**
- Worker crashes → Supervisor restarts with exponential backoff
- Network blip → Reconnect on next request
- Temporary overload → Reject request, client retries

**Permanent Errors (Circuit Breaker):**
- Worker crashes 10 times → Circuit breaker opens for 30s
- Prevents restart storm
- After cooldown, allow one restart attempt
- If successful, reset counter

**Request-Level Errors (Cleanup):**
```rust
// Router cleanup on error
fn cleanup_request(&mut self, request_id: u64, function_name: &str) {
    self.pending.remove(&request_id);
    if let Some(count) = self.function_counts.get_mut(function_name) {
        *count = count.saturating_sub(1);
    }
}
```

**System-Level Errors (Graceful Degradation):**
- If Splice unavailable, fall back to in-process execution (future enhancement)
- Drain requests before shutdown
- Preserve error messages for debugging

### Error Code Mapping

| RouterError | Protocol Code | ErrorKind | HTTP Status | Description |
|-------------|---------------|-----------|-------------|-------------|
| ExecutionError | 2000 | User | 500 | User function returned error |
| Timeout | 2001 | Timeout | 504 | Request exceeded deadline |
| Cancelled | 2002 | Cancelled | 499 | Request explicitly cancelled |
| Overloaded | 3002 | System | 503 | Concurrency limit exceeded |
| WorkerUnavailable | 3001 | System | 503 | Worker not connected |

**Error Mapping Example:**
```rust
match router.invoke(...).await {
    Ok(result) => Ok(result),
    Err(RouterError::ExecutionError(msg)) => {
        Message::InvokeError {
            code: 2000,
            kind: ErrorKind::User,
            message: msg,
            details: None,
        }
    }
    Err(RouterError::Timeout) => {
        Message::InvokeError {
            code: 2001,
            kind: ErrorKind::Timeout,
            message: "Request timeout".to_string(),
            details: None,
        }
    }
    // ... other error mappings
}
```

---

## Testing and Development

### Test Structure

**Unit Tests (145 protocol tests):**
- `packages/server/splice/src/protocol.rs`: Message encoding/decoding, frame boundaries, error cases
- `packages/server/splice/src/router.rs`: Concurrency limits, timeout handling, request correlation
- `packages/server/splice/src/supervisor.rs`: State machine transitions, backoff calculation, circuit breaker

**Integration Tests (21 E2E tests across 4 suites):**
- `tests/e2e-splice/splice-e2e.test.ts`: Basic invocation, parameters, error handling, concurrency (10 tests)
- `tests/e2e-splice/splice-crash-recovery.test.ts`: Supervisor restart logic (3 tests)
- `tests/e2e-splice/splice-context.test.ts`: RequestContext propagation (5 tests)
- `tests/e2e-splice/splice-hot-reload.test.ts`: Hot reload sequence (3 tests)

**Test Categories:**
- **Codec Tests**: Frame encoding, MessagePack roundtrips, boundary conditions
- **State Machine Tests**: WorkerState transitions, circuit breaker logic
- **Concurrency Tests**: 100+ concurrent requests, per-function limits
- **Error Recovery Tests**: Crash recovery, timeout handling, cancellation
- **Context Propagation Tests**: trace_id, headers, auth forwarding

### Running Tests

**Unit Tests:**
```bash
# All unit tests
cargo test -p splice

# Protocol tests only
cargo test -p splice --lib protocol

# Verbose output
cargo test -p splice -- --nocapture
```

**E2E Tests:**
```bash
# All Splice E2E tests
bun test tests/e2e-splice/

# Specific test suite
bun test tests/e2e-splice/splice-e2e.test.ts

# Single test
bun test tests/e2e-splice/splice-e2e.test.ts -t "should handle high request volume"
```

### Development Workflow

**Build Splice Library:**
```bash
cd packages/server/splice
cargo build
```

**Build Supervisor Binary:**
```bash
cd packages/server
cargo build --release -p splice-bin

# Binary location:
# target/release/splice  (or target/aarch64-apple-darwin/release/splice on macOS)
```

**Build Test Worker:**
```bash
cargo build -p test-server
# Binary: target/debug/test-server
```

**Manual Testing:**
```bash
# Terminal 1: Start supervisor
RUST_LOG=debug ./target/release/splice \
  --socket /tmp/test.sock \
  --worker ./target/debug/test-server

# Terminal 2: Send test message (requires test harness or nc)
# See tests/e2e-splice/utils/splice-harness.ts for example
```

**Debugging:**
```bash
# Trace-level logging
RUST_LOG=splice=trace ./target/release/splice ...

# Protocol message logging
RUST_LOG=splice::protocol=trace ...

# Specific module
RUST_LOG=splice::router=debug ...
```

### Adding New Message Types

**5-Step Process:**

1. **Add Message Variant** (`protocol.rs:200-300`)
   ```rust
   pub enum Message {
       // ... existing variants

       MyNewMessage {
           request_id: u64,
           data: String,
       },
   }
   ```

2. **Add Message Type Constant** (`protocol.rs:50-100`)
   ```rust
   pub const MSG_MY_NEW_MESSAGE: u8 = 0x14;
   ```

3. **Update `message_type()` Method** (`protocol.rs:350-450`)
   ```rust
   pub fn message_type(&self) -> u8 {
       match self {
           // ... existing matches
           Message::MyNewMessage { .. } => MSG_MY_NEW_MESSAGE,
       }
   }
   ```

4. **Add Handler** (Supervisor: `splice-bin/src/main.rs:200-250`, Worker: `splice_worker.rs:100-150`)
   ```rust
   Message::MyNewMessage { request_id, data } => {
       // Handle message
       debug!("Received MyNewMessage: request_id={}, data={}", request_id, data);
       // Send response if needed
   }
   ```

5. **Add Roundtrip Test** (`protocol.rs` test module)
   ```rust
   #[test]
   fn test_my_new_message_roundtrip() {
       let msg = Message::MyNewMessage {
           request_id: 123,
           data: "test".to_string(),
       };

       let encoded = rmp_serde::to_vec(&msg).unwrap();
       let decoded: Message = rmp_serde::from_slice(&encoded).unwrap();

       match decoded {
           Message::MyNewMessage { request_id, data } => {
               assert_eq!(request_id, 123);
               assert_eq!(data, "test");
           }
           _ => panic!("Wrong message type"),
       }
   }
   ```

### Key Test Utilities

**SpliceTestHarness** (`tests/e2e-splice/utils/splice-harness.ts`):
- Spawns supervisor and worker processes
- Manages socket lifecycle
- Provides `invokeViaRpc()` helper for E2E tests
- Handles cleanup on test completion

**Protocol Helpers** (`protocol.rs` test module):
- `encode_frame()`: Manually create protocol frames
- `decode_frame()`: Parse raw bytes into messages
- Fixtures for common message types

---

## Performance Characteristics

### Latency Profile

**Component Latency:**
- Unix socket hop: ~5-10μs (macOS/Linux)
- MessagePack serialization: ~1-2μs (typical 1KB payload)
- Router overhead: ~100ns (hash lookup + atomic ops)
- Total RPC overhead: ~15-25μs

**Comparison:**
- In-process function call: ~100ns
- Splice RPC: ~15-25μs (150-250x overhead)
- TCP localhost: ~50-100μs
- HTTP request: ~1-5ms

**Trade-off:** 150x overhead acceptable for:
- Crash isolation (user code panics don't crash server)
- Hot reload (update functions without downtime)
- npm distribution (pre-built binaries + user code)

### Throughput Limits

**Concurrency:**
- Global limit: 1024 concurrent requests
- Per-function limit: 256 concurrent requests
- Test results: 100 concurrent requests complete in <50ms

**Bottleneck Analysis:**
- Not protocol overhead (MessagePack is fast)
- Not socket throughput (Unix sockets handle 10K+ msg/s)
- Usually user function execution time

**Scaling Strategies:**
- Horizontal: Worker pool (future - Phase 4.5)
- Vertical: Increase concurrency limits
- Optimization: Cache expensive computations in worker

### Memory Characteristics

**Per-Request Overhead:**
- Pending request: ~200 bytes (oneshot channel + metadata)
- In-flight tracking: ~150 bytes (CancellationToken + HashMap entry)
- Total: ~350 bytes per concurrent request

**Maximum Memory Usage:**
- 1024 concurrent requests × 350 bytes = ~350KB
- Export cache: ~500 bytes per function (9 functions = 4.5KB)
- Total supervisor overhead: ~5MB RSS

**Worker Memory:**
- Baseline: ~5-10MB RSS (Tokio runtime + linkme registry)
- User code: Variable (depends on function logic)
- Max frame size: 100MB (configurable via `DEFAULT_MAX_FRAME_SIZE`)

### Optimization Opportunities

**Already Implemented:**
- Zero-copy with `Bytes` (avoids allocation in protocol layer)
- Atomic metrics (lock-free counters, `Ordering::Relaxed`)
- mpsc channels (efficient backpressure, bounded queues)

**Future Optimizations:**
- Connection pooling: Multiple workers per supervisor (Phase 4.5)
- Compression: Enable `CAP_COMPRESSION` for large payloads
- Batching: Group multiple small requests into single frame

---

## Debugging and Troubleshooting

### Common Issues

**"Worker not available" Error:**
- **Symptom:** `RouterError::WorkerUnavailable` when calling functions
- **Cause:** Router's `worker_tx` mpsc channel not set
- **Solution:** Check Phase 4 wiring in `splice-bin/src/main.rs:71-74`
  ```rust
  let mut router = Router::new(router_config);
  let (supervisor_tx, mut supervisor_rx) = mpsc::channel::<Message>(100);
  router.set_worker_tx(supervisor_tx);  // Must call this!
  ```

**"Connection refused" Error:**
- **Symptom:** `SpliceClient::connect()` fails
- **Cause:** Supervisor not running or wrong socket path
- **Solution:**
  1. Check supervisor is running: `ps aux | grep splice`
  2. Check socket exists: `ls -la .zap/splice.sock`
  3. Verify socket path matches config

**"Protocol version mismatch" Error:**
- **Symptom:** Handshake fails with version error
- **Cause:** Supervisor and worker built from different Splice versions
- **Solution:** Rebuild both with same source:
  ```bash
  cargo build --release -p splice-bin
  cargo build -p user-server
  ```

**"Concurrency limit exceeded" Error:**
- **Symptom:** `RouterError::Overloaded` under load
- **Cause:** More than 1024 concurrent requests or 256 per function
- **Solution:** Increase limits in `RouterConfig`:
  ```rust
  RouterConfig {
      max_concurrent_requests: 2048,
      max_concurrent_per_function: 512,
      default_timeout: Duration::from_secs(30),
  }
  ```

**"Request timeout" Error:**
- **Symptom:** `RouterError::Timeout` for slow functions
- **Cause:** Function execution exceeds 30s default timeout
- **Solution:**
  1. Increase default timeout in `RouterConfig`
  2. Pass custom `deadline_ms` in `Invoke` message
  3. Optimize function to run faster

### Debugging Tools

**Environment Variables:**
```bash
# Debug-level logging for all Splice components
RUST_LOG=debug ./target/release/splice ...

# Trace-level for protocol messages (very verbose)
RUST_LOG=splice::protocol=trace ...

# Multiple modules
RUST_LOG=splice::router=debug,splice::supervisor=info ...
```

**Socket Inspection:**
```bash
# Check socket files exist
ls -la .zap/splice.sock
ls -la /tmp/worker.sock

# Check socket permissions
stat .zap/splice.sock

# Monitor socket activity (Linux)
sudo strace -e trace=network -p $(pgrep splice)
```

**Process Monitoring:**
```bash
# Find supervisor and worker PIDs
ps aux | grep splice
ps aux | grep user-server

# Monitor restarts
tail -f /var/log/splice.log | grep "Spawning worker"

# Check worker health
kill -USR1 $(pgrep user-server)  # Triggers health check log
```

### Log Interpretation

**Success Messages:**
```
INFO splice: Worker handshake complete
  → Worker connected successfully

INFO splice: Router setup complete
  → Bidirectional bridges working

INFO splice: Received 9 exports from worker
  → Function discovery succeeded
```

**Warning Messages:**
```
WARN splice: Restart backoff: 2s
  → Worker crashed multiple times (attempt 4/10)

WARN splice::supervisor: Worker not ready, attempting restart
  → Health check failed, restarting worker
```

**Error Messages:**
```
ERROR splice::supervisor: Circuit breaker open
  → Max restarts (10) exceeded, 30s cooldown

ERROR splice::router: Worker unavailable
  → No worker connected, cannot route requests

ERROR splice::protocol: Frame too large: 150MB
  → Payload exceeds max frame size (100MB)
```

### Performance Issues

**High `active_requests` Count:**
- **Symptom:** `metrics.active_requests` stays elevated
- **Diagnosis:** Slow functions or missing cancellation checks
- **Solution:**
  ```rust
  // Add cancellation checks in long loops
  #[zap::export]
  pub async fn long_task(ctx: &Context, data: Vec<u64>) -> Result<u64> {
      for (i, chunk) in data.chunks(1000).enumerate() {
          if ctx.is_cancelled() {  // Check periodically
              return Err("Cancelled".to_string());
          }
          // Process chunk...
      }
  }
  ```

**High `restart_count`:**
- **Symptom:** Worker restarts frequently
- **Diagnosis:** User code panicking
- **Solution:**
  1. Check worker logs: `RUST_LOG=debug ./target/debug/user-server`
  2. Add error handling: Use `Result<T, String>` instead of `.unwrap()`
  3. Test functions in isolation before deploying

**"Overloaded" Errors:**
- **Symptom:** Frequent `RouterError::Overloaded`
- **Diagnosis:** Concurrency limits too low for traffic
- **Solutions:**
  1. Increase limits (see "Concurrency limit exceeded" above)
  2. Optimize slow functions to reduce active request duration
  3. Scale horizontally with worker pool (future enhancement)

---

## Future Enhancements

### Phase 3.5: Streaming Support
- **Goal:** Support `AsyncIterable<T>` return types for large responses
- **Protocol:** `StreamStart`, `StreamChunk`, `StreamEnd` messages
- **Backpressure:** Window-based flow control with `StreamAck`
- **Use Cases:** Large query results, file downloads, real-time data feeds

### Worker Pool (Scaling)
- **Goal:** Multiple worker processes per supervisor
- **Routing:** Round-robin or least-loaded worker selection
- **Concurrency:** Scale beyond 256 concurrent per function
- **High Availability:** Worker failures don't affect others

### Compression
- **Goal:** Enable `CAP_COMPRESSION` capability flag
- **Algorithms:** LZ4 (fast) or Zstd (high compression)
- **Negotiation:** Per-connection compression based on capabilities
- **Use Cases:** Large payloads (>10KB), network-constrained environments

### Observability
- **Prometheus Metrics:** Export metrics via `/metrics` endpoint
- **Distributed Tracing:** Integrate with OpenTelemetry using `trace_id`/`span_id`
- **Health Endpoint:** HTTP health check for orchestrators (Kubernetes, etc.)
- **Structured Logging:** JSON logs for centralized aggregation

### Security Enhancements
- **Socket Permissions:** Enforce file permissions on Unix sockets
- **Function Authorization:** Check `ctx.has_role()` before execution
- **Rate Limiting:** Per-user or per-function request limits
- **Audit Logging:** Log all function invocations with auth context

---

## References and Related Documentation

### Internal Documentation

- **`/Users/deepsaint/Desktop/zapjs/BINFIX.md`** - Complete Splice implementation history and Phase 4 details
- **`packages/server/src/context.rs`** - Context API for user functions (trace_id, headers, auth, cancellation)
- **`packages/server/src/registry.rs`** - Linkme integration and function dispatcher
- **`packages/server/internal/macros/src/lib.rs`** - `#[zap::export]` macro implementation

### External Resources

- **MessagePack Specification:** https://msgpack.org/
- **Linkme Crate Documentation:** https://docs.rs/linkme
- **Tokio Runtime Documentation:** https://tokio.rs/
- **Unix Socket Programming:** `man 7 unix` (Linux), `man 4 unix` (macOS)

### Code Navigation

| Component | File Path | Lines | Key Exports |
|-----------|-----------|-------|-------------|
| Protocol Definitions | `packages/server/splice/src/protocol.rs` | 1-1747 | `Message`, `SpliceCodec`, `RequestContext`, error constants |
| Supervisor Binary | `packages/server/splice-bin/src/main.rs` | 1-286 | Main supervisor loop, bidirectional bridges |
| Router Logic | `packages/server/splice/src/router.rs` | 1-261 | `Router`, `RouterConfig`, `RouterError` |
| Worker Runtime | `packages/server/src/splice_worker.rs` | 1-250 | `run()`, message loop, cancellation |
| Supervisor Logic | `packages/server/splice/src/supervisor.rs` | 1-247 | `Supervisor`, `WorkerState`, crash recovery |
| Reload Manager | `packages/server/splice/src/reload.rs` | 1-90 | `ReloadManager`, SHA256 detection |
| Metrics | `packages/server/splice/src/metrics.rs` | 1-105 | `Metrics`, atomic counters |
| Host Client | `packages/server/src/splice_client.rs` | 1-300 | `SpliceClient`, connection management |

---

**Contributors:** This README serves as the architectural reference for Splice. For usage documentation, see the main ZapJS README. For implementation details of specific features, refer to inline code comments and commit history.
