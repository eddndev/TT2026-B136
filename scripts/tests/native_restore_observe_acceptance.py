"""Opt-in acceptance of real Linux observers using one disposable user unit."""
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))

from restore_commands import run
from restore_quiesce_observe import listening_ports, process_snapshot


OPT_IN = "TT_RESTORE_OBSERVER_NATIVE"


def publish(path, value):
    staged = path.with_suffix(".tmp")
    with staged.open("x", encoding="ascii") as stream:
        json.dump(value, stream)
        stream.flush()
        os.fsync(stream.fileno())
    staged.replace(path)


def read_receipt(path):
    with path.open("rb") as stream:
        raw = stream.read(4097)
    if len(raw) > 4096 or not raw.isascii():
        raise ValueError("native observer readiness exceeded its boundary")
    return json.loads(raw)


def start_ticks(pid):
    raw = Path(f"/proc/{pid}/stat").read_text(encoding="ascii")
    return int(raw.rsplit(")", 1)[1].split()[19])


def stopping_event():
    stopped = threading.Event()
    signal.signal(signal.SIGTERM, lambda *_: stopped.set())
    return stopped


def child(directory):
    stopped = stopping_event()
    publish(directory / "child.json", {"pid": os.getpid()})
    if not stopped.wait(55):
        raise RuntimeError("native observer child exceeded its lifetime")


def worker(directory):
    stopped = stopping_event()
    listeners, process = [], None
    try:
        for family, address in ((socket.AF_INET, "127.0.0.1"), (socket.AF_INET6, "::1")):
            for _ in range(8):
                listener = socket.socket(family, socket.SOCK_STREAM)
                if family == socket.AF_INET6:
                    listener.setsockopt(socket.IPPROTO_IPV6, socket.IPV6_V6ONLY, 1)
                listener.bind((address, 0))
                if listener.getsockname()[1] not in [item.getsockname()[1] for item in listeners]:
                    listeners.append(listener)
                    listener.listen(1)
                    break
                listener.close()
            else:
                raise RuntimeError("native observer listener allocation failed")
        process = subprocess.Popen([sys.executable, "-B", str(Path(__file__).resolve()),
                                    "--child", str(directory)], stdin=subprocess.DEVNULL,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        deadline = time.monotonic() + 5
        while not (directory / "child.json").exists():
            if process.poll() is not None or time.monotonic() >= deadline or stopped.is_set():
                raise RuntimeError("native observer child did not become ready")
            time.sleep(0.02)
        if read_receipt(directory / "child.json") != {"pid": process.pid}:
            raise RuntimeError("native observer child identity differs")
        pid = os.getpid()
        group = Path(f"/proc/{pid}/cgroup").read_text(encoding="ascii").strip()
        if not group.startswith("0::") or "\n" in group:
            raise RuntimeError("native observer requires cgroup v2")
        publish(directory / "ready.json", {"pid": pid, "child": process.pid,
            "uid": os.getuid(), "group": group[3:],
            "ticks": {str(pid): start_ticks(pid), str(process.pid): start_ticks(process.pid)},
            "ports": [item.getsockname()[1] for item in listeners]})
        if not stopped.wait(50):
            raise RuntimeError("native observer worker exceeded its lifetime")
        for listener in listeners:
            listener.close()
        process.terminate()
        status = process.wait(timeout=3)
        if status != 0:
            raise RuntimeError("native observer child did not close normally")
        publish(directory / "stopped.json", {"pid": pid, "child": process.pid,
                                             "signal": "SIGTERM", "child_status": status})
    finally:
        for listener in listeners:
            listener.close()
        if process is not None and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3)


