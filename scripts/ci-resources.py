"""Measure the ordinary build before selecting bounded coverage compile jobs."""
import json
import os
from pathlib import Path
import resource
import subprocess

GIB = 1024**3


def jobs(limit_bytes, compiler_peak_bytes):
    if compiler_peak_bytes <= 0:
        return 1
    # Reserve services/runner memory and twice the measured compiler footprint.
    # The extra margin covers instrumentation and the grouped test harnesses.
    worker_bytes = 2 * compiler_peak_bytes + 256 * 1024**2
    return max(1, min(2, (limit_bytes - 2 * GIB) // worker_bytes))


def memory_limit():
    relative = next(
        line.split(":", 2)[2]
        for line in Path("/proc/self/cgroup").read_text().splitlines()
        if line.startswith("0::")
    )
    root = Path("/sys/fs/cgroup")
    group = root / relative.lstrip("/")
    limits = [6 * GIB]
    while group == root or root in group.parents:
        path = group / "memory.max"
        if path.exists() and path.read_text().strip() != "max":
            limits.append(int(path.read_text()))
        if group == root:
            break
        group = group.parent
    return min(limits)


def main():
    build = subprocess.run(
        ["cargo", "build", "--workspace", "--locked", "--message-format=json"],
        env={**os.environ, "CARGO_BUILD_JOBS": "1"}, text=True, stdout=subprocess.PIPE,
    )
    artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith("{")]
    for item in artifacts:
        if item.get("reason") == "compiler-message":
            print(item["message"].get("rendered", ""), end="")
    build.check_returncode()
    compiled = any(item.get("reason") == "compiler-artifact" and not item.get("fresh")
                   for item in artifacts)
    peak = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss * 1024
    # A warm build cannot measure rustc. Reuse the cold-build high watermark.
    measurement = Path(os.environ["CARGO_TARGET_DIR"]) / "compiler-peak.json"
    previous = json.loads(measurement.read_text()) if measurement.exists() else 0
    peak = max(peak if compiled else 0, previous)
    # Cargo alone on a warm cache is not evidence that two rustc workers fit.
    workers = jobs(memory_limit(), peak) if peak >= 256 * 1024**2 else 1
    measurement.write_text(json.dumps(peak))
    print(f"Compiler peak: {peak // 1024**2} MiB; coverage build jobs: {workers}")
    with open(os.environ["GITHUB_ENV"], "a") as stream:
        stream.write(f"CARGO_BUILD_JOBS={workers}\n")


if __name__ == "__main__":
    main()
