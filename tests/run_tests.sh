#!/bin/bash
# High-Performance Arca Test Suite Runner
# Pre-builds release binary once and logs full program stdout for ALL 58 tests.
set +e

mkdir -p tests/logs

TIMESTAMP=$(date +%s)
LOG_FILE="tests/logs/test_log_${TIMESTAMP}.txt"
LATEST_LOG="tests/logs/latest_test_log.txt"

echo "Building Arca compiler binary (Release mode)..."
cargo build --release -q

ARCA_BIN="./target/release/arca-cli"

echo "Arca Test Execution & Runtime Stdout Log" > "$LOG_FILE"
echo "Timestamp: $(date -u)" >> "$LOG_FILE"
echo "Unix Timestamp: $TIMESTAMP" >> "$LOG_FILE"
echo "=========================================" >> "$LOG_FILE"

cp "$LOG_FILE" "$LATEST_LOG"

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
    if [[ "$name" == "serve.test.arca" ]]; then
      output=$(perl -e 'alarm 2; exec @ARGV' "$ARCA_BIN" run "$f" 2>&1 || true)
    else
      output=$("$ARCA_BIN" run "$f" 2>&1)
    fi
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

    {
      echo "-----------------------------------------"
      echo "Test #$TOTAL: $name"
      echo "File: $f"
      echo "Duration: ${DURATION}s"
      echo "Runtime Stdout:"
      echo "$output"
    } >> "$LATEST_LOG"

    if ! echo "$output" | grep -qi "error:"; then
      echo "PASS"
      echo "Result: PASS" >> "$LOG_FILE"
      echo "Result: PASS" >> "$LATEST_LOG"
      PASS=$((PASS + 1))
    else
      echo "FAIL"
      echo "Result: FAIL" >> "$LOG_FILE"
      echo "Result: FAIL" >> "$LATEST_LOG"
      echo "$output" | tail -5
      FAIL=$((FAIL + 1))
    fi
  done
done

{
  echo "========================================="
  echo "Results: $PASS passed, $FAIL failed out of $TOTAL total tests"
} >> "$LOG_FILE"

{
  echo "========================================="
  echo "Results: $PASS passed, $FAIL failed out of $TOTAL total tests"
} >> "$LATEST_LOG"

echo "---"
echo "Results: $PASS passed, $FAIL failed out of $TOTAL total tests"
echo "Timestamped log saved to: $LOG_FILE"
echo "Latest test log saved to: $LATEST_LOG"
[ "$FAIL" -eq 0 ]
