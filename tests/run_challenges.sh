#!/bin/bash
# Challenge test runner — runs examples/challenges/*.arca with timing + expected output check
set +e

ARCA_BIN="./target/release/arca-cli"
LOG_FILE="tests/logs/challenge_latest.txt"
RAW_FILE="tests/logs/challenge_raw_latest.txt"
PASS=0; FAIL=0; TOTAL=0

mkdir -p tests/logs

echo "Arca Challenge Test Log" > "$LOG_FILE"
echo "Timestamp: $(date -u)" >> "$LOG_FILE"
echo "=========================================" > "$RAW_FILE"
echo " Arca Challenges" >> "$RAW_FILE"
echo "=========================================" >> "$RAW_FILE"

for f in examples/challenges/*.arca; do
  [ -f "$f" ] || continue
  name=$(basename "$f" .arca)
  TOTAL=$((TOTAL + 1))

  START_NS=$(date +%s%N)
  output=$(echo "" | perl -e 'alarm 15; exec @ARGV' "$ARCA_BIN" run "$f" 2>&1 || true)
  END_NS=$(date +%s%N)
  DURATION_MS=$(( (END_NS - START_NS) / 1000000 ))

  # Expected output patterns per challenge
  case "$name" in
    todo_cli)
      expected="\[ \] Learn Arca"
      ;;
    statistics)
      expected="Min: 1"
      ;;
    expression_eval)
      expected="^11$"
      ;;
    *)
      expected=""
      ;;
  esac

  if echo "$output" | grep -qi "error:"; then
    status="FAIL"
    echo "[challenge] $name  ${DURATION_MS}ms  FAIL (compile/runtime error)" >> "$RAW_FILE"
    echo "$name: ${DURATION_MS}ms FAIL" >> "$LOG_FILE"
    echo "$output" | grep -i error | head -3 >> "$LOG_FILE"
    FAIL=$((FAIL + 1))
  elif [ -n "$expected" ] && ! echo "$output" | grep -q "$expected"; then
    status="FAIL"
    echo "[challenge] $name  ${DURATION_MS}ms  FAIL (output mismatch: expected '$expected')" >> "$RAW_FILE"
    echo "$name: ${DURATION_MS}ms FAIL" >> "$LOG_FILE"
    echo "Expected: $expected" >> "$LOG_FILE"
    echo "Got:" >> "$LOG_FILE"
    echo "$output" >> "$LOG_FILE"
    FAIL=$((FAIL + 1))
  else
    status="PASS"
    echo "[challenge] $name  ${DURATION_MS}ms  PASS" >> "$RAW_FILE"
    echo "$name: ${DURATION_MS}ms PASS" >> "$LOG_FILE"
    PASS=$((PASS + 1))
  fi

  echo "[challenge] $name  ${DURATION_MS}ms  $status"
done

{
  echo "---"
  echo "Challenges: $PASS passed, $FAIL failed out of $TOTAL tests"
} | tee -a "$RAW_FILE"
echo "Log saved to: $LOG_FILE"
echo "Raw log saved to: $RAW_FILE"
[ "$FAIL" -eq 0 ]
