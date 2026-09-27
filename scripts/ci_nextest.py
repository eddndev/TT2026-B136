"""Compile once and schedule individual tests against independent backends."""

import argparse
import os
from pathlib import Path
import shlex
import shutil
import subprocess

from ci_test_shard import coverage_helpers


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--slots", type=int, choices=range(1, 17), default=4)
    parser.add_argument("targets", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    root = Path.cwd()
    helpers = coverage_helpers(root)
    toolchain = "".join(subprocess.check_output(command, text=True) for command in (
        ["cargo", "llvm-cov", "--version"], ["cargo", "nextest", "--version"],
        ["rustc", "-Vv"], ["cargo", "-V"],
    ))
    target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
    directory = target.parent / "coverage" / helpers.cache_key(root, os.environ, toolchain)
    directory.mkdir(parents=True, exist_ok=True)
    report = root / "coverage-shard-1.info"
    helpers.clear_measurements(directory, report)
    junit = root / "output" / "nextest" / "ci" / "junit.xml"
    published_junit = root / "output" / "ci-nextest-junit.xml"
    junit.unlink(missing_ok=True)
    published_junit.unlink(missing_ok=True)
    postgres = os.environ["IDENTITY_TEST_DATABASE_URL"]
    for slot in range(args.slots):
        for kind in ("identity", "case", "document"):
            subprocess.run([
                "psql", postgres, "-X", "-v", "ON_ERROR_STOP=1", "-q", "-c",
                f"CREATE DATABASE ci_{kind}_{slot}",
            ], check=True)
    environment = {
        **os.environ,
        "CARGO_LLVM_COV_TARGET_DIR": str(directory),
        "TT_CI_TEST_SLOTS": str(args.slots),
        "TT_CI_POSTGRES_BASE": postgres,
        "TT_CI_REDIS_BASE": os.environ["IDENTITY_TEST_REDIS_URL"],
        "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER": shlex.join([
            "python3", str(root / "scripts" / "ci_test_slot.py"),
        ]),
    }
    targets = args.targets or ["--workspace"]
    if targets[0] == "--":
        targets = targets[1:]
    print(f"Coverage with {args.slots} isolated test slots", flush=True)
    try:
        subprocess.run([
            "cargo", "llvm-cov", "nextest", "--no-report", "--locked", *targets,
            "--profile", "ci", "--test-threads", str(args.slots),
        ], env=environment, check=True)
    finally:
        if junit.is_file():
            published_junit.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(junit, published_junit)
    subprocess.run([
        "cargo", "llvm-cov", "report", "--lcov", "--output-path", str(report),
    ], env=environment, check=True)
    if not list(directory.glob("*.profraw")) or not report.is_file():
        raise RuntimeError("coverage requires fresh profiles and a new report")


if __name__ == "__main__":
    main()
