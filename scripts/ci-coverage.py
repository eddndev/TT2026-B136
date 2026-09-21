"""Reuse compatible builds while collecting coverage only from this execution."""
import hashlib
import os
from pathlib import Path
import subprocess


def cache_key(root, environment, toolchain):
    digest = hashlib.sha256(toolchain.encode())
    manifests = sorted(root.glob("crates/*/Cargo.toml"))
    for path in [root / "Cargo.toml", root / "Cargo.lock", *manifests,
                 *sorted((root / ".cargo").glob("*")), Path(__file__)]:
        if path.is_file():
            digest.update(path.read_bytes())
    for key, value in sorted(environment.items()):
        if key.startswith(("CARGO_PROFILE_", "CARGO_FEATURE_")) or key in {
            "RUSTFLAGS", "RUSTDOCFLAGS", "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_ENCODED_RUSTDOCFLAGS", "CARGO_BUILD_TARGET", "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER", "CARGO_INCREMENTAL",
        }:
            digest.update(f"{key}={value}\n".encode())
    return digest.hexdigest()[:24]


def clear_measurements(directory, report):
    report.unlink(missing_ok=True)
    for pattern in ("*.profraw", "*.profdata", "*-profraw-list"):
        for path in directory.glob(pattern):
            path.unlink()


def main():
    root = Path.cwd()
    version = subprocess.check_output(["cargo", "llvm-cov", "--version"], text=True)
    toolchain = version + subprocess.check_output(["rustc", "-Vv"], text=True)
    toolchain += subprocess.check_output(["cargo", "-V"], text=True)
    target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
    directory = target.parent / "coverage" / cache_key(root, os.environ, toolchain)
    directory.mkdir(parents=True, exist_ok=True)
    report = root / "coverage.json"
    clear_measurements(directory, report)
    environment = {**os.environ, "CARGO_LLVM_COV_TARGET_DIR": str(directory)}
    print(f"Coverage build cache: {directory}", flush=True)
    subprocess.run([
        "cargo", "llvm-cov", "--workspace", "--locked", "--no-clean",
        "--json", "--summary-only", "--output-path", str(report),
    ], env=environment, check=True)
    if not list(directory.glob("*.profraw")) or not report.is_file():
        raise RuntimeError("coverage requires fresh profiles and a new report")


if __name__ == "__main__":
    main()
