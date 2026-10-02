"""Run each workspace test executable on exactly one coverage runner."""

import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess


# See docs/adr/0049-parallel-coverage-runners.md for the capacity ratio.
RUNNER_RELATIVE_CAPACITY = (2, 3)


def workspace_targets(metadata):
    members = set(metadata["workspace_members"])
    targets = []
    for package in metadata["packages"]:
        if package["id"] not in members:
            continue
        for target in package["targets"]:
            kind = target["kind"][0]
            if kind in {"lib", "bin", "test"}:
                targets.append((package["name"], kind, target["name"]))
    return targets


def weight(target, weights):
    package, kind, name = target
    if package != "infrastructure":
        return 0
    if kind == "lib":
        return 20
    return weights.get(name, 180 if "suite" in name else 30)


def split_targets(targets, weights):
    first = [target for target in targets if target[0] != "infrastructure"]
    second = []
    totals = [120, 0]
    infrastructure = sorted(
        (target for target in targets if target[0] == "infrastructure"),
        key=lambda target: (-weight(target, weights), target),
    )
    for target in infrastructure:
        shard = (0 if totals[0] * RUNNER_RELATIVE_CAPACITY[1]
                 <= totals[1] * RUNNER_RELATIVE_CAPACITY[0] else 1)
        (first, second)[shard].append(target)
        totals[shard] += weight(target, weights)
    return first, second


def command(target):
    package, kind, name = target
    args = ["cargo", "llvm-cov", "--no-report", "--locked", "-p", package]
    if kind == "lib":
        args.append("--lib")
    elif kind == "bin":
        args.extend(["--bin", name])
    else:
        args.extend(["--test", name])
    if package == "infrastructure":
        # Fixtures share a database-wide audit lock; see docs/adr/0049-parallel-coverage-runners.md.
        args.extend(["--", "--test-threads=1"])
    return args


def coverage_helpers(root):
    path = root / "scripts" / "ci-coverage.py"
    spec = importlib.util.spec_from_file_location("ci_coverage_existing", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("shard", type=int, choices=(1, 2))
    args = parser.parse_args()
    root = Path.cwd()
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"], text=True
    ))
    weights = json.loads((root / "scripts" / "ci-test-duration-seconds.json").read_text())
    targets = workspace_targets(metadata)
    shards = split_targets(targets, weights)
    selected = shards[args.shard - 1]
    if (not selected or set(shards[0]) & set(shards[1])
            or set(shards[0]) | set(shards[1]) != set(targets)
            or len(shards[0]) + len(shards[1]) != len(targets)):
        raise RuntimeError("coverage shards must partition every test executable")
    version = subprocess.check_output(["cargo", "llvm-cov", "--version"], text=True)
    toolchain = version + subprocess.check_output(["rustc", "-Vv"], text=True)
    toolchain += subprocess.check_output(["cargo", "-V"], text=True)
    helpers = coverage_helpers(root)
    target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
    directory = target.parent / "coverage" / helpers.cache_key(root, os.environ, toolchain)
    directory.mkdir(parents=True, exist_ok=True)
    report = root / f"coverage-shard-{args.shard}.info"
    helpers.clear_measurements(directory, report)
    environment = {**os.environ, "CARGO_LLVM_COV_TARGET_DIR": str(directory)}
    print(f"Coverage shard {args.shard}/2: {len(selected)} test executables", flush=True)
    for package, kind, name in selected:
        print(f"Running {package} {kind} {name}", flush=True)
        subprocess.run(command((package, kind, name)), env=environment, check=True)
    subprocess.run(
        ["cargo", "llvm-cov", "report", "--lcov", "--output-path", str(report)],
        env=environment, check=True,
    )
    if not list(directory.glob("*.profraw")) or not report.is_file():
        raise RuntimeError("coverage shard requires fresh profiles and a new report")


if __name__ == "__main__":
    main()
