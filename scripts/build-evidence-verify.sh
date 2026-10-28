#!/usr/bin/env bash
# Build and bind the evidence validation module from the locked Rust sources.
# CARGO may name a cargo command to use instead of the one on PATH.
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-1}
export RUST_TEST_THREADS=1
mkdir -p output/tmp
chmod 700 output/tmp
export TMPDIR="$PWD/output/tmp"
rustup target add wasm32-unknown-unknown
binding=$(python3 -B scripts/browser-crypto-toolchain.py)
"${CARGO:-cargo}" build --locked --release -p evidence-verify-wasm --target wasm32-unknown-unknown
"$binding" --target web --no-typescript \
  --out-dir web/apps/qadra/src/generated/evidence-verify \
  "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/evidence_verify_wasm.wasm"
node scripts/evidence-verify-artifact.mjs --write
