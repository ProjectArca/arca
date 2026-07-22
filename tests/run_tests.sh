#!/bin/bash
# Arca Language Feature Tests
# Compiles every *.test.arca file and checks for SUCCESS
set -e

PASS=0
FAIL=0

for f in tests/features/*.test.arca; do
  name=$(basename "$f")
  echo -n "[test] $name ... "
  output=$(cargo run -q -- build "$f" 2>&1)
  if echo "$output" | grep -q "Build status: SUCCESS"; then
    echo "PASS"
    PASS=$((PASS + 1))
  else
    echo "FAIL"
    echo "$output" | tail -5
    FAIL=$((FAIL + 1))
  fi
done

echo "---"
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
