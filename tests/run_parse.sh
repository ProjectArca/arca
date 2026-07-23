#!/bin/bash
# Parse-layer tests: verify AST output matches expected snapshots
set +e

ARCA_BIN="./target/release/arca-cli"
SNAP_DIR="tests/snapshots/parse"
LOG_FILE="tests/logs/parse_latest.txt"
RAW_FILE="tests/logs/parse_raw_latest.txt"
PASS=0; FAIL=0; TOTAL=0

mkdir -p "$SNAP_DIR" tests/logs

echo "Arca Parse Test Log" > "$LOG_FILE"
echo "Timestamp: $(date -u)" >> "$LOG_FILE"

for f in tests/parse/*.arca; do
  [ -f "$f" ] || continue
  name=$(basename "$f" .arca)
  TOTAL=$((TOTAL + 1))

  START_NS=$(date +%s%N)
  output=$("$ARCA_BIN" ast "$f" 2>&1)
  END_NS=$(date +%s%N)
  DURATION_MS=$(( (END_NS - START_NS) / 1000000 ))

  snap="$SNAP_DIR/$name.snap"
  if [ -f "$snap" ]; then
    expected=$(cat "$snap")
    if [ "$output" = "$expected" ]; then
      echo "[parse] $name  ${DURATION_MS}ms  PASS"
      echo "$name: ${DURATION_MS}ms PASS" >> "$LOG_FILE"
      PASS=$((PASS + 1))
    else
      echo "[parse] $name  ${DURATION_MS}ms  FAIL (snapshot mismatch)"
      echo "$name: ${DURATION_MS}ms FAIL" >> "$LOG_FILE"
      FAIL=$((FAIL + 1))
    fi
  else
    echo "[parse] $name  ${DURATION_MS}ms  NEW (snapshot created)"
    echo "$name: ${DURATION_MS}ms NEW" >> "$LOG_FILE"
    echo "$output" > "$snap"
    PASS=$((PASS + 1))
  fi
done

{
  echo "---"
  echo "Parse layer: $PASS passed, $FAIL failed out of $TOTAL tests"
} | tee "$RAW_FILE"
echo "Log saved to: $LOG_FILE"
echo "Raw log saved to: $RAW_FILE"
[ "$FAIL" -eq 0 ]
