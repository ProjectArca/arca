#!/bin/bash
# Arca Multi-Layer Test Suite Runner (delegates to `arca test`)
set +e

mkdir -p tests/logs
RAW_LOG="tests/logs/raw_latest.txt"

cargo build --release -q

./target/release/arca-cli test "$@" 2>&1 | tee "$RAW_LOG"