@unittest.skipUnless(os.environ.get(OPT_IN) == "1", f"set {OPT_IN}=1 for a disposable native user unit")
class NativeRestoreObservers(unittest.TestCase):
    def executable(self, name):
        candidate = shutil.which(name)
        self.assertIsNotNone(candidate, f"native acceptance requires {name}")
        return Path(candidate).resolve(strict=True)

    def ready(self, directory, launched):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if (directory / "ready.json").exists():
                return read_receipt(directory / "ready.json")
            if launched.done():
                launched.result()
                self.fail("native observer unit exited before readiness")
            time.sleep(0.05)
        self.fail("native observer unit did not become ready within 15 seconds")

    def test_owned_unit_processes_and_ipv4_ipv6_listeners_disappear_after_graceful_stop(self):
        self.assertEqual(sys.platform, "linux")
        self.assertGreater(os.getuid(), 0, "use the current unprivileged user manager")
        systemctl, systemd_run = self.executable("systemctl"), self.executable("systemd-run")
        environment = {key: os.environ[key] for key in
                       ("HOME", "XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS")}
        self.assertEqual(environment["DBUS_SESSION_BUS_ADDRESS"],
                         "unix:path=" + environment["XDG_RUNTIME_DIR"] + "/bus")
        environment["LC_ALL"] = "C"
        unit = "tt-restore-observer-" + str(uuid.uuid4()) + ".service"
        with tempfile.TemporaryDirectory(prefix="tt-restore-observer-") as scratch:
            directory = Path(scratch).resolve()
            directory.chmod(0o700)
            command = [systemd_run, "--user", "--quiet", "--wait", "--collect", "--service-type=exec",
                "--unit=" + unit, "--property=MemoryMax=64M", "--property=RuntimeMaxSec=60s",
                "--property=TimeoutStopSec=5s", "--property=KillMode=mixed",
                "--property=KillSignal=SIGTERM", "--property=StandardOutput=null",
                "--property=StandardError=journal", "--setenv=" + OPT_IN + "=1",
                str(Path(sys.executable).resolve()), "-B", str(Path(__file__).resolve()),
                "--worker", str(directory)]
            with ThreadPoolExecutor(max_workers=1) as executor:
                launched = executor.submit(run, command, env=environment, timeout=75, maximum=65536)
                closed = False
                try:
                    ready = self.ready(directory, launched)
                    self.assertEqual(set(ready), {"pid", "child", "uid", "group", "ticks", "ports"})
                    properties = ("Id", "LoadState", "ActiveState", "SubState", "MainPID",
                                  "ControlGroup", "MemoryMax", "RuntimeMaxUSec")
                    shown = run([systemctl, "--user", "--no-pager", "show", unit,
                        "--property=" + ",".join(properties)], env=environment, text=True,
                        timeout=5, maximum=65536).stdout
                    state = dict(line.split("=", 1) for line in shown.splitlines())
                    self.assertEqual(set(state), set(properties))
                    self.assertEqual((state["Id"], state["LoadState"], state["ActiveState"],
                                      state["SubState"]), (unit, "loaded", "active", "running"))
                    self.assertEqual(state["MemoryMax"], str(64 * 1024 * 1024))
                    self.assertIn(state["RuntimeMaxUSec"], ("1min", "60s", "60000000"))
                    self.assertEqual(int(state["MainPID"]), ready["pid"])
                    group = state["ControlGroup"]
                    self.assertTrue(group.startswith(f"/user.slice/user-{os.getuid()}.slice/"))
                    self.assertTrue(group.endswith("/" + unit))
                    self.assertEqual(ready["group"], group)
                    self.assertEqual(ready["uid"], os.getuid())
                    self.assertNotEqual(ready["pid"], ready["child"])
                    expected = [{"pid": pid, "uid": os.getuid(),
                        "start_ticks": ready["ticks"][str(pid)], "control_group": group}
                        for pid in (ready["pid"], ready["child"])]
                    self.assertTrue(all(row["start_ticks"] > 0 for row in expected))
                    observed = process_snapshot(group, timeout=5)
                    self.assertEqual(sorted(observed, key=lambda row: row["pid"]),
                                     sorted(expected, key=lambda row: row["pid"]))
                    self.assertEqual(len(set(ready["ports"])), 2)
                    self.assertEqual(listening_ports(ready["ports"], timeout=5), sorted(ready["ports"]))
                    run([systemctl, "--user", "stop", unit], env=environment, timeout=10, maximum=65536)
                    self.assertEqual(launched.result(timeout=10).returncode, 0)
                    self.assertEqual(read_receipt(directory / "stopped.json"),
                        {"pid": ready["pid"], "child": ready["child"], "signal": "SIGTERM", "child_status": 0})
                    owned_group = Path("/sys/fs/cgroup") / group.lstrip("/")
                    deadline = time.monotonic() + 5
                    while owned_group.exists() and time.monotonic() < deadline:
                        time.sleep(0.05)
                    self.assertFalse(owned_group.exists(), "stopped native unit retained its cgroup")
                    self.assertEqual(process_snapshot(group, timeout=5), [])
                    self.assertEqual(listening_ports(ready["ports"], timeout=5), [])
                    closed = True
                finally:
                    if not closed:
                        try:
                            run([systemctl, "--user", "stop", unit], env=environment, timeout=10, maximum=65536)
                        except RuntimeError:
                            if not launched.done():
                                raise
                    launched.result(timeout=80)


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] in ("--worker", "--child"):
        if os.environ.get(OPT_IN) != "1":
            raise SystemExit("native observer helpers require explicit opt-in")
        os.umask(0o077)
        target = Path(sys.argv[2]).resolve(strict=True)
        (worker if sys.argv[1] == "--worker" else child)(target)
    else:
        unittest.main()
