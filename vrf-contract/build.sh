#!/bin/bash
set -e

echo "Building vrf-contract..."

cargo near build non-reproducible-wasm

echo ""
echo "Build complete:"
echo "  target/near/vrf_contract.wasm"
