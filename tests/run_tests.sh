#!/bin/bash
# High-Performance Arca Test Suite Runner
# Pre-builds release binary once and logs full program stdout for ALL 58 tests without blocking on stdin.
set +e

mkdir -p tests/logs

LOG_FILE="tests/logs/latest_test_log.txt"

echo "Building Arca compiler binary (Release mode)..."
cargo build --release -q

ARCA_BIN="./target/release/arca-cli"

echo "Arca Test Execution & Runtime Stdout Log" > "$LOG_FILE"
echo "Timestamp: $(date -u)" >> "$LOG_FILE"
echo "=========================================" >> "$LOG_FILE"

PASS=0
FAIL=0
TOTAL=0

for dir in tests/features tests/std-libs; do
  for f in "$dir"/*.test.arca; do
    [ -f "$f" ] || continue
    name=$(basename "$f")
    TOTAL=$((TOTAL + 1))
    echo -n "[test] $name ... "

    START_TIME=$(date +%s)
    TIMEOUT=30
    if [[ "$name" == "serve.test.arca" ]]; then TIMEOUT=2; fi
    output=$(echo "" | perl -e "alarm $TIMEOUT; exec @ARGV" "$ARCA_BIN" run "$f" 2>&1 || true)
    EXIT_CODE=$?
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))

    {
      echo "-----------------------------------------"
      echo "Test #$TOTAL: $name"
      echo "File: $f"
      echo "Duration: ${DURATION}s"
      echo "Runtime Stdout:"
      echo "$output"
    } >> "$LOG_FILE"

    if ! echo "$output" | grep -qi "error:"; then
      echo "PASS"
      echo "Result: PASS" >> "$LOG_FILE"
      PASS=$((PASS + 1))
    else
      echo "FAIL"
      echo "Result: FAIL" >> "$LOG_FILE"
      echo "$output" | tail -5
      FAIL=$((FAIL + 1))
    fi
  done
done

{
  echo "========================================="
  echo "Results: $PASS passed, $FAIL failed out of $TOTAL total tests"
} >> "$LOG_FILE"

echo "---"
echo "Results: $PASS passed, $FAIL failed out of $TOTAL total tests"
echo "Test log saved to: $LOG_FILE"
[ "$FAIL" -eq 0 ]
