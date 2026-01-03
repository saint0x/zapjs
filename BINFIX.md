# zap-rustd: Rust Functions Runtime

Production specification for ZapJS Rust server functions.

---

## The Problem

ZapJS uses `inventory::collect!` to discover `#[zap::export]` functions at startup. This works when user code is compiled into the same binary as the zap runtime. It fails completely when the runtime is pre-built and distributed via npm—user code compiled separately cannot register into a pre-compiled binary's inventory.

```
Pre-built zap binary              User's Rust code
┌─────────────────────┐           ┌─────────────────────┐
│ inventory::collect! │     ✗     │ inventory::submit!  │
│ (frozen at build)   │ ←───────→ │ (separate compile)  │
└─────────────────────┘           └─────────────────────┘
```

**Result**: 0% of Rust server functions work for external users.

---

## The Solution

Introduce `zap-rustd`—a dedicated Rust Functions Runtime that:

- Runs as a supervised child process
- Loads and executes user Rust code
- Speaks a stable protocol with the main zap runtime
- Handles async, concurrency, cancellation, streaming, crashes, and hot reload

The main `zap` binary becomes a pure client—no FFI, no dylibs, no unsafe code.

---

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                         zap (main)                              │
│                                                                 │
│  HTTP Server    Static Files    TS Handlers    Rust Proxy      │
│       │              │               │              │          │
└───────┼──────────────┼───────────────┼──────────────┼──────────┘
        │              │               │              │
        └──────────────┴───────────────┘              │
                                                      │ Unix Socket
                                                      ▼
┌────────────────────────────────────────────────────────────────┐
│                      zap-rustd (supervisor)                     │
│                                                                 │
│  Protocol Router    Health Monitor    Hot Reload    Metrics    │
│         │                 │                │           │       │
└─────────┼─────────────────┼────────────────┼───────────┼───────┘
          │                 │                │           │
          └─────────────────┴────────────────┘           │
                                                         │ Unix Socket
                                                         ▼
┌────────────────────────────────────────────────────────────────┐
│                     user-server (worker)                        │
│                                                                 │
│  Tokio Runtime    #[export] Functions    DB/HTTP/FS Access     │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

**Process hierarchy**:
```
zap (PID 1000)
└── zap-rustd (PID 1001)
    └── user-server (PID 1002)
```

**Crash isolation**: User code crashes kill only the worker. zap-rustd restarts it. zap stays up.

---

## Protocol

### Transport

| Platform | Transport | Address |
|----------|-----------|---------|
| macOS/Linux | Unix socket | `.zap/rustd.sock` |
| Windows | Named pipe | `\\.\pipe\zap-rustd-{hash}` |

### Framing

```
┌──────────────┬──────────────┬─────────────────────────┐
│ Length (4B)  │ Type (1B)    │ Payload (msgpack)       │
│ big-endian   │              │                         │
└──────────────┴──────────────┴─────────────────────────┘
```

### Roles

Two protocol roles with distinct message sets:

**Host role** (zap → zap-rustd):
```
Send:    Handshake, ListExports, Invoke, Cancel, Shutdown, HealthCheck, StreamAck
Receive: HandshakeAck, ListExportsResult, InvokeResult, InvokeError,
         StreamStart, StreamChunk, StreamEnd, StreamError,
         CancelAck, ShutdownAck, HealthStatus, LogEvent
```

**Worker role** (user-server → zap-rustd):
```
Send:    Handshake, InvokeResult, InvokeError,
         StreamStart, StreamChunk, StreamEnd, StreamError,
         CancelAck, LogEvent
Receive: HandshakeAck, Invoke, Cancel, Shutdown, StreamAck
```

### Multiplexing

- Multiple Invoke requests in-flight concurrently on single connection
- Responses arrive in any order, correlated by `request_id: u64`
- No head-of-line blocking
- Payloads > max_frame_size rejected with `FrameTooLarge` error

### Messages

