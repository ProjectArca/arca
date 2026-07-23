#!/bin/bash
# Arca Benchmark Suite — measures end-to-end compile+run time per test
set +e

ARCA_BIN="./target/release/arca-cli"
LOG_FILE="tests/benchmarks/latest_bench.txt"
mkdir -p tests/benchmarks

echo "Building Arca compiler binary (Release mode)..."
cargo build --release -q 2>/dev/null

echo "========================================="
echo " Arca Benchmark Suite"
echo " Date: $(date -u)"
echo "========================================="

BENCH_TARGETS=(
  "tests/runtime/features/arithmetic.arca"
  "tests/runtime/features/fib.arca"
  "tests/runtime/features/prime.arca"
  "tests/runtime/features/http.arca"
)

echo "Arca Benchmark Results" > "$LOG_FILE"
echo "Timestamp: $(date -u)" >> "$LOG_FILE"
echo "-----------------------------------------" >> "$LOG_FILE"

for target in "${BENCH_TARGETS[@]}"; do
  if [ ! -f "$target" ]; then
    echo "[bench] SKIP $(basename "$target") — not found"
    continue
  fi
  name=$(basename "$target" .arca)

  # Cold run (includes compilation)
  echo -n "[bench] $name (cold) ... "
  start=$(date +%s%N)
  echo "" | "$ARCA_BIN" run "$target" > /dev/null 2>&1
  end=$(date +%s%N)
  cold_ms=$(( (end - start) / 1000000 ))
  echo "${cold_ms}ms"

  # Warm run (cached C object)
  echo -n "[bench] $name (warm) ... "
  start=$(date +%s%N)
  echo "" | "$ARCA_BIN" run "$target" > /dev/null 2>&1
  end=$(date +%s%N)
  warm_ms=$(( (end - start) / 1000000 ))
  echo "${warm_ms}ms"

  {
    echo "$name: cold=${cold_ms}ms warm=${warm_ms}ms"
  } >> "$LOG_FILE"
done

echo "========================================="
echo "Benchmark log saved to: $LOG_FILE"
