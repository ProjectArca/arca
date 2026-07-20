#!/usr/bin/env bash
set -euo pipefail

BENCH_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$BENCH_DIR/.." && pwd)"
RESULTS_DIR="$BENCH_DIR/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULT_FILE="$RESULTS_DIR/benchmark_$TIMESTAMP.txt"

mkdir -p "$RESULTS_DIR"

say() { echo "[bench] $*"; }
header() { echo; echo "===== $* ====="; }

ARCA_CLI="$ROOT_DIR/target/debug/arca-cli"

run_arca() {
  local name=$1 src=$2
  say "Running Arca $name..."
  local start=$(date +%s%N)
  $ARCA_CLI run "$src" 2>&1 | grep -v "\[arca\]\|warning\|note:\|dead_code\|^$\|^  $" | tail -5
  local end=$(date +%s%N)
  local elapsed=$(( (end - start) / 1000000 ))
  echo "  (Arca runtime: ${elapsed}ms)"
}

run_rust() {
  local name=$1 src=$2
  say "Compiling Rust $name..."
  rustc -C opt-level=3 -C target-cpu=native -o "/tmp/rs_$name" "$src"
  say "Running Rust $name..."
  /tmp/rs_$name
}

run_go() {
  local name=$1 src=$2
  say "Running Go $name..."
  go run "$src"
}

run_bun() {
  local name=$1 src=$2
  say "Running Bun $name..."
  bun run "$src"
}

run_web_bench() {
  local lang=$1 bin=$2

  echo "--- $lang ---" | tee -a "$RESULT_FILE"
  $bin &
  SERVER_PID=$!
  sleep 2

  if ! curl -sf http://localhost:3000 > /dev/null 2>&1; then
    echo "  Server failed to start" | tee -a "$RESULT_FILE"
    kill $SERVER_PID 2>/dev/null || true
    return 1
  fi

  for i in $(seq 1 100); do curl -sf http://localhost:3000 > /dev/null 2>&1; done
  bun run "$BENCH_DIR/web_api/bench_client.js" "http://localhost:3000" 50000 200 2>&1 | tee -a "$RESULT_FILE"

  kill $SERVER_PID 2>/dev/null || true
  wait $SERVER_PID 2>/dev/null || true
  echo "" | tee -a "$RESULT_FILE"
}

# ===== WEB API BENCHMARKS (FIRST) =====
header "WEB API BENCHMARKS" | tee -a "$RESULT_FILE"

# Kill leftover port 3000
lsof -ti:3000 2>/dev/null | xargs kill -9 2>/dev/null || true
sleep 1

# 1. Arca Web Server
rm -f /tmp/arca_web_server
cd "$ROOT_DIR"
$ARCA_CLI build "$BENCH_DIR/web_api/server.arca" > /dev/null 2>&1
cc -o /tmp/arca_web_server "$ROOT_DIR/build/output.c" 2>/dev/null
if [ -x /tmp/arca_web_server ]; then
  run_web_bench "Arca Web Server" "/tmp/arca_web_server" || true
else
  echo "--- Arca Web Server ---" | tee -a "$RESULT_FILE"
  echo "  Build failed" | tee -a "$RESULT_FILE"
fi

# 2. Rust Web Server
rustc -C opt-level=3 -C target-cpu=native -o /tmp/rs_web_server "$BENCH_DIR/web_api/server.rs" 2>/dev/null
run_web_bench "Rust (std::net)" "/tmp/rs_web_server" || true

# 3. Go Web Server
rm -rf /tmp/web_bench_go
mkdir -p /tmp/web_bench_go
cp "$BENCH_DIR/web_api/server.go" /tmp/web_bench_go/main.go
cp "$BENCH_DIR/web_api/go.mod" /tmp/web_bench_go/go.mod
(cd /tmp/web_bench_go && go build -o server .) 2>/dev/null
run_web_bench "Go (net/http)" "/tmp/web_bench_go/server" || true

# 4. Bun Web Server
run_web_bench "Bun (Bun.serve)" "bun run $BENCH_DIR/web_api/server.js" || true

# Kill any leftover server
lsof -ti:3000 2>/dev/null | xargs kill -9 2>/dev/null || true

# ===== ALGORITHM BENCHMARKS =====
header "ALGORITHM BENCHMARKS" | tee -a "$RESULT_FILE"

for algo in fib prime sort; do
  header "Benchmark: $algo" | tee -a "$RESULT_FILE"

  echo "--- Arca $algo ---" | tee -a "$RESULT_FILE"
  if run_arca "$algo" "$BENCH_DIR/algorithm/${algo}.arca" >> "$RESULT_FILE" 2>&1; then
    echo "OK" >> "$RESULT_FILE"
  else
    echo "FAILED" >> "$RESULT_FILE"
  fi
  echo "" >> "$RESULT_FILE"

  echo "--- Rust $algo ---" | tee -a "$RESULT_FILE"
  if run_rust "$algo" "$BENCH_DIR/algorithm/${algo}.rs" >> "$RESULT_FILE" 2>&1; then
    echo "OK" >> "$RESULT_FILE"
  else
    echo "FAILED" >> "$RESULT_FILE"
  fi

  echo "--- Go $algo ---" | tee -a "$RESULT_FILE"
  if run_go "$algo" "$BENCH_DIR/algorithm/${algo}.go" >> "$RESULT_FILE" 2>&1; then
    echo "OK" >> "$RESULT_FILE"
  else
    echo "FAILED" >> "$RESULT_FILE"
  fi

  echo "--- Bun $algo ---" | tee -a "$RESULT_FILE"
  if run_bun "$algo" "$BENCH_DIR/algorithm/${algo}.js" >> "$RESULT_FILE" 2>&1; then
    echo "OK" >> "$RESULT_FILE"
  else
    echo "FAILED" >> "$RESULT_FILE"
  fi
  echo "" >> "$RESULT_FILE"
done

# ===== SUMMARY =====
header "SUMMARY" | tee -a "$RESULT_FILE"
grep -E '(^(Rust|Go|Bun|Arca)|^RPS|^  Duration|FAILED|OK|fib\(|primes under|sorted|=== WEB)' "$RESULT_FILE" || true

say "Results saved to $RESULT_FILE"
echo
cat "$RESULT_FILE"
