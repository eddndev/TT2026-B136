# Qadra web workspace

Install dependencies once from this directory with `npm ci`. The root lockfile
keeps the applications and shared UI on the same dependency versions.

- [apps/qadra](apps/qadra/README.md): existing office portal and its tests.
- [apps/ca](apps/ca/README.md): location of the separate CA operator portal.
- [packages/ui](packages/ui/README.md): shared visual components and brand assets.

The existing root commands `npm run dev`, `npm test`, `npm run test:e2e`,
`npm run build` and `npm run preview` delegate to the Qadra application.
For example, `npm run dev -- --port 4321` forwards the port to that application.
Formatting commands cover Qadra and the shared UI.

Before starting or building Qadra, prepare its local Rust/WASM module once
from the repository root with `bash scripts/build-browser-crypto.sh`.
This requires Linux x86_64, Rust/rustup, clang, Python 3 and Node. The script
installs the wasm32 target and downloads the checksum-pinned wasm-bindgen
binary into ignored `output/browser-toolchain`; it does not install system
packages. Generated JS/WASM stays under `apps/qadra/src/generated/` and is
bundled into Qadra. Rebuild it after changes to its Rust sources or lockfile;
`predev` and `prebuild` reject a missing/stale artifact. The CA portal does
not load this module. See [local cryptography](../docs/browser-local-cryptography.md).

The evidence validator runs a second module, built from the repository root
with `bash scripts/build-evidence-verify.sh` (same requirements, without
clang). Its generated files go to `apps/qadra/src/generated/evidence-verify/`
and `predev` and `prebuild` also reject them when missing or stale, including
after changes to `crates/evidence-verify`. The validator trusts only the roots
pinned in `apps/qadra/src/lib/evidence/pinned-roots.json`. A build for another
environment sets `QADRA_PINNED_ROOTS` to the absolute path of a replacement
file with the same format; `astro.config.mjs` validates it and fails the build
when it is invalid. `scripts/web-demo.sh` pins its disposable CA this way. See
[ADR 0085](../docs/adr/0085-in-browser-evidence-validation.md).

The validator panel sits in the Evidencia tab of the selected document
version. It reads a local ZIP of at most 26 MiB, asks
`GET /api/v1/evidence/trust` for the trust the server currently admits and
offers the result as a JSON report; the file itself is never sent. Mocked
browser tests replace the worker with `tests/browser/evidence-worker-stub.js`,
while `tests/browser/evidence-validation-wasm.spec.mjs` runs the real module.

Qadra builds to `apps/qadra/dist`. Release packaging still installs those files
under `web/`; the source layout does not relocate production data or services.
Real backend browser acceptance starts at repository root with
`bash scripts/web-demo.sh`. Browser results are under `apps/qadra/test-results*`.

The former sample is archived at
[archive/frontend-prototype](../archive/frontend-prototype/README.md), outside
this workspace. The [documentation index](../docs/README.md) separates current
contracts, operations and historical evidence.
