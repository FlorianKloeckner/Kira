#!/usr/bin/env sh
# Compiles ../Kira (unmodified) to WebAssembly and copies it into site/.
# One-time setup:  rustup target add wasm32-wasip1
set -e
cd "$(dirname "$0")"
cargo build --release --target wasm32-wasip1 --manifest-path engine/Cargo.toml
cp engine/target/wasm32-wasip1/release/kira.wasm site/kira.wasm
echo "Built site/kira.wasm"
