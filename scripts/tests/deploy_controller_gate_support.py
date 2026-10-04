"""Private unit files and a file-backed manager double for persistent entry gates."""
from contextlib import ExitStack, contextmanager
import importlib
import json
import os
from pathlib import Path
import stat
import subprocess
from unittest.mock import patch

from deploy_controller_publication_support import REJECTIONS, put, tree
from deploy_restore_quiesce_support import (
    OPERATION, OTHER, PROPERTIES, UNITS, QuiesceFixture, fingerprint, host, journal,
)


def node(path):
    info = path.lstat()
    return {"device": info.st_dev, "inode": info.st_ino, "uid": info.st_uid,
            "mode": stat.S_IMODE(info.st_mode), "mtime_ns": info.st_mtime_ns,
            "links": info.st_nlink,
            "value": os.readlink(path) if path.is_symlink() else path.read_bytes()}


class GateFixture(QuiesceFixture):
    def setUp(self):
        self.product = importlib.import_module("controller_gate")
        super().setUp()
        put(self.root / "deploy.lock", b"")
        put(self.root / "tools/runtime.py", b"VALUE = 'preserved legacy source'\n")
        put(self.root / "tools/__pycache__/runtime.fixture.pyc", b"preserved cache bytes")
        put(self.units_directory / "unrelated.service", b"[Service]\nExecStart=/bin/true\n")
        for unit in UNITS:
            fragment = self.units_directory / unit
            lines = fragment.read_bytes().splitlines(keepends=True)
            if unit in UNITS[2:]:
                lines = [line for line in lines if not line.startswith(b"ExecStartPre=")]
            fragment.write_bytes(b"".join(lines).replace(b"CPUQuota=200%", b"CPUQuota=175%"))
            self.target["units"][unit]["fragment"] = fingerprint(fragment)
        self.pin_target()
        for row in self.units.values():
            row["UnitFileState"] = "enabled"
        self.manager_path = self.base / "manager.json"
        self.private_json(self.manager_path, self.units)
        self.initialize_gate()

    def adopt(self, base):
        self.product = importlib.import_module("controller_gate")
        self.base = Path(base)
        self.root, self.home, self.runtime = [self.base / name for name in ("root", "home", "runtime")]
        self.units_directory = self.home / ".config/systemd/user"
        self.target_path = self.root / "receipts/restore-target.json"
        self.target = json.loads(self.target_path.read_bytes())
        self.target_digest = fingerprint(self.target_path)["sha256"]
        self.environment = self.target["environment"]
        self.config = self.root / "config/settings.json"
        self.settings = json.loads(self.config.read_bytes())
        self.secret = self.settings["admin_password"]
        self.systemctl = Path(self.target["systemctl"]["path"])
        self.manager_path = self.base / "manager.json"
        self.units = json.loads(self.manager_path.read_bytes())
        self.initialize_gate()

    def initialize_gate(self):
        self.installation = self.root / "maintenance/controller-installation"
        self.operation = self.installation / OPERATION
        self.marker = self.installation / "active.json"
        self.record = self.operation / "journal.json"
        self.slots = self.operation / "slots"
        self.originals = {unit: node(self.units_directory / unit) for unit in UNITS}
        self.protected = self.preserved()
        self.calls, self.events, self.preflight = [], [], set()
        self.lock_entries = 0
        self.now, self.timeout = 10.0, 30
        self.leftovers, self.foreign_ports = {}, set()
        self.process_override = None
        self.show_override = None
        self.reload_behavior = None
        self.after_exchange = None
        self.synced, self.fsynced = [], set()

    def preserved(self):
        return {str(path): tree(path) for path in (
            self.root / "config", self.root / "data", self.root / "tools",
            self.root / "receipts", self.units_directory / "unrelated.service")}

    def snapshot(self):
        return tree(self.root), tree(self.home), tree(self.runtime)

    def record_value(self):
        return json.loads(self.record.read_bytes())

    def sync_gate(self, directory):
        self.assert_locked()
        self.real_sync(directory)
        state = self.record_value()["state"] if self.record.exists() else None
        self.synced.append((Path(directory), state))
        self.events.append(("sync", Path(directory), state))

    def sync_file(self, descriptor):
        self.assert_locked()
        self.real_fsync(descriptor)
        info = os.fstat(descriptor)
        self.fsynced.add((info.st_dev, info.st_ino))

    def exchange(self, fragment, slot):
        self.assert_locked()
        fragment, slot = Path(fragment), Path(slot)
        self.assertEqual(fragment.parent, self.units_directory)
        self.assertIn(fragment.name, UNITS)
        self.assertEqual(slot, self.slots / fragment.name)
        self.assertEqual(self.preflight, set(UNITS))
        observed = {event[1] for event in self.events if event[0] == "processes"}
        self.assertEqual(observed, set(UNITS))
        self.assertEqual(self.record_value()["state"], "masking")
        self.assertEqual(json.loads(self.marker.read_bytes()),
                         {"operation_id": OPERATION, "target_sha256": self.target_digest})
        self.assertIn((self.operation, "masking"), self.synced)
        self.assertTrue(any(path == self.installation for path, _ in self.synced))
        self.assertTrue(any(path == self.slots for path, _ in self.synced))
        for path in (self.marker, self.record):
            info = path.stat()
            self.assertIn((info.st_dev, info.st_ino), self.fsynced)
        self.assertEqual(node(fragment), self.originals[fragment.name])
        self.assertEqual(os.readlink(slot), "/dev/null")
        result = self.real_exchange(fragment, slot)
        self.events.append(("exchange", fragment.name))
        self.assertEqual(node(slot), self.originals[fragment.name])
        if self.after_exchange:
            self.after_exchange(fragment.name)
        return result

    def execute_gate(self, arguments, **kwargs):
        self.assert_locked()
        args = [str(value) for value in arguments]
        self.calls.append((args, kwargs))
        self.assertEqual(args[:2], [str(self.systemctl), "--user"])
        self.assertEqual(kwargs.get("env"), {**self.environment, "LC_ALL": "C"})
        self.assertTrue(0 < kwargs.get("timeout", 0) <= min(10, self.timeout - (self.now - 10)))
        self.assertTrue(0 < kwargs.get("maximum", 0) <= 65536)
        if args[2] == "daemon-reload":
            self.assertEqual(args[2:], ["daemon-reload"])
            self.assertEqual(self.preflight, set(UNITS))
            self.assertIn(self.record_value()["state"], ("reload_pending", "gated"))
            self.assert_masks()
            self.assertTrue(any(path == self.units_directory for path, _ in self.synced))
            self.assertTrue(any(path == self.slots for path, _ in self.synced))
            self.events.append(("reload",))
            if self.reload_behavior == "before":
                raise RuntimeError(self.secret)
            for row in self.units.values():
                row.update(LoadState="masked", UnitFileState="masked",
                           FragmentPath="/dev/null", NeedDaemonReload="no")
            self.private_json(self.manager_path, self.units)
            if self.reload_behavior == "after":
                raise RuntimeError(self.secret)
            if self.reload_behavior == "late":
                self.now += self.timeout + 1
            output = ""
        elif args[2] == "show":
            units = [argument for argument in args[3:] if argument in UNITS]
            self.assertEqual(set(units), set(UNITS))
            self.assertEqual(len(units), len(UNITS))
            requested = set()
            for argument in args:
                if argument.startswith("--property="):
                    requested.update(argument.split("=", 1)[1].split(","))
            self.assertTrue((PROPERTIES | {"UnitFileState"}).issubset(requested))
            rows = []
            for unit in units:
                self.preflight.add(unit)
                row = dict(self.units[unit])
                if self.show_override:
                    self.show_override(unit, row)
                rows.append("\n".join(f"{key}={value}" for key, value in row.items()))
            self.events.append(("show", tuple(units)))
            output = "\n\n".join(rows) + "\n"
        else:
            self.fail("entry gate attempted an operation other than show or daemon-reload")
        return subprocess.CompletedProcess(args, 0, stdout=output if kwargs.get("text") else output.encode(), stderr=b"")

    @contextmanager
    def controller(self):
        module = self.product
        self.real_locked, self.real_sync = journal.locked, journal.sync_directory
        self.real_fsync, self.real_exchange = os.fsync, module._exchange
        with ExitStack() as stack:
            stack.enter_context(patch.dict(os.environ, {**self.environment, "UNRELATED_SECRET": self.secret}, clear=True))
            for owner, name, effect in (
                (module, "run", self.execute_gate), (module, "process_snapshot", self.processes),
                (module, "monotonic", lambda: self.now), (module, "_exchange", self.exchange),
                (module, "sync_directory", self.sync_gate), (journal, "sync_directory", self.sync_gate),
                (journal, "locked", self.lock), (os, "fsync", self.sync_file),
            ):
                stack.enter_context(patch.object(owner, name, side_effect=effect))
            for owner, name in ((host, "prepare"), (host, "start_databases"),
                                (subprocess, "Popen"), (os, "system"), (os, "execve"),
                                (os, "kill"), (os, "killpg")):
                stack.enter_context(patch.object(owner, name, side_effect=AssertionError("unexpected operational edge")))
            yield module

    def invoke(self, module, operation=OPERATION, **overrides):
        arguments = {"target_path": self.target_path,
                     "expected_target_sha256": self.target_digest, "timeout": self.timeout}
        arguments.update(overrides)
        return module.close_entries(self.root, operation, **arguments)

    def assert_masks(self):
        for unit in UNITS:
            mask = self.units_directory / unit
            self.assertTrue(mask.is_symlink())
            self.assertEqual(os.readlink(mask), "/dev/null")
            self.assertEqual(mask.lstat().st_uid, os.getuid())
            self.assertEqual(node(self.slots / unit), self.originals[unit])
        self.assertEqual(self.preserved(), self.protected)

    def assert_gated(self, result):
        self.assertEqual(result, {"operation_id": OPERATION, "target_sha256": self.target_digest, "state": "gated"})
        self.assert_masks()
        self.assertEqual(self.record_value()["state"], "gated")
        self.assertEqual(self.record_value()["operation_id"], OPERATION)
        self.assertEqual(self.record_value()["target_sha256"], self.target_digest)
        self.assertEqual(json.loads(self.marker.read_bytes()), {"operation_id": OPERATION, "target_sha256": self.target_digest})
        for path in (self.marker, self.record):
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
            self.assertNotIn(self.secret.encode(), path.read_bytes())
        for path in (self.installation, self.operation, self.slots):
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o700)
        self.assertEqual(set(self.slots.iterdir()), {self.slots / unit for unit in UNITS})
        for row in self.units.values():
            self.assertEqual((row["LoadState"], row["UnitFileState"]), ("masked", "masked"))
            self.assertEqual((row["ActiveState"], row["SubState"]), ("active", "running"))
            self.assertNotEqual(row["MainPID"], "0")
        self.assertFalse((self.root / "maintenance/restore").exists())
        self.assertFalse((self.root / "maintenance/controllers").exists())

    def reset_observations(self):
        self.calls, self.events, self.preflight = [], [], set()
        self.synced, self.fsynced = [], set()
        self.now = 10.0
