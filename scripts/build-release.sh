#!/usr/bin/env bash
# Package the same source revision that passed the reusable verification workflows.
set -euo pipefail
version=${1:?version is required}
commit=${2:?commit is required}
test "$(git rev-parse HEAD)" = "$commit"
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-1}
export RUST_TEST_THREADS=1
mkdir -p output/tmp
chmod 700 output/tmp
export TMPDIR="$PWD/output/tmp"
library=$(bash scripts/setup-document-formats.sh)
cargo build --workspace --release --locked
size=$(stat -c %s target/release/despacho-cli)
test "$size" -le $((25 * 1024 * 1024))
npm --prefix web ci
npm --prefix web run build
python3 -B ops/deploy/bundle.py "$version" "$commit" "$library"
cd output/releases
sha256sum --check "qadra-$version-$commit.tar.gz.sha256"
