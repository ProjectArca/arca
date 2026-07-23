#!/bin/bash
# Arca Multi-Layer Test Suite Runner
set +e

mkdir -p tests/logs
RAW_LOG="tests/logs/raw_latest.txt"

LAYERS=("parse" "semantic" "codegen" "runtime")
RESULTS=()
START_TOTAL=$(date +%s%N)

{
echo "========================================="
echo " Arca Multi-Layer Test Suite"
echo " Date: $(date -u)"
echo "========================================="
echo ""
} | tee "$RAW_LOG"

cargo build --release -q

echo ""

TOTAL_PASS=0; TOTAL_FAIL=0
for layer in "${LAYERS[@]}"; do
  SCRIPT="tests/run_${layer}.sh"
  if [ -f "$SCRIPT" ]; then
    {
      echo "----- Layer: $layer -----"
    } | tee -a "$RAW_LOG"
    $SCRIPT 2>&1 | tee -a "$RAW_LOG"
    RC=${PIPESTATUS[0]}
    RESULTS+=("$layer: $([ $RC -eq 0 ] && echo PASS || echo FAIL)")
    [ $RC -eq 0 ] && TOTAL_PASS=$((TOTAL_PASS + 1)) || TOTAL_FAIL=$((TOTAL_FAIL + 1))
  else
    RESULTS+=("$layer: SKIP")
  fi
  echo "" | tee -a "$RAW_LOG"
done

END_TOTAL=$(date +%s%N)
DURATION_MS=$(( (END_TOTAL - START_TOTAL) / 1000000 ))

{
echo "========================================="
echo " Layer Results:"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done
echo "========================================="
echo " Total time: ${DURATION_MS}ms"
echo " Layers: $((TOTAL_PASS + TOTAL_FAIL)) total, ${TOTAL_PASS} passed, ${TOTAL_FAIL} failed"
echo "========================================="
} | tee -a "$RAW_LOG"

echo "Raw log saved to: $RAW_LOG"
[ "$TOTAL_FAIL" -eq 0 ]
