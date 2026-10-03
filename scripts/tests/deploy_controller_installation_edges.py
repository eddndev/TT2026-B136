"""External service observations and syscall fault points for installer fixtures."""
import json
import os
from pathlib import Path
import stat
import subprocess

from deploy_controller_publication_support import identity, inventory
from deploy_restore_quiesce_support import UNITS


class InstallationEdges:
    def processes(self, control_group, *, timeout):
        self.assert_locked()
        self.assertTrue(0 < timeout <= self.timeout)
        unit = next(name for name in UNITS if self.target["units"][name]["control_group"] == control_group)
        row = self.units[unit]
        members = [] if row["MainPID"] == "0" else [{
            "pid": int(row["MainPID"]), "uid": os.getuid(), "start_ticks": 12345,
            "control_group": control_group,
        }]
        self.events.append(("processes", unit, len(members)))
        return members

    def execute_installation(self, arguments, **kwargs):
        self.assert_locked()
        args = [str(value) for value in arguments]
        self.assertEqual(args[:2], [str(self.systemctl), "--user"])
        self.assertEqual(kwargs.get("env"), {**self.environment, "LC_ALL": "C"})
        self.assertTrue(0 < kwargs.get("timeout", 0) <= self.timeout)
        self.assertTrue(0 < kwargs.get("maximum", 0) <= 65536)
        self.assertNotIn(self.secret, json.dumps(args))
        self.calls.append((args, kwargs))
        action = args[2]
        selected = [name for name in args[3:] if name in UNITS]
        if action == "show":
            requested = next(value.split("=", 1)[1].split(",") for value in args if value.startswith("--property="))
            self.assertTrue(selected)
            rows = []
            for unit in selected:
                self.preflight.add(unit)
                row = dict(self.units[unit])
                self.assertTrue(set(requested).issubset(row))
                rows.append("\n".join(f"{name}={row[name]}" for name in requested))
            output = "\n\n".join(rows) + "\n"
        elif action == "stop":
            self.assertTrue(self.references_active)
            self.assertGreaterEqual(kwargs["timeout"], 90)
            self.assertFalse((self.root / "maintenance/restore/active.json").exists())
            self.assertEqual(self.preflight, set(UNITS))
            record = json.loads(self.stop_record.read_bytes())
            self.assertEqual(record["state"], "closing")
            self.assertIn(identity(self.stop_record), self.fsynced)
            self.assertIn(identity(self.operation), self.fsynced)
            self.assertTrue(self.intent_record.exists())
            for unit in selected:
                self.assertFalse((self.units_directory / unit).is_symlink())
                self.assertEqual(record["units"][unit]["process"]["pid"], int(self.units[unit]["MainPID"]))
                self.assertIsNone(record["units"][unit]["exit"])
                if unit in UNITS[2:]:
                    self.assertTrue(all(self.units[name]["ActiveState"] == "inactive" for name in UNITS[:2]))
                self.units[unit].update(ActiveState="inactive", SubState="dead", MainPID="0",
                                        ExecMainCode="1", ExecMainStatus="0")
                self.events.append(("stop", unit))
            if self.fault == "stop_response" and not self.fault_reached:
                self.fault_reached = True
                raise RuntimeError(self.secret)
            output = ""
        elif action == "daemon-reload":
            self.assertEqual(selected, [])
            self.events.append(("reload",))
            if self.fault == "reopen_reload" and any(not (self.units_directory / unit).is_symlink() for unit in UNITS):
                self.fault_reached = True
                raise RuntimeError(self.secret)
            for unit, row in self.units.items():
                masked = (self.units_directory / unit).is_symlink()
                row.update(LoadState="masked" if masked else "loaded", NeedDaemonReload="no",
                           UnitFileState="masked" if masked else "enabled",
                           FragmentPath="/dev/null" if masked else str(self.units_directory / unit))
            output = ""
        elif action == "start":
            self.assertGreaterEqual(kwargs["timeout"], 240)
            self.assertTrue(self.reopen_record.exists())
            self.assertIn(identity(self.reopen_record), self.fsynced)
            self.assertIn(identity(self.operation), self.fsynced)
            self.assertTrue(selected)
            for unit in selected:
                self.assertFalse((self.units_directory / unit).is_symlink())
                self.assertEqual(self.units[unit]["LoadState"], "loaded")
                self.assertIn(self.candidate_sha.encode(), (self.units_directory / "qadra-api.service").read_bytes())
                if unit == UNITS[1]:
                    self.assertTrue(all(self.units[name]["ActiveState"] == "active" for name in UNITS[2:]))
                if unit == UNITS[0]:
                    self.assertEqual(self.units[UNITS[1]]["ActiveState"], "active")
                self.units[unit].update(ActiveState="active", SubState="running", MainPID=str(6100 + UNITS.index(unit)),
                                        ExecMainCode="0", ExecMainStatus="0")
                self.events.append(("start", unit))
            output = ""
        else:
            self.fail("installer attempted an unsupported service operation")
        return subprocess.CompletedProcess(args, 0, stdout=output if kwargs.get("text") else output.encode(), stderr=b"")

    def sync_file(self, descriptor):
        self.assert_locked()
        self.real_fsync(descriptor)
        info = os.fstat(descriptor)
        observed = info.st_dev, info.st_ino
        self.fsynced.add(observed)
        if stat.S_ISDIR(info.st_mode) and self.operation.exists() and observed == identity(self.operation):
            if self.stop_record.exists() and json.loads(self.stop_record.read_bytes())["state"] == "stopped":
                self.assertIn(identity(self.stop_record), self.fsynced)
                if not self.stop_durable:
                    self.stop_durable = True
                    self.events.append(("stop-durable",))
        if self.fault == "cache_sync" and not self.fault_reached and self.retained_cache.exists():
            self.assertFalse(self.cache.exists())
            self.fault_reached = True
            raise OSError(self.secret)

    def rename(self, source, destination, flags, check):
        source, destination = Path(source), Path(destination)
        unit = next((path for path in (source, destination) if path.parent == self.units_directory), None)
        reopening = unit is not None and unit.is_symlink()
        if unit is not None:
            self.assertEqual(flags, 2)
            self.assertTrue(self.stop_durable, "unit replacement preceded durable exact stop")
            if reopening:
                self.assertTrue(self.reopen_record.exists())
                self.assertIn(identity(self.reopen_record), self.fsynced)
                self.assertIn(identity(self.operation), self.fsynced)
            else:
                self.assertEqual(self.python_inventory(self.root / "tools"), self.previous_files)
                self.assertTrue(self.cache.exists())
        result = self.real_rename(source, destination, flags, check)
        if unit is not None:
            event = "reopen" if reopening else "mask"
            self.units[unit.name]["NeedDaemonReload"] = "yes"
            self.events.append((event, unit.name))
            count = sum(entry[0] == event for entry in self.events)
            if self.fault == event and count == 1 and not self.fault_reached:
                self.fault_reached = True
                raise OSError(self.secret)
        return result

    def exchange_sources(self, left, right):
        self.assert_locked()
        self.assertTrue(self.stop_durable)
        self.assertTrue(all((self.units_directory / unit).is_symlink() for unit in UNITS))
        self.assertTrue(all(row["LoadState"] == "masked" for row in self.units.values()))
        self.assertFalse(self.cache.exists())
        result = self.real_source_exchange(left, right)
        self.events.append(("publish",))
        if self.fault == "publication" and not self.fault_reached:
            self.fault_reached = True
            raise OSError(self.secret)
        return result

    def replace(self, source, destination):
        result = self.real_replace(source, destination)
        if Path(destination) == self.current_approval:
            self.events.append(("approve",))
            if self.fault == "approval" and not self.fault_reached:
                self.fault_reached = True
                raise OSError(self.secret)
        return result

    def ready(self, runtime, release, *, deadline=None):
        self.assert_locked()
        self.assertIsNotNone(deadline)
        self.assertTrue(0 < deadline - self.now <= self.timeout)
        self.assertEqual(runtime.root, self.root)
        self.assertEqual(Path(release), self.release)
        self.assertTrue(all(row["ActiveState"] == "active" for row in self.units.values()))
        self.assertEqual(inventory(self.root / "tools"), self.candidate_files)
        self.events.append(("ready",))
        if self.fault == "readiness":
            self.fault_reached = True
            raise RuntimeError(self.secret)
        if self.fault == "readiness_deadline":
            self.fault_reached = True
            self.now = deadline + 1
