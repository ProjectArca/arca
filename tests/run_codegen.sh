#!/bin/bash
# Codegen-layer tests: verify AIR and C output generates without errors
set +e

ARCA_BIN="./target/release/arca-cli"
LOG_FILE="tests/logs/codegen_latest.txt"
RAW_FILE="tests/logs/codegen_raw_latest.txt"
PASS=0; FAIL=0; TOTAL=0

mkdir -p tests/logs
echo "Arca Codegen Test Log" > "$LOG_FILE"
echo "Timestamp: $(date -u)" >> "$LOG_FILE"

for f in tests/codegen/*.arca; do
  [ -f "$f" ] || continue
  name=$(basename "$f" .arca)
  TOTAL=$((TOTAL + 1))

  START_AIR_NS=$(date +%s%N)
  air_output=$("$ARCA_BIN" air "$f" --json 2>&1)
  air_rc=$?
  END_AIR_NS=$(date +%s%N)
  AIR_MS=$(( (END_AIR_NS - START_AIR_NS) / 1000000 ))

  START_C_NS=$(date +%s%N)
  c_output=$("$ARCA_BIN" build "$f" --backend=c 2>&1)
  c_rc=$?
  END_C_NS=$(date +%s%N)
  C_MS=$(( (END_C_NS - START_C_NS) / 1000000 ))

  errors=""
  [ $air_rc -ne 0 ] && errors=" AIR_FAIL"
  [ $c_rc -ne 0 ] && errors="${errors} C_FAIL"

  if [ -z "$errors" ]; then
    echo "[codegen] $name  AIR=${AIR_MS}ms C=${C_MS}ms  PASS"
    echo "$name: AIR=${AIR_MS}ms C=${C_MS}ms PASS" >> "$LOG_FILE"
    PASS=$((PASS + 1))
  else
    echo "[codegen] $name  AIR=${AIR_MS}ms C=${C_MS}ms  FAIL$errors"
    echo "$name: AIR=${AIR_MS}ms C=${C_MS}ms FAIL$errors" >> "$LOG_FILE"
    FAIL=$((FAIL + 1))
  fi
done

{
  echo "---"
  echo "Codegen layer: $PASS passed, $FAIL failed out of $TOTAL tests"
} | tee "$RAW_FILE"
echo "Log saved to: $LOG_FILE"
echo "Raw log saved to: $RAW_FILE"
[ "$FAIL" -eq 0 ]
