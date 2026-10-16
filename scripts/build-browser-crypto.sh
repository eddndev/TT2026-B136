#!/usr/bin/env bash
# Build and bind the local cryptographic module from the locked Rust sources.
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-1}
export RUST_TEST_THREADS=1
mkdir -p output/tmp
chmod 700 output/tmp
export TMPDIR="$PWD/output/tmp"
command -v clang >/dev/null
rustup target add wasm32-unknown-unknown
binding=$(python3 -B scripts/browser-crypto-toolchain.py)
cargo build --locked --release -p browser-crypto --target wasm32-unknown-unknown
"$binding" --target web --no-typescript \
  --out-dir web/apps/qadra/src/generated/browser-crypto \
  "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/browser_crypto.wasm"
node scripts/browser-crypto-artifact.mjs --write
