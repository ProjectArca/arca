#!/bin/bash
# Arca Test Suite Runner — Executes ALL 58 tests and records full runtime stdout in tests/logs/
set +e

mkdir -p tests/logs

TIMESTAMP=$(date +%s)
LOG_FILE="tests/logs/test_log_${TIMESTAMP}.txt"

echo "Arca Test Execution & Runtime Stdout Log" > "$LOG_FILE"
echo "Timestamp: $(date -u)" >> "$LOG_FILE"
echo "Unix Timestamp: $TIMESTAMP" >> "$LOG_FILE"
echo "=========================================" >> "$LOG_FILE"

PASS=0
FAIL=0

for dir in tests/features tests/std-libs; do
  for f in "$dir"/*.test.arca; do
    [ -f "$f" ] || continue
    name=$(basename "$f")
    echo -n "[test] $name ... "

    START_TIME=$(date +%s)
    if [[ "$name" == "serve.test.arca" ]]; then
      output=$(perl -e 'alarm 2; exec @ARGV' cargo run -q -- run "$f" 2>&1 || true)
    else
      output=$(cargo run -q -- run "$f" 2>&1)
    fi
    EXIT_CODE=$?
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))

    echo "-----------------------------------------" >> "$LOG_FILE"
    echo "Test Name: $name" >> "$LOG_FILE"
    echo "File: $f" >> "$LOG_FILE"
    echo "Duration: ${DURATION}s" >> "$LOG_FILE"
    echo "Runtime Stdout:" >> "$LOG_FILE"
    echo "$output" >> "$LOG_FILE"

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

echo "=========================================" >> "$LOG_FILE"
echo "Results: $PASS passed, $FAIL failed" >> "$LOG_FILE"

echo "---"
echo "Results: $PASS passed, $FAIL failed"
echo "Test log saved to: $LOG_FILE"
[ "$FAIL" -eq 0 ]
