"""Run CI against bounded disposable PostgreSQL; see docs/adr/0055-native-ci-postgres.md."""

import json
import os
from pathlib import Path
import re
import secrets
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
MEMORY_BYTES = 6 * 1024**3


def state_path():
    return Path(os.environ["RUNNER_TEMP"]).resolve() / "tt-ci-postgres.json"


def cleanup(marker=None):
    marker = marker or state_path()
    if not marker.exists():
        return
    state = json.loads(marker.read_text())
    directory = Path(state["directory"]).resolve()
    parent = Path(os.environ["RUNNER_TEMP"]).resolve()
    unit = state["unit"]
    if (not re.fullmatch(r"tt-ci-postgres-[0-9a-f]{32}", unit)
            or directory.parent != parent or not directory.name.startswith("tt-ci-postgres-")):
        raise ValueError("refusing to clean unrelated service or directory")
    stopped = subprocess.run(["systemctl", "--user", "stop", unit], capture_output=True)
    if stopped.returncode:
        state = subprocess.check_output(
            ["systemctl", "--user", "show", unit, "-p", "LoadState", "--value"], text=True
        ).strip()
        if state != "not-found":
            raise RuntimeError("could not stop disposable PostgreSQL; retaining its data")
    log = directory / "postgres.log"
    if log.is_file():
        destination = ROOT / "output" / "ci-native-postgres.log"
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(log, destination)
    shutil.rmtree(directory)
    marker.unlink()


class Backend:
    def __init__(self):
        self.marker = state_path()
        if self.marker.exists():
            raise RuntimeError("a previous PostgreSQL marker requires cleanup first")
        self.directory = Path(tempfile.mkdtemp(prefix="tt-ci-postgres-", dir=self.marker.parent))
        self.unit = "tt-ci-postgres-" + uuid.uuid4().hex
        self.marker.write_text(json.dumps({"unit": self.unit, "directory": str(self.directory)}))
        self.marker.chmod(0o600)

    def start(self):
        for command in ("initdb", "postgres", "psql", "pg_isready", "systemd-run", "systemctl"):
            if not shutil.which(command):
                raise RuntimeError(f"native CI PostgreSQL requires {command} on PATH")
        version = subprocess.check_output(["postgres", "--version"], text=True)
        if not re.search(r"\(PostgreSQL\) 16\.", version):
            raise RuntimeError("native CI requires PostgreSQL 16")
        password = secrets.token_hex(24)
        if os.environ.get("GITHUB_ACTIONS") == "true":
            print(f"::add-mask::{password}", flush=True)
        password_file = self.directory / "password"
        password_file.write_text(password + "\n")
        password_file.chmod(0o600)
        data = self.directory / "data"
        subprocess.run([
            "initdb", "-D", str(data), "--auth=scram-sha-256", "--no-locale",
            "--encoding=UTF8", "--username=postgres", f"--pwfile={password_file}",
        ], check=True, stdout=subprocess.DEVNULL)
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            port = sock.getsockname()[1]
        subprocess.run([
            "systemd-run", "--user", "--quiet", "--collect", f"--unit={self.unit}",
            "--service-type=exec", "--property=MemoryMax=6G", "--property=RuntimeMaxSec=12600",
            "--property=KillSignal=SIGINT", "--property=TimeoutStopSec=15s",
            f"--property=StandardOutput=append:{self.directory / 'postgres.log'}",
            f"--property=StandardError=append:{self.directory / 'postgres.log'}",
            shutil.which("postgres"), "-D", str(data), "-p", str(port),
            "-h", "127.0.0.1", "-c", "unix_socket_directories=",
            "-c", "max_locks_per_transaction=256",
        ], check=True)
        limit = subprocess.check_output([
            "systemctl", "--user", "show", self.unit, "-p", "MemoryMax", "--value",
        ], text=True).strip()
        if limit != str(MEMORY_BYTES):
            raise RuntimeError("PostgreSQL memory limit was not applied")
        for _ in range(100):
            ready = subprocess.run([
                "pg_isready", "-h", "127.0.0.1", "-p", str(port), "-U", "postgres",
            ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            if ready.returncode == 0:
                break
            time.sleep(0.1)
        else:
            raise RuntimeError("disposable PostgreSQL did not become ready")
        base = f"postgresql://postgres:{password}@127.0.0.1:{port}"
        locks = subprocess.check_output([
            "psql", base + "/postgres", "-X", "-v", "ON_ERROR_STOP=1", "-At",
            "-c", "SHOW max_locks_per_transaction",
        ], text=True).strip()
        if locks != "256":
            raise RuntimeError("PostgreSQL shared lock capacity was not applied")
        environment = {**os.environ,
                       "IDENTITY_TEST_DATABASE_URL": base + "/postgres",
                       "CASE_TEST_DATABASE_URL": base + "/case_tests",
                       "DOCUMENT_TEST_DATABASE_URL": base + "/document_tests"}
        subprocess.run(["bash", "scripts/configure-ci-postgres.sh"],
                       cwd=ROOT, env=environment, check=True, stdout=subprocess.DEVNULL)
        subprocess.run([
            "psql", environment["IDENTITY_TEST_DATABASE_URL"], "-X", "-v", "ON_ERROR_STOP=1",
            "-c", "CREATE DATABASE case_tests", "-c", "CREATE DATABASE document_tests",
        ], check=True, stdout=subprocess.DEVNULL)
        print("Native PostgreSQL 16 ready: loopback, SCRAM, MemoryMax=6GiB", flush=True)
        return environment

    def stop(self):
        cleanup(self.marker)


def cancel(signum, _frame):
    raise SystemExit(128 + signum)


def run(command):
    backend = Backend()
    child = None
    try:
        environment = backend.start()
        child = subprocess.Popen(command, cwd=ROOT, env=environment, start_new_session=True)
        return child.wait()
    finally:
        try:
            if child is not None and child.poll() is None:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
        finally:
            backend.stop()


if __name__ == "__main__":
    for event in (signal.SIGINT, signal.SIGTERM):
        signal.signal(event, cancel)
    arguments = sys.argv[1:]
    if arguments == ["--cleanup"]:
        cleanup()
    else:
        if arguments[:1] == ["--"]:
            arguments = arguments[1:]
        if not arguments:
            raise SystemExit("usage: ci_native_postgres.py -- COMMAND [ARG ...] | --cleanup")
        raise SystemExit(run(arguments))
