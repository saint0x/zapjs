# ZapJS Bundle Size Report

## Summary

ZapJS is organized into two main packages with clean separation between client and server code.

## Package Sizes

### @zap-js/client
- **Total Size**: 1.5MB (including source, compiled JS, and internal tools)
- **JavaScript Files**: ~250KB (compiled)
- **Key Components**:
  - Router: 16KB
  - Runtime (including IPC client): 24KB
  - Error boundaries: 12KB
  - Logger: 8KB
  - Middleware: <4KB

### @zap-js/server  
- **Total Package Size**: 472KB
- **JavaScript exports**: 16KB (RPC/IPC wrappers)
- **Rust Binary**: 3.3MB (release build, includes all optimizations)
- **Rust Libraries**:
  - Core HTTP primitives: 917KB
  - Server implementation: 5.2MB
  - Code generation: 894KB

## Production Bundle Analysis

### Client-Side (What users ship)
When building a ZapJS app, users only ship:
- Router + React components: ~20-30KB (minified)
- Runtime utilities: ~15KB (minified)
- **Total client runtime**: ~35-45KB minified + gzipped

### Server-Side
- Single binary: 3.3MB (includes everything)
- No Node.js dependencies required
- All Rust code is compiled with:
  - LTO (Link Time Optimization)
  - Single codegen unit
  - Maximum optimization level

## Comparison to Other Frameworks

| Framework | Client Bundle | Server Binary |
|-----------|--------------|---------------|
| ZapJS | ~40KB | 3.3MB (Rust) |
| Next.js | ~90KB | N/A (Node.js) |
| Remix | ~100KB | N/A (Node.js) |
| SvelteKit | ~50KB | N/A (Node.js) |

## Optimization Features

1. **Zero runtime dependencies** in production
2. **Tree-shakeable** client exports
3. **SIMD-optimized** HTTP parsing (Rust)
4. **Zero-allocation** routing (9ns lookup time)
5. **Single binary** deployment for server

## Build Profiles

Current release profile optimizations:
```toml
[profile.release]
lto = "fat"
codegen-units = 1
panic = "abort"
opt-level = 3
```

## Recommendations

1. The client bundle is already quite small at ~40KB
2. Server binary could be reduced by:
   - Stripping debug symbols: `strip target/release/zap`
   - Using `opt-level = "z"` for size optimization
   - Conditional compilation to exclude unused features