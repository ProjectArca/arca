#!/bin/bash
# Arca Multi-Layer Test Suite Runner (delegates to `arca test`)
set +e

cargo build --release -q

./target/release/arca-cli test "$@"
