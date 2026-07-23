#!/bin/bash
# Runtime-layer tests: compile .arca sources through the full pipeline and verify output
set +e

mkdir -p tests/logs

LOG_FILE="tests/logs/runtime_latest.txt"
RAW_FILE="tests/logs/runtime_raw_latest.txt"
ARCA_BIN="./target/release/arca-cli"

{
  echo "Arca Runtime Test Log"
  echo "Timestamp: $(date -u)"
  echo "========================================="
} > "$LOG_FILE"

> "$RAW_FILE"

PASS=0; FAIL=0; TOTAL=0

for dir in tests/runtime/features tests/runtime/std-libs; do
  for f in "$dir"/*.arca; do
    [ -f "$f" ] || continue
    name=$(basename "$f")
    TOTAL=$((TOTAL + 1))

    START_NS=$(date +%s%N)
    output=$("$ARCA_BIN" run "$f" 2>&1 || true)
    END_NS=$(date +%s%N)
    DURATION_MS=$(( (END_NS - START_NS) / 1000000 ))
    DURATION_S=$(( (DURATION_MS + 500) / 1000 ))

    if ! echo "$output" | grep -qi "error:"; then
      line="[runtime] $name  ${DURATION_MS}ms  PASS"
      echo "$line"
      echo "$line" >> "$RAW_FILE"
      {
        echo "-----------------------------------------"
        echo "Test #$TOTAL: $name"
        echo "File: $f"
        echo "Duration: ${DURATION_S}s"
        echo "Runtime Stdout:"
        echo "$output"
        echo "Result: PASS"
      } >> "$LOG_FILE"
      PASS=$((PASS + 1))
    else
      error_preview=$(echo "$output" | grep -i error | head -1)
      line="[runtime] $name  ${DURATION_MS}ms  FAIL  $error_preview"
      echo "$line"
      echo "$line" >> "$RAW_FILE"
      {
        echo "-----------------------------------------"
        echo "Test #$TOTAL: $name"
        echo "File: $f"
        echo "Duration: ${DURATION_S}s"
        echo "Runtime Stdout:"
        echo "$output"
        echo "Result: FAIL"
      } >> "$LOG_FILE"
      FAIL=$((FAIL + 1))
    fi
  done
done

{
  echo "========================================="
  echo "Runtime layer: $PASS passed, $FAIL failed out of $TOTAL tests"
} | tee -a "$LOG_FILE" -a "$RAW_FILE"

echo ""
echo "Detailed log: $LOG_FILE"
echo "Raw log:      $RAW_FILE"
[ "$FAIL" -eq 0 ]
