#!/bin/bash
# Complete benchmark suite runner
# Runs Criterion micro-benchmarks and wrk load tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BENCHMARK_DIR="${SCRIPT_DIR}/.."
REPORTS_DIR="${BENCHMARK_DIR}/reports"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT_FILE="${REPORTS_DIR}/benchmark_${TIMESTAMP}.txt"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  ZapJS Performance Benchmark Suite${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Create reports directory
mkdir -p "${REPORTS_DIR}"

# Start logging
exec > >(tee -a "${REPORT_FILE}") 2>&1

echo "📊 Starting benchmark suite at $(date)"
echo "📁 Report will be saved to: ${REPORT_FILE}"
echo ""

# Parse arguments
SKIP_MICRO=false
SKIP_LOAD=false
QUICK=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-micro)
            SKIP_MICRO=true
            shift
            ;;
        --skip-load)
            SKIP_LOAD=true
            shift
            ;;
        --quick)
            QUICK=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--skip-micro] [--skip-load] [--quick]"
            exit 1
            ;;
    esac
done

# ═══════════════════════════════════════════
# Step 1: Criterion Micro-Benchmarks
# ═══════════════════════════════════════════

if [ "$SKIP_MICRO" = false ]; then
    echo -e "${GREEN}━━━ Step 1: Criterion Micro-Benchmarks ━━━${NC}"
    echo ""

    cd "${PROJECT_ROOT}"

    if [ "$QUICK" = true ]; then
        echo "⚡ Running quick benchmarks..."
        cargo bench -- --quick
    else
        echo "🔬 Running full benchmarks (this may take 5-10 minutes)..."
        cargo bench
    fi

    echo ""
    echo -e "${GREEN}✅ Criterion benchmarks complete${NC}"
    echo "📊 HTML reports: ${PROJECT_ROOT}/target/criterion/"
    echo ""
else
    echo -e "${YELLOW}⏭️  Skipping Criterion micro-benchmarks${NC}"
    echo ""
fi

# ═══════════════════════════════════════════
# Step 2: Load Tests with wrk
# ═══════════════════════════════════════════

if [ "$SKIP_LOAD" = false ]; then
    echo -e "${GREEN}━━━ Step 2: Load Tests (wrk) ━━━${NC}"
    echo ""

    # Check if wrk is installed
    if ! command -v wrk &> /dev/null; then
        echo -e "${RED}❌ wrk not found. Please run: ./benchmarks/scripts/install-tools.sh${NC}"
        exit 1
    fi

    # Check if server is needed
    echo "🔍 Checking if ZapJS test server is running..."
    if ! curl -s http://localhost:3000/health > /dev/null 2>&1; then
        echo -e "${YELLOW}⚠️  No server detected on port 3000${NC}"
        echo "📝 You need to start a ZapJS test server before running load tests."
        echo ""
        echo "Example:"
        echo "  cd zaptest && bun run dev"
        echo ""
        read -p "Start server and press Enter to continue, or Ctrl+C to exit..."
    fi

    echo -e "${GREEN}✅ Server detected${NC}"
    echo ""

    # Warmup
    echo "🔥 Warming up server (1000 requests)..."
    wrk -t2 -c10 -d5s http://localhost:3000/health > /dev/null 2>&1
    sleep 2
    echo ""

    # Load test parameters
    if [ "$QUICK" = true ]; then
        DURATION="10s"
        THREADS=2
        CONNECTIONS=50
    else
        DURATION="30s"
        THREADS=4
        CONNECTIONS=100
    fi

    # Test 1: Static Routes
    echo -e "${BLUE}━━━ Test 1: Static Routes ━━━${NC}"
    echo "Target: 180,000 RPS"
    echo "Duration: ${DURATION}, Threads: ${THREADS}, Connections: ${CONNECTIONS}"
    echo ""

    wrk -t${THREADS} -c${CONNECTIONS} -d${DURATION} \
        -s "${BENCHMARK_DIR}/load-tests/wrk-scripts/static-route.lua" \
        http://localhost:3000/

    echo ""
    sleep 2

    # Test 2: Dynamic Routes
    echo -e "${BLUE}━━━ Test 2: Dynamic Routes + IPC ━━━${NC}"
    echo "Target: 45,000 RPS"
    echo "Duration: ${DURATION}, Threads: ${THREADS}, Connections: ${CONNECTIONS}"
    echo ""

    wrk -t${THREADS} -c${CONNECTIONS} -d${DURATION} \
        -s "${BENCHMARK_DIR}/load-tests/wrk-scripts/dynamic-route.lua" \
        http://localhost:3000/

    echo ""
    sleep 2

    # Test 3: Mixed Workload
    echo -e "${BLUE}━━━ Test 3: Mixed Workload ━━━${NC}"
    echo "Target: 100,000 RPS (mixed)"
    echo "Duration: ${DURATION}, Threads: ${THREADS}, Connections: ${CONNECTIONS}"
    echo ""

    wrk -t${THREADS} -c${CONNECTIONS} -d${DURATION} \
        -s "${BENCHMARK_DIR}/load-tests/wrk-scripts/mixed-workload.lua" \
        http://localhost:3000/

    echo ""
    echo -e "${GREEN}✅ Load tests complete${NC}"
    echo ""
else
    echo -e "${YELLOW}⏭️  Skipping load tests${NC}"
    echo ""
fi

# ═══════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✅ Benchmark suite complete!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "📊 Results saved to: ${REPORT_FILE}"
echo ""
echo "📁 Additional reports:"
if [ "$SKIP_MICRO" = false ]; then
    echo "  • Criterion HTML: ${PROJECT_ROOT}/target/criterion/"
fi
echo "  • Full output: ${REPORT_FILE}"
echo ""
echo "🎯 Performance Targets:"
echo "  • Router (static):  9ns    (target < 15ns)"
echo "  • Router (dynamic): 80ns   (target < 120ns)"
echo "  • HTTP parse:       125ns  (target < 200ns)"
echo "  • Static RPS:       180k   (target > 150k)"
echo "  • Dynamic RPS:      45k    (target > 35k)"
echo ""
echo "💡 View Criterion HTML reports:"
echo "   open ${PROJECT_ROOT}/target/criterion/report/index.html"
echo ""
