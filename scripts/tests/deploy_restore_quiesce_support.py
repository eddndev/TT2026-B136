"""Private target attestation and explicit service/process observation doubles."""
from contextlib import ExitStack, contextmanager
import hashlib
import importlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
import crl_journal as journal
import host

OPERATION = "10000000-0000-4000-8000-000000000001"
OTHER = "20000000-0000-4000-8000-000000000002"
UNITS = ("qadra-web.service", "qadra-api.service", "qadra-postgres.service", "qadra-redis.service")
PROPERTIES = {"Id", "LoadState", "ActiveState", "SubState", "Result", "ExecMainCode",
              "ExecMainStatus", "MainPID", "ControlPID", "ControlGroup", "FragmentPath",
              "DropInPaths", "NeedDaemonReload", "WorkingDirectory", "TimeoutStopUSec"}


def fingerprint(path):
    raw = path.read_bytes()
    return {"path": str(path), "size": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


class ManagerReferences:
    manager = ":1.55"

    def __init__(self, fixture):
        self.fixture = fixture

    def check(self, *, timeout):
        self.fixture.assert_locked()
        self.fixture.assertTrue(0 < timeout <= self.fixture.timeout)

    def status(self, unit, *, timeout):
        self.check(timeout=timeout)
        row = self.fixture.units[unit]
        if row["MainPID"] != "0":
            self.fixture.reference_pids[unit] = int(row["MainPID"])
        pid = self.fixture.reference_pids[unit]
        return {"pid": pid, "code": int(row["ExecMainCode"]), "status": int(row["ExecMainStatus"]),
                "started_usec": pid * 1000, "exited_usec": pid * 1000 + 500 if row["MainPID"] == "0" else 0}


class QuiesceFixture(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.base = Path(self.scratch.name).resolve()
        self.root, self.home, self.runtime = [self.base / name for name in ("root", "home", "runtime")]
        for directory in (self.root, self.home, self.runtime, self.root / "config",
                          self.root / "data", self.root / "receipts", self.root / "tools"):
            directory.mkdir(mode=0o700)
        self.units_directory = self.home / ".config/systemd/user"
        self.units_directory.mkdir(parents=True, mode=0o700)
        self.environment = {"HOME": str(self.home), "XDG_RUNTIME_DIR": str(self.runtime),
                            "DBUS_SESSION_BUS_ADDRESS": "unix:path=" + str(self.runtime / "bus")}
        self.secret = "private-quiesce-fixture-secret"
        self.settings = {"web_port": 18086, "api_port": 18087, "postgres_port": 15486,
                         "redis_port": 16386, "admin_password": self.secret,
                         "runtime_password": self.secret, "redis_password": self.secret, "kek": self.secret}
        self.config = self.root / "config/settings.json"
        self.private_json(self.config, self.settings)
        (self.root / "data/preserved.bin").write_bytes(b"preserve original SQL Redis and PKI")
        self.systemctl = self.root / "tools/systemctl"
        self.systemctl.write_bytes(b"explicit command double, never execute this file\n")
        self.systemctl.chmod(0o700)
        generated = host.service_units(self.root, self.root / "tools", self.root / "tools/python",
                                       self.root / "tools/redis-server")
        self.units, identities = {}, {}
        for index, unit in enumerate(UNITS):
            fragment = self.units_directory / unit
            fragment.write_text(generated[unit.removesuffix(".service")])
            fragment.chmod(0o600)
            group = f"/user.slice/user-{os.getuid()}.slice/user@{os.getuid()}.service/app.slice/{unit}"
            identities[unit] = {"fragment": fingerprint(fragment), "working_directory": str(self.root / "data"),
                                "control_group": group}
            self.units[unit] = {"Id": unit, "LoadState": "loaded", "ActiveState": "active",
                                "SubState": "running", "Result": "success", "ExecMainCode": "0",
                                "ExecMainStatus": "0", "MainPID": str(4100 + index), "ControlPID": "0",
                                "ControlGroup": group, "FragmentPath": str(fragment), "DropInPaths": "",
                                "NeedDaemonReload": "no", "WorkingDirectory": str(self.root / "data"),
                                "TimeoutStopUSec": "1min 30s"}
        root_info = self.root.stat()
        self.target = {"format": "qadra-restore-target", "version": 1, "target_id": OTHER,
                       "root": {"path": str(self.root), "uid": os.getuid(),
                                "device": root_info.st_dev, "inode": root_info.st_ino},
                       "environment": self.environment, "settings": fingerprint(self.config),
                       "systemctl": fingerprint(self.systemctl), "units": identities,
                       "ports": dict(zip(UNITS, (18086, 18087, 15486, 16386)))}
        self.target_path = self.root / "receipts/restore-target.json"
        self.pin_target()
        self.marker = self.root / "maintenance/restore/active.json"
        self.operation = self.root / "maintenance/restore" / OPERATION
        self.record = self.operation / "journal.json"
        self.calls, self.events, self.preflight = [], [], set()
        self.lock_entries = 0
        self.fence_durable = False
        self.now, self.stop_cost = 10.0, 0.0
        self.after_stop = None
        self.show_override = None
        self.leftovers, self.foreign_ports = {}, set()
        self.process_override = None
        self.reference_pids = {unit: int(row["MainPID"]) for unit, row in self.units.items()}
        self.before_data = {path: path.read_bytes() for path in (self.config, self.root / "data/preserved.bin")}

    def private_json(self, path, value):
        path.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="ascii")
        path.chmod(0o600)

    def pin_target(self):
        self.private_json(self.target_path, self.target)
        self.target_digest = fingerprint(self.target_path)["sha256"]

    def assert_locked(self):
        with self.assertRaises(BlockingIOError, msg="quiesce released the deployment lock"):
            with self.real_locked(self.root):
                pass

    @contextmanager
    def lock(self, root):
        self.lock_entries += 1
        with self.real_locked(root):
            yield

    def sync(self, directory):
        self.assert_locked()
        self.real_sync(directory)
        if Path(directory) == self.marker.parent and self.marker.exists():
            self.fence_durable = True
        if self.record.exists():
            value = json.loads(self.record.read_bytes())
            self.events.append(("sync", Path(directory), value.get("state")))

    def execute(self, arguments, **kwargs):
        self.assert_locked()
        args = [str(value) for value in arguments]
        self.calls.append((args, kwargs))
        self.assertEqual(args[0], str(self.systemctl))
        self.assertEqual(args[1], "--user")
        self.assertNotIn(self.secret, json.dumps(args))
        self.assertTrue(0 < kwargs.get("timeout", 0) <= self.timeout - (self.now - 10.0))
        self.assertLessEqual(kwargs.get("maximum", 0), 65536)
        self.assertGreater(kwargs.get("maximum", 0), 0)
        env = kwargs.get("env", {})
        self.assertTrue(all(env.get(key) == value for key, value in self.environment.items()))
        self.assertNotIn(self.secret, json.dumps(env))
        self.assertEqual(env.get("LC_ALL"), "C")
        units = [value for value in args if value in UNITS]
        self.assertTrue(units)
        if args[2] == "show":
            requested = set()
            for index, value in enumerate(args):
                if value.startswith("--property="):
                    requested.update(value.split("=", 1)[1].split(","))
                if value in ("--property", "-p"):
                    requested.update(args[index + 1].split(","))
            self.assertTrue(PROPERTIES.issubset(requested), "unit snapshot is incomplete")
            rows = []
            for unit in units:
                self.preflight.add(unit)
                row = dict(self.units[unit])
                if self.show_override:
                    self.show_override(unit, row)
                rows.append("\n".join(f"{key}={value}" for key, value in row.items()))
            output = "\n\n".join(rows) + "\n"
        elif args[2] == "stop":
            self.assertEqual(self.preflight, set(UNITS), "stop preceded complete target inspection")
            self.assertTrue(self.fence_durable, "stop preceded durable admission closure")
            self.assertEqual(json.loads(self.marker.read_bytes()), {"operation_id": OPERATION})
            record = json.loads(self.record.read_bytes())
            self.assertEqual(record["operation_id"], OPERATION)
            self.assertEqual(record["target_sha256"], self.target_digest)
            self.assertEqual(record["state"], "closing")
            self.assertIn(("sync", self.operation, "closing"), self.events)
            self.assertGreaterEqual(kwargs["timeout"], 90, "stop cannot cover existing service grace period")
            for unit in units:
                if unit in UNITS[2:]:
                    self.assertTrue(all(self.units[name]["ActiveState"] == "inactive" for name in UNITS[:2]))
                    self.assertTrue(all(self.units[name]["MainPID"] == "0" for name in UNITS[:2]))
                self.events.append(("stop", unit))
                self.units[unit].update(ActiveState="inactive", SubState="dead", MainPID="0",
                                        ExecMainCode="1", ExecMainStatus="0")
            self.now += self.stop_cost
            if self.after_stop:
                self.after_stop(units)
            output = ""
        else:
            self.fail("quiesce attempted an action other than show or stop")
        return subprocess.CompletedProcess(args, 0, stdout=output if kwargs.get("text") else output.encode(), stderr=b"")

    def processes(self, control_group, *, timeout):
        self.assert_locked()
        self.assertTrue(0 < timeout <= self.timeout)
        unit = next(name for name, row in self.units.items() if row["ControlGroup"] == control_group)
        row = self.units[unit]
        members = list(self.leftovers.get(unit, []))
        if row["MainPID"] != "0":
            members.append({"pid": int(row["MainPID"]), "uid": os.getuid(),
                            "start_ticks": 12345, "control_group": control_group})
        if self.process_override:
            self.process_override(unit, members)
        self.events.append(("processes", unit, len(members)))
        return members

    def listeners(self, ports, *, timeout):
        self.assert_locked()
        self.assertTrue(0 < timeout <= self.timeout)
        self.assertTrue(set(ports).issubset(set(self.target["ports"].values())))
        active = {port for unit, port in self.target["ports"].items()
                  if self.units[unit]["ActiveState"] != "inactive"}
        self.events.append(("listeners", tuple(sorted(ports))))
        return sorted((active | self.foreign_ports) & set(ports))

    @contextmanager
    def retain_units(self, environment, units, *, timeout):
        self.assert_locked()
        self.assertEqual(environment, {**self.environment, "LC_ALL": "C"})
        self.assertEqual(tuple(units), UNITS)
        self.assertTrue(0 < timeout <= self.timeout)
        try:
            yield ManagerReferences(self)
        finally:
            self.assert_locked()

    @contextmanager
    def controller(self, timeout=240):
        module = importlib.import_module("restore_quiesce")
        self.timeout = timeout
        self.real_sync = journal.sync_directory
        self.real_locked = journal.locked
        with ExitStack() as stack:
            stack.enter_context(patch.dict(os.environ, {**self.environment, "UNRELATED_SECRET": self.secret}, clear=True))
            stack.enter_context(patch.object(module, "run", side_effect=self.execute))
            stack.enter_context(patch.object(module, "process_snapshot", side_effect=self.processes))
            stack.enter_context(patch.object(module, "listening_ports", side_effect=self.listeners))
            stack.enter_context(patch.object(module, "retain_units", side_effect=self.retain_units))
            stack.enter_context(patch.object(module, "monotonic", side_effect=lambda: self.now))
            stack.enter_context(patch.object(journal, "locked", side_effect=self.lock))
            stack.enter_context(patch.object(journal, "sync_directory", side_effect=self.sync))
            stack.enter_context(patch.object(os, "kill", side_effect=AssertionError("must not signal a PID")))
            stack.enter_context(patch.object(os, "killpg", side_effect=AssertionError("must not signal a process group")))
            yield module

    def invoke(self, module, operation=OPERATION, **overrides):
        arguments = dict(target_path=self.target_path, expected_target_sha256=self.target_digest, timeout=self.timeout)
        arguments.update(overrides)
        return module.quiesce(self.root, operation, **arguments)

    def stops(self):
        return [event[1] for event in self.events if event[0] == "stop"]

    def assert_preserved(self):
        for path, value in self.before_data.items():
            self.assertEqual(path.read_bytes(), value)
        for path in (self.marker, self.record):
            self.assertTrue(path.is_file())
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
            self.assertNotIn(self.secret.encode(), path.read_bytes())
        self.assertEqual(json.loads(self.marker.read_bytes()), {"operation_id": OPERATION})
        self.assertEqual(stat.S_IMODE(self.operation.stat().st_mode), 0o700)
