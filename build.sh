#!/bin/bash
set -e

echo "Building vrf-example for wasm32-wasip2..."

rustup target add wasm32-wasip2 2>/dev/null || true

cargo build --target wasm32-wasip2 --release

echo ""
echo "Build complete:"
echo "  target/wasm32-wasip2/release/vrf-example.wasm"
echo ""
ls -lh target/wasm32-wasip2/release/vrf-example.wasm
