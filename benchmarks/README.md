# ZapJS Performance Benchmarks

Comprehensive benchmark suite for validating ZapJS performance claims.

## 📊 Performance Targets

| Metric | Target | Description |
|--------|--------|-------------|
| Router (static) | < 15ns | Static route lookup time |
| Router (dynamic) | < 120ns | Dynamic route with parameters |
| HTTP parse (simple) | < 200ns | Parse simple GET request |
| HTTP parse (headers) | < 400ns | Parse GET with headers |
| IPC round-trip | < 150μs | Rust ↔ TypeScript communication |
| Static route RPS | > 150k | Requests per second (static) |
| Dynamic route RPS | > 35k | Requests per second (dynamic) |
| **vs Express** | **10-100x** | Performance comparison |

## 🏗️ Directory Structure

```
benchmarks/
├── load-tests/           # wrk load testing scripts
│   ├── wrk-scripts/      # Lua scripts for wrk
│   └── configs/          # Test configurations
├── comparative/          # Framework comparison
│   ├── servers/          # Test servers (Express, Fastify, Bun, Zap)
│   └── compare.ts        # Comparison runner
├── regression/           # Regression detection
│   ├── baselines/        # Performance baselines
│   └── detect-regression.ts
├── scripts/              # Utility scripts
│   ├── install-tools.sh  # Install wrk, Rust, Bun
│   └── run-all.sh        # Complete benchmark suite
└── reports/              # Generated reports (gitignored)
```

## 🚀 Quick Start

### 1. Install Tools

```bash
cd benchmarks
./scripts/install-tools.sh
```

This installs:
- `wrk` (HTTP benchmarking)
- `cargo` (Rust toolchain)
- `bun` (JavaScript runtime)

### 2. Run All Benchmarks

```bash
# Full benchmark suite
./scripts/run-all.sh

# Quick run (shorter duration)
./scripts/run-all.sh --quick

# Skip micro-benchmarks
./scripts/run-all.sh --skip-micro

# Skip load tests
./scripts/run-all.sh --skip-load
```

### 3. View Results

- **Criterion HTML**: `open ../target/criterion/report/index.html`
- **Text report**: `benchmarks/reports/benchmark_TIMESTAMP.txt`

## 📈 Benchmark Types

### 1. Criterion Micro-Benchmarks

Located in:
- `packages/server/internal/core/benches/router_bench.rs`
- `packages/server/internal/core/benches/http_parser_bench.rs`
- `packages/server/benches/ipc_bench.rs`

Run individually:
```bash
# All benchmarks
cargo bench

# Specific benchmark
cargo bench --bench router_bench

# Quick run
cargo bench -- --quick
```

**Validates**:
- Router lookup performance (9ns static, 80ns dynamic)
- HTTP parsing speed (125ns simple GET)
- IPC serialization overhead

### 2. Load Tests (wrk)

Three test scenarios:
1. **Static routes** - `/health`, `/`, `/api/status` rotation
2. **Dynamic routes** - `/api/users/:id` with varying IDs
3. **Mixed workload** - 60% static, 40% dynamic (realistic)

Run manually:
```bash
# Start server first
cd zaptest && bun run dev

# In another terminal
cd benchmarks/load-tests
wrk -t4 -c100 -d30s -s wrk-scripts/static-route.lua http://localhost:3000/
```

**Validates**:
- Throughput (180k RPS static, 45k RPS dynamic)
- Latency distribution (p50, p90, p99)
- Real-world performance under load

### 3. Comparative Benchmarks

Compares ZapJS against:
- Express.js (baseline)
- Fastify
- Bun native HTTP

```bash
cd benchmarks/comparative

# Install Express & Fastify
npm install

# Run comparison
bun run compare.ts
```

**Validates**:
- "10-100x faster than Express" claim
- Speedup factors across different scenarios
- JSON report: `benchmarks/reports/comparison_TIMESTAMP.json`

### 4. Regression Detection