```
0x01 Handshake          Connection init (includes role: Host | Worker)
0x02 HandshakeAck       Connection accepted
0x03 Shutdown           Graceful shutdown request
0x04 ShutdownAck        Shutdown acknowledged

0x10 ListExports        Request function registry
0x11 ListExportsResult  Function registry

0x20 Invoke             Call function
0x21 InvokeResult       Function result
0x22 InvokeError        Function error

0x30 StreamStart        Begin streaming (includes initial window)
0x31 StreamChunk        Stream data
0x32 StreamEnd          End streaming
0x33 StreamError        Stream failed
0x34 StreamAck          Backpressure acknowledgment

0x40 Cancel             Cancel request
0x41 CancelAck          Cancellation signal delivered

0x50 LogEvent           Structured log
0x60 HealthCheck        Health probe
0x61 HealthStatus       Health response
```

### Handshake

```rust
struct Handshake {
    protocol_version: u32,         // 0x00010000 = v1.0
    role: Role,                    // Host | Worker
    capabilities: u32,             // Bitflags
    max_frame_size: u32,           // 100MB default
}

enum Role {
    Host = 1,
    Worker = 2,
}

// Capability flags
const CAP_STREAMING: u32    = 1 << 0;
const CAP_CANCELLATION: u32 = 1 << 1;
const CAP_COMPRESSION: u32  = 1 << 2;

struct HandshakeAck {
    protocol_version: u32,
    capabilities: u32,             // Negotiated (intersection)
    server_id: [u8; 16],           // UUID
    export_count: u32,
}
```

### Invoke

```rust
struct Invoke {
    request_id: u64,             // Compact, fast comparison
    function_name: String,
    params: Bytes,               // msgpack-encoded
    deadline_ms: u32,            // 0 = no deadline
    context: RequestContext,
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

struct InvokeResult {
    request_id: u64,
    result: Bytes,               // msgpack-encoded
    duration_us: u64,
}

struct InvokeError {
    request_id: u64,
    code: u16,
    kind: u8,
    message: String,
    details: Option<Bytes>,      // msgpack-encoded
}
```

### Error

```rust
enum ErrorKind {
    User = 1,        // Expected error from user code
    System = 2,      // Internal error
    Timeout = 3,     // Deadline exceeded
    Cancelled = 4,   // Request cancelled
}

// Error codes
1000 InvalidRequest
1001 InvalidParams
1002 FunctionNotFound
1003 Unauthorized
1004 FrameTooLarge
2000 ExecutionFailed
2001 Timeout
2002 Cancelled
2003 Panic
3000 InternalError
3001 Unavailable
3002 Overloaded
```

### Streaming

```rust
struct StreamStart {
    request_id: u64,
    window: u32,                 // Initial credit (chunks)
}

struct StreamChunk {
    request_id: u64,
    sequence: u64,
    data: Bytes,
}

struct StreamEnd {
    request_id: u64,
    total_chunks: u64,
}

struct StreamError {
    request_id: u64,
    code: u16,
    message: String,
}

struct StreamAck {
    request_id: u64,
    ack_sequence: u64,           // Ack'd up to this sequence
    window: u32,                 // Additional credit (0 = pause)
}
```

**Backpressure flow**:
1. `StreamStart` includes initial window (e.g., 16 chunks)
2. Sender sends chunks until window exhausted
3. Receiver sends `StreamAck` to grant more credit
4. Sender blocks if window is 0

### Cancellation Semantics

```
- Once zap-rustd sends InvokeError{Timeout} or InvokeError{Cancelled},
  that request_id is terminal
- Any subsequent worker output for that request_id is silently discarded
- CancelAck means "signal delivered to worker" not "task stopped"
- Cancel arriving after InvokeResult is a no-op (result already sent)
```

---

## SDK

### User writes

```rust
use zap::{export, Context, Error, Result, Stream};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use once_cell::sync::OnceCell;

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
}

// Worker owns its resources (not provided by Context)
static POOL: OnceCell<PgPool> = OnceCell::new();

pub async fn init() {
    let url = std::env::var("DATABASE_URL").unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    POOL.set(pool).unwrap();
}

#[export]
pub async fn get_user(id: i64, ctx: Context) -> Result<User> {
    let pool = POOL.get().unwrap();
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::not_found("user not found"))?;
    Ok(user)
}

#[export]
pub async fn list_users(ctx: Context) -> Result<Stream<User>> {
    let pool = POOL.get().unwrap();
    let (tx, stream) = Stream::channel();

    tokio::spawn(async move {
        let mut cursor = sqlx::query_as!(User, "SELECT * FROM users")
            .fetch(pool);

        while let Some(result) = cursor.next().await {
            if ctx.is_cancelled() {
                break;
            }
            match result {
                Ok(user) => {
                    if tx.send(user).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tx.error(Error::internal(e.to_string())).await;
                    break;
                }
            }
        }
    });

    Ok(stream)
}
```

### Macro generates

```rust
// Original function preserved
pub async fn get_user(id: i64, ctx: Context) -> Result<User> { ... }

// Dispatch wrapper
#[doc(hidden)]
pub async fn __zap_dispatch_get_user(
    params: &[u8],
    ctx: Context,
) -> std::result::Result<Vec<u8>, zap::Error> {
    #[derive(Deserialize)]
    struct Params { id: i64 }

    let p: Params = rmp_serde::from_slice(params)
        .map_err(|e| zap::Error::invalid_params(e.to_string()))?;

    let result = get_user(p.id, ctx).await?;

    rmp_serde::to_vec(&result)
        .map_err(|e| zap::Error::internal(e.to_string()))
}

// Static registry entry (no inventory crate)
#[used]
#[link_section = ".zap_exports"]
static __ZAP_EXPORT_GET_USER: zap::ExportEntry = zap::ExportEntry {
    name: "get_user",
    is_async: true,
    is_streaming: false,
    dispatch: __zap_dispatch_get_user,
    params_schema: r#"{"type":"object","properties":{"id":{"type":"integer"}},"required":["id"]}"#,
    return_schema: r#"{"$ref":"User"}"#,
};
```

No `inventory` crate. Static registry via linker section or explicit `&[ExportEntry]` array.

### SDK types

```rust
// Context is request-scoped only. No infrastructure (db, etc).
pub struct Context {
    trace_id: u64,
    span_id: u64,
    headers: Headers,
    auth: Option<Auth>,
    cancel: CancellationToken,
}

impl Context {
    pub fn trace_id(&self) -> u64;
    pub fn span_id(&self) -> u64;
    pub fn header(&self, name: &str) -> Option<&str>;
    pub fn user_id(&self) -> Option<&str>;
    pub fn has_role(&self, role: &str) -> bool;
    pub fn is_cancelled(&self) -> bool;
    pub fn cancellation_token(&self) -> CancellationToken;
}

pub struct Error {
    code: ErrorCode,
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Error {
    pub fn user(msg: impl Into<String>) -> Self;
    pub fn not_found(msg: impl Into<String>) -> Self;
    pub fn unauthorized() -> Self;
    pub fn internal(msg: impl Into<String>) -> Self;
    pub fn invalid_params(msg: impl Into<String>) -> Self;
}

impl<E: std::error::Error + Send + Sync + 'static> From<E> for Error {
    fn from(e: E) -> Self {
        Error::internal(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Stream<T> { ... }

impl<T: Serialize> Stream<T> {
    pub fn channel() -> (StreamSender<T>, Stream<T>);
}

impl<T: Serialize> StreamSender<T> {
    pub async fn send(&self, item: T) -> Result<(), Error>;
    pub async fn error(&self, err: Error);
}
```

### Generated main.rs

```rust
mod lib;

#[tokio::main]
async fn main() {
    // Worker initialization (user-defined)
    lib::init().await;

    // Run protocol loop
    zap::runtime::run().await
}
```

The `zap::runtime::run()` function:
1. Reads `ZAP_SOCKET` from environment
2. Connects to zap-rustd
3. Sends Handshake{Worker} with export registry
4. Enters message loop
5. Dispatches Invoke to registered functions
6. Handles Cancel and Shutdown

---

## Supervision

### Startup

