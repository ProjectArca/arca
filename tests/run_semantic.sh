#!/bin/bash
# Semantic-layer tests: verify typechecking and borrow checking
set +e

ARCA_BIN="./target/release/arca-cli"
LOG_FILE="tests/logs/semantic_latest.txt"
RAW_FILE="tests/logs/semantic_raw_latest.txt"
PASS=0; FAIL=0; TOTAL=0

mkdir -p tests/logs
echo "Arca Semantic Test Log" > "$LOG_FILE"
echo "Timestamp: $(date -u)" >> "$LOG_FILE"

for f in tests/semantic/*.arca; do
  [ -f "$f" ] || continue
  name=$(basename "$f" .arca)
  TOTAL=$((TOTAL + 1))

  START_NS=$(date +%s%N)
  output=$("$ARCA_BIN" check "$f" 2>&1)
  rc=$?
  END_NS=$(date +%s%N)
  DURATION_MS=$(( (END_NS - START_NS) / 1000000 ))

  if [[ "$name" == *_invalid ]]; then
    if [ $rc -ne 0 ]; then
      echo "[semantic] $name  ${DURATION_MS}ms  PASS (expected error)"
      echo "$name: ${DURATION_MS}ms PASS" >> "$LOG_FILE"
      PASS=$((PASS + 1))
    else
      echo "[semantic] $name  ${DURATION_MS}ms  FAIL (expected error but passed)"
      echo "$name: ${DURATION_MS}ms FAIL" >> "$LOG_FILE"
      FAIL=$((FAIL + 1))
    fi
  else
    if [ $rc -eq 0 ]; then
      echo "[semantic] $name  ${DURATION_MS}ms  PASS"
      echo "$name: ${DURATION_MS}ms PASS" >> "$LOG_FILE"
      PASS=$((PASS + 1))
    else
      error_line=$(echo "$output" | grep -i error | head -1)
      echo "[semantic] $name  ${DURATION_MS}ms  FAIL  $error_line"
      echo "$name: ${DURATION_MS}ms FAIL" >> "$LOG_FILE"
      FAIL=$((FAIL + 1))
    fi
  fi
done

{
  echo "---"
  echo "Semantic layer: $PASS passed, $FAIL failed out of $TOTAL tests"
} | tee "$RAW_FILE"
echo "Log saved to: $LOG_FILE"
echo "Raw log saved to: $RAW_FILE"
[ "$FAIL" -eq 0 ]