Automated CI checks for performance degradation:

```bash
cd benchmarks/regression

bun run detect-regression.ts \
  baselines/main.json \
  results/pr-123.json \
  0.10  # 10% threshold
```

**Features**:
- 10% regression threshold (configurable)
- Blocks PRs with performance degradation
- Tracks improvements

## 🔧 Configuration

### Load Test Parameters

Edit `benchmarks/load-tests/configs/localhost.json`:

```json
{
  "tests": {
    "static_route": {
      "threads": 4,
      "connections": 100,
      "duration": "30s",
      "target_rps": 180000
    }
  }
}
```

### Regression Thresholds

Edit baseline: `benchmarks/regression/baselines/main.json`

```json
{
  "router_static_lookup_ns": 9.0,
  "router_dynamic_lookup_ns": 80.0,
  "rps_static_route": 180000
}
```

## 📊 CI Integration

GitHub Actions workflow: `.github/workflows/benchmark.yml`

**Runs on**:
- Every push to main
- Every pull request
- Manual trigger (`workflow_dispatch`)
- Weekly schedule (Sundays)

**Jobs**:
1. **Criterion benchmarks** - Always runs
2. **Regression check** - PRs only, blocks on failure
3. **Load tests** - Manual trigger only
4. **Comparative** - Weekly on main

**Artifacts**:
- Criterion HTML reports (30 days)
- Benchmark JSON results (90 days)
- Load test logs (30 days)

## 🎯 Performance Tips

### Reduce Variance

```bash
# Linux: Set CPU governor to performance
sudo cpupower frequency-set --governor performance

# macOS: Disable CPU throttling
sudo pmset -a womp 0

# Disable Turbo Boost (for consistency)
# Linux: echo 1 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo
```

### Accurate Results

1. **Close background apps** - Minimize CPU contention
2. **Disable power saving** - Prevent CPU frequency scaling
3. **Run multiple times** - Criterion runs 100+ iterations automatically
4. **Use release builds** - `cargo bench` uses `--release`

### Profiling

Generate flamegraphs for hotspot analysis:

```bash
# Install flamegraph
cargo install flamegraph

# Profile router benchmarks
cargo flamegraph --bench router_bench

# View SVG
open flamegraph.svg
```

## 📖 Reading Results

### Criterion Output

```
router_static_lookup/10_routes
                        time:   [8.95 ns 9.02 ns 9.10 ns]
                        change: [-1.2% +0.3% +1.8%]
```

- **Time**: Mean ± std deviation
- **Change**: vs previous run (if available)
- **Outliers**: Flagged automatically

### wrk Output

```
Requests/sec:  182,450
Latency (avg): 0.54ms
P99 Latency:   1.23ms
```

- **RPS**: Higher is better
- **Latency**: Lower is better
- **P99**: 99th percentile (worst 1%)

## 🐛 Troubleshooting

### "wrk: command not found"

```bash
# macOS
brew install wrk

# Ubuntu/Debian
sudo apt-get install wrk

# Or use install script
./scripts/install-tools.sh
```

### "Server not running"

Load tests require a running server:

```bash
cd zaptest
bun run dev
```

### Criterion compilation errors

Ensure you have the latest Rust toolchain:

```bash
rustup update stable
```

### High variance in results

- Close background applications
- Run on a consistent machine
- Use `--quick` for faster (less accurate) runs
- Check CPU governor settings

## 📚 Additional Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [wrk Benchmarking Guide](https://github.com/wg/wrk)
- [ZapJS Performance Internals](../docs/internals/performance.md)

## 🤝 Contributing

When adding benchmarks:

1. Add to appropriate directory
2. Update baselines in `regression/baselines/`
3. Document expected performance
4. Update this README

For performance improvements:

1. Run benchmarks before changes
2. Implement optimization
3. Run benchmarks after changes
4. Compare results with regression detection
5. Update baseline if improvement is significant

## 📝 License

MIT - See [LICENSE](../LICENSE)