```
1. zap starts
2. zap detects server/Cargo.toml
3. zap spawns zap-rustd --socket .zap/rustd.sock --worker ./server/target/release/server
4. zap-rustd listens on socket
5. zap-rustd spawns user-server with ZAP_SOCKET=.zap/worker.sock
6. user-server connects, sends Handshake
7. zap-rustd marks worker Ready
8. zap connects to zap-rustd
9. zap sends ListExports
10. zap caches registry
11. Ready
```

### Crash recovery

```
Worker crashes:
1. zap-rustd detects exit/EOF
2. In-flight requests get InvokeError{Panic}
3. Restart with backoff: 0ms, 100ms, 500ms, 2s, 5s
4. Max 10 attempts before circuit break
5. Circuit break: return Unavailable for 30s

zap-rustd crashes:
1. zap detects socket EOF
2. zap restarts zap-rustd
3. All state rebuilt from scratch
```

### Timeouts

```
Two-level enforcement:

1. zap-rustd:
   - Starts timer on Invoke
   - Fires: send Cancel to worker, send InvokeError{Timeout} to zap

2. user-server:
   - Wraps execution in tokio::time::timeout
   - Cooperative cancellation via CancellationToken
```

### Concurrency

```
Limits (configurable):
- max_concurrent_requests: 1024
- max_concurrent_per_function: 100

Backpressure:
- Queue depth tracked
- Over limit: InvokeError{Overloaded}
```

---

## Hot Reload

```
File change detected in server/src/**/*.rs
          │
          ▼
    cargo build --release
          │
          ▼
    Hash new binary
          │
    Different from current?
          │
          ▼ Yes
    Spawn new worker (worker-new)
          │
          ▼
    Wait for Handshake from worker-new
          │
          ▼
    Validate exports compatible
          │
          ▼
    Switch traffic: new requests → worker-new
          │
          ▼
    Drain in-flight on worker-old (max 30s)
          │
          ▼
    Send Shutdown to worker-old
          │
          ▼
    Wait ShutdownAck (5s) or SIGTERM
          │
          ▼
    If still alive after 5s, SIGKILL
          │
          ▼
    Done
```

**Zero downtime**: New requests never wait. Old requests complete or timeout.

---

## CLI

### zap dev

```typescript
if (fs.existsSync('server/Cargo.toml')) {
    // Build
    await exec('cargo build --release', { cwd: 'server' });

    // Start zap-rustd
    spawn(rustdBinary, [
        '--socket', '.zap/rustd.sock',
        '--worker', 'server/target/release/server',
        '--watch', 'server/src',
    ]);

    await waitForSocket('.zap/rustd.sock');

    // Start zap
    spawn(zapBinary, [
        '--config', 'zap.config.json',
        '--rustd-socket', '.zap/rustd.sock',
    ]);
}
```

### zap build

```typescript
if (fs.existsSync('server/Cargo.toml')) {
    await exec('cargo build --release', { cwd: 'server' });
    fs.copySync('server/target/release/server', 'dist/bin/server');
}
fs.copySync(zapBinary, 'dist/bin/zap');
fs.copySync(rustdBinary, 'dist/bin/zap-rustd');
```

### zap serve

```typescript
if (fs.existsSync('dist/bin/server')) {
    spawn('dist/bin/zap-rustd', [
        '--socket', '/tmp/zap-rustd.sock',
        '--worker', 'dist/bin/server',
    ]);
    await waitForSocket('/tmp/zap-rustd.sock');
}

spawn('dist/bin/zap', [
    '--config', 'dist/config.json',
    '--rustd-socket', '/tmp/zap-rustd.sock',
]);
```

---

## TypeScript Codegen

From ListExportsResult, generate:

```typescript
// src/generated/server.ts

import { rpc } from '@zap-js/client';

export interface User {
    id: number;
    name: string;
}

export async function getUser(id: number): Promise<User> {
    return rpc.call('get_user', { id });
}

export function listUsers(): AsyncIterable<User> {
    return rpc.stream('list_users', {});
}
```

---

## Configuration

