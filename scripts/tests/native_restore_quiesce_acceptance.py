"""Opt-in complete quiesce protocol against four harmless disposable local services."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import sys
import tempfile
import threading
import time
import unittest
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
from restore_commands import run
from restore_quiesce import PROPERTIES, UNITS, quiesce
from restore_quiesce_observe import listening_ports

OPT_IN = "TT_RESTORE_QUIESCE_NATIVE"


def fingerprint(path):
    data = path.read_bytes()
    return {"path": str(path), "size": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def private_json(path, value):
    with path.open("x", encoding="ascii") as stream:
        json.dump(value, stream)
        stream.flush()
        os.fsync(stream.fileno())
    path.chmod(0o600)


def rows(raw):
    result = {}
    for block in raw.strip().split("\n\n"):
        row = dict(line.split("=", 1) for line in block.splitlines())
        result[row["Id"]] = row
    return result


def worker(root, unit):
    if unit not in UNITS:
        raise RuntimeError("only the disposable fixture services are accepted")
    stopped = threading.Event()
    signal.signal(signal.SIGINT, lambda *_: stopped.set())
    ports = json.loads((root / "config/settings.json").read_text())
    port = ports[dict(zip(UNITS, ("web_port", "api_port", "postgres_port", "redis_port")))[unit]]
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", port))
        listener.listen(1)
        private_json(root / "run" / (unit + ".ready"), {"pid": os.getpid()})
        if not stopped.wait(50):
            raise RuntimeError("disposable fixture service exceeded its lifetime")
    private_json(root / "run" / (unit + ".closed"), {"signal": "SIGINT", "status": 0})


@unittest.skipUnless(os.environ.get(OPT_IN) == "1", "explicit local quiesce acceptance is required")
class NativeQuiesce(unittest.TestCase):
    def test_ordered_stop_and_exact_reentry_preserve_private_data(self):
        self.assertGreater(os.getuid(), 0)
        controller = Path(shutil.which("systemctl")).resolve(strict=True)
        environment = {key: os.environ[key] for key in
                       ("HOME", "XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS")}
        environment["LC_ALL"] = "C"
        def command(*args):
            return run([controller, "--user", *args], env=environment,
                       text=True, timeout=15, maximum=65536).stdout
        initial = rows(command("show", *UNITS, "--property=Id,LoadState,FragmentPath"))
        self.assertEqual(set(initial), set(UNITS))
        self.assertTrue(all(r["LoadState"] == "not-found" and not r["FragmentPath"]
                            for r in initial.values()), "refusing to use existing Qadra services")
        units_directory = Path(environment["HOME"]) / ".config/systemd/user"
        self.assertTrue(units_directory.is_dir())
        self.assertTrue(all(not (units_directory / unit).exists() and
                            not (units_directory / unit).is_symlink() for unit in UNITS))
        created = {}
        with tempfile.TemporaryDirectory(prefix="tt-quiesce-native-") as scratch:
            root = Path(scratch).resolve()
            root.chmod(0o700)
            for name in ("data", "config", "receipts", "run"):
                (root / name).mkdir(mode=0o700)
            sentinel = root / "data/untouched.bin"
            sentinel.write_bytes(b"synthetic SQL Redis and PKI remain untouched\n")
            before = fingerprint(sentinel)
            reservations = []
            try:
                for _ in UNITS:
                    reservation = socket.socket()
                    reservation.bind(("127.0.0.1", 0))
                    reservations.append(reservation)
                values = [sock.getsockname()[1] for sock in reservations]
                ports = dict(zip(UNITS, values))
                settings = dict(zip(("web_port", "api_port", "postgres_port", "redis_port"), values))
                private_json(root / "config/settings.json", settings)
                for unit in UNITS:
                    source = (
                        "[Unit]\nDescription=Disposable native quiesce acceptance\n[Service]\n"
                        f"Type=simple\nWorkingDirectory={root}/data\n"
                        f"ExecStart={Path(sys.executable).resolve()} -B {Path(__file__).resolve()} --worker {root} {unit}\n"
                        f"Environment={OPT_IN}=1\nMemoryMax=32M\nRuntimeMaxSec=60\n"
                        "TimeoutStopSec=90\nKillSignal=SIGINT\nKillMode=control-group\n"
                        "StandardOutput=null\nStandardError=journal\nUMask=0077\n"
                    )
                    path = units_directory / unit
                    with path.open("x", encoding="ascii") as stream:
                        stream.write(source)
                    path.chmod(0o600)
                    created[path] = source.encode()
                command("daemon-reload")
                inherited = rows(command("show", *UNITS, "--property=Id,DropInPaths"))
                self.assertTrue(all(not row["DropInPaths"] for row in inherited.values()),
                                "use a disposable user manager without inherited unit overrides")
                for sock in reservations:
                    sock.close()
                command("start", *UNITS)
                deadline = time.monotonic() + 10
                while not all((root / "run" / (unit + ".ready")).exists() for unit in UNITS):
                    if time.monotonic() >= deadline:
                        self.fail("disposable fixture services did not become ready")
                    time.sleep(0.025)
                shown = rows(command("show", *UNITS, "--property=" + ",".join(PROPERTIES)))
                self.assertTrue(all(row["ActiveState"] == "active" for row in shown.values()))
                self.assertEqual(listening_ports(values, timeout=5), sorted(values))
                info = root.stat()
                target = {"format": "qadra-restore-target", "version": 1,
                    "target_id": str(uuid.uuid4()),
                    "root": {"path": str(root), "uid": os.getuid(), "device": info.st_dev, "inode": info.st_ino},
                    "environment": {k: v for k, v in environment.items() if k != "LC_ALL"},
                    "settings": fingerprint(root / "config/settings.json"),
                    "systemctl": fingerprint(controller), "ports": ports,
                    "units": {unit: {"fragment": fingerprint(units_directory / unit),
                        "working_directory": str(root / "data"), "control_group": shown[unit]["ControlGroup"]}
                        for unit in UNITS}}
                path = root / "receipts/target.json"
                private_json(path, target)
                digest = fingerprint(path)["sha256"]
                operation = str(uuid.uuid4())
                try:
                    result = quiesce(root, operation, target_path=path, expected_target_sha256=digest)
                except RuntimeError:
                    state = rows(command("show", *UNITS, "--property=" + ",".join(PROPERTIES)))
                    self.fail("native closure failed: " + json.dumps(state, sort_keys=True))
                self.assertEqual(result, {"operation_id": operation, "target_sha256": digest, "state": "stopped"})
                for unit in UNITS:
                    self.assertEqual(json.loads((root / "run" / (unit + ".closed")).read_text()),
                                     {"signal": "SIGINT", "status": 0})
                self.assertEqual(fingerprint(sentinel), before)
                self.assertEqual(listening_ports(values, timeout=5), [])
                self.assertEqual(quiesce(root, operation, target_path=path, expected_target_sha256=digest), result)
                marker = root / "maintenance/restore/active.json"
                self.assertEqual(json.loads(marker.read_text()), {"operation_id": operation})
            finally:
                for sock in reservations:
                    sock.close()
                if created:
                    command("stop", *[path.name for path in created])
                for path, data in created.items():
                    self.assertEqual(path.read_bytes(), data, "refusing to remove a replaced unit fragment")
                    path.unlink()
                if created:
                    command("daemon-reload")
        final = rows(command("show", *UNITS, "--property=Id,LoadState,FragmentPath"))
        self.assertTrue(all(r["LoadState"] == "not-found" and not r["FragmentPath"] for r in final.values()))


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--worker":
        if os.environ.get(OPT_IN) != "1":
            raise SystemExit("explicit native fixture opt-in required")
        worker(Path(sys.argv[2]), sys.argv[3])
    else:
        unittest.main()