```json
{
  "rust": {
    "enabled": true,
    "serverDir": "server",

    "runtime": {
      "timeoutMs": 30000,
      "maxConcurrency": 1024
    },

    "supervisor": {
      "maxRestarts": 10,
      "restartBackoff": [0, 100, 500, 2000, 5000],
      "healthCheckInterval": 5000,
      "drainTimeout": 30000
    }
  }
}
```

---

## File Layout

### npm package

```
@zap-js/server/
├── package.json
├── index.js
├── rust-sdk/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── context.rs
│       ├── error.rs
│       ├── stream.rs
│       └── runtime.rs
└── rust-macros/
    ├── Cargo.toml
    └── src/
        └── lib.rs
```

### Platform package

```
@zap-js/darwin-arm64/
├── package.json
└── bin/
    ├── zap
    ├── zap-rustd
    └── zap-codegen
```

### User project

```
my-app/
├── package.json
├── zap.config.json
├── src/
│   ├── routes/
│   └── generated/
│       └── server.ts
├── server/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── lib.rs
└── dist/
    └── bin/
        ├── zap
        ├── zap-rustd
        └── server
```

---

## Implementation

### Phase 1: Protocol

- [ ] Message types and codecs
- [ ] Framing layer
- [ ] Handshake flow
- [ ] Invoke/Result/Error
- [ ] Cancel
- [ ] Streaming

### Phase 2: zap-rustd

- [ ] CLI and config
- [ ] Socket server
- [ ] Worker spawn/monitor
- [ ] Crash recovery
- [ ] Timeout enforcement
- [ ] Concurrency limits
- [ ] Health checks
- [ ] Logging bridge

### Phase 3: SDK

- [ ] rust-sdk crate
- [ ] rust-macros crate
- [ ] Context type
- [ ] Error type
- [ ] Stream type
- [ ] Runtime harness

### Phase 4: Integration

- [ ] zap --rustd-socket
- [ ] RPC client in zap
- [ ] Request forwarding
- [ ] Response handling
- [ ] Stream passthrough

### Phase 5: CLI

- [ ] zap dev rust mode
- [ ] zap build rust mode
- [ ] zap serve rust mode
- [ ] zap doctor

### Phase 6: Hot Reload

- [ ] File watcher
- [ ] Zero-downtime swap
- [ ] Drain logic

### Phase 7: Codegen

- [ ] Parse ListExportsResult
- [ ] Generate TypeScript
- [ ] Generate types

---

## Security Model

The user-server runs as native code with the same privileges as the project. This is **trusted code**—no sandboxing, no capability restrictions.

If sandboxing is required in the future, a WASM backend can be added as an alternative execution mode.

---

## zap-rustd Structure

Library + binary for testability and future embedding:

```
packages/server/
├── zap-rustd/              # Library crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Public API
│       ├── protocol.rs     # Message types, codecs
│       ├── supervisor.rs   # Worker spawn, health, restart
│       ├── router.rs       # Request dispatch, timeout
│       ├── reload.rs       # Hot reload logic
│       └── metrics.rs
│
└── zap-rustd-bin/          # Binary crate (thin wrapper)
    ├── Cargo.toml
    └── src/
        └── main.rs         # CLI, calls into library
```

Benefits:
- Integration tests against library
- Embeddable in zap for future "single process mode"
- Clean separation of concerns

---

## Success Criteria

**Functional**
- External users can write `#[zap::export]` functions
- Full async Rust works (sqlx, reqwest, tokio)
- TypeScript calls Rust seamlessly
- Streaming works with backpressure
- Cancellation works with strict semantics

**Performance**
- p50 < 500μs IPC overhead (small payloads <1KB)
- p99 < 2ms IPC overhead
- Benchmark harness exists and is measured

**Reliability**
- Worker crash doesn't crash zap
- Automatic restart with backoff
- Graceful drain on shutdown
- Timeouts enforced at two levels
- Terminal request states are strict

**DX**
- `npm create zap my-app -- --with-rust` works
- `zap dev` auto-builds and hot-reloads
- Clear error messages
- `zap doctor` validates setup
