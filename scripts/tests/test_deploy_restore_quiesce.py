"""Ordered, bounded closure of the exact managed deployment before restoration."""
import copy
import json
import os
from unittest.mock import patch

from deploy_restore_quiesce_support import OPERATION, UNITS, QuiesceFixture, journal


class RestoreQuiesceTests(QuiesceFixture):
    def test_fence_and_journal_are_durable_before_ordered_stop_under_one_lock(self):
        original_target = self.target_path.read_bytes()
        with self.controller() as module:
            result = self.invoke(module)
        self.assertEqual(result, {"operation_id": OPERATION, "target_sha256": self.target_digest,
                                  "state": "stopped"})
        self.assertEqual(self.lock_entries, 1, "lock acquisition must cover the entire operation without a gap")
        self.assertEqual(set(self.stops()), set(UNITS))
        positions = {name: self.stops().index(name) for name in UNITS}
        self.assertLess(max(positions[name] for name in UNITS[:2]), min(positions[name] for name in UNITS[2:]))
        self.assertEqual(json.loads(self.record.read_bytes())["state"], "stopped")
        self.assertIn(("sync", self.operation, "stopped"), self.events)
        for unit in UNITS:
            self.assertIn(("processes", unit, 0), self.events)
        observed_ports = set().union(*(set(event[1]) for event in self.events if event[0] == "listeners"))
        self.assertEqual(observed_ports, set(self.target["ports"].values()))
        self.assertEqual(self.target_path.read_bytes(), original_target)
        self.assert_preserved()

    def test_changed_root_config_environment_or_attestation_rejects_before_stop(self):
        original = copy.deepcopy(self.target)
        mutations = (
            lambda value: value["root"].update(uid=os.getuid() + 1),
            lambda value: value["root"].update(inode=value["root"]["inode"] + 1),
            lambda value: value["root"].update(device=value["root"]["device"] + 1),
            lambda value: value["root"].update(path=str(self.home)),
            lambda value: value["settings"].update(sha256="0" * 64),
            lambda value: value["ports"].update({UNITS[0]: 18099}),
            lambda value: value["systemctl"].update(sha256="0" * 64),
            lambda value: value["environment"].update(DBUS_SESSION_BUS_ADDRESS="tcp:host=example.test,port=9999"),
        )
        with self.controller() as module:
            for mutation in mutations:
                self.target = copy.deepcopy(original)
                mutation(self.target)
                self.pin_target()
                with self.subTest(target=self.target_digest), self.assertRaises((ValueError, RuntimeError)):
                    self.invoke(module)
                self.assertEqual(self.stops(), [])
            self.target = original
            self.pin_target()
            for key in self.environment:
                with self.subTest(environment=key), patch.dict(os.environ, {key: "/foreign-context"}):
                    with self.assertRaises((ValueError, RuntimeError)):
                        self.invoke(module)
            with self.assertRaises((ValueError, RuntimeError)):
                self.invoke(module, expected_target_sha256="0" * 64)
        self.assertEqual(self.stops(), [])
        for path, content in self.before_data.items():
            self.assertEqual(path.read_bytes(), content)

    def test_redirected_root_or_unit_fragment_and_unsafe_private_files_never_stop(self):
        with self.controller() as module:
            redirected = self.base / "root-link"
            redirected.symlink_to(self.root, target_is_directory=True)
            with self.assertRaises((ValueError, RuntimeError, OSError)):
                module.quiesce(redirected, OPERATION, target_path=self.target_path,
                               expected_target_sha256=self.target_digest, timeout=240)
            self.target_path.chmod(0o644)
            with self.assertRaises((ValueError, RuntimeError)):
                self.invoke(module)
            self.target_path.chmod(0o600)
            fragment = self.units_directory / UNITS[0]
            content = fragment.read_bytes()
            fragment.unlink()
            outside = self.base / "foreign.service"
            outside.write_bytes(content)
            fragment.symlink_to(outside)
            with self.assertRaises((ValueError, RuntimeError, OSError)):
                self.invoke(module)
        self.assertEqual(self.stops(), [])

    def test_complete_loaded_unit_snapshot_is_required_before_first_stop(self):
        failures = {"Id": "foreign.service", "LoadState": "not-found",
                    "FragmentPath": str(self.base / "foreign.service"),
                    "WorkingDirectory": str(self.home), "DropInPaths": "/foreign/override.conf",
                    "NeedDaemonReload": "yes", "ControlGroup": "/foreign.slice",
                    "TimeoutStopUSec": "infinity"}
        with self.controller() as module:
            for property_name, value in failures.items():
                def changed(unit, row):
                    if unit == UNITS[-1]:
                        row[property_name] = value
                self.show_override = changed
                with self.subTest(property=property_name), self.assertRaises((ValueError, RuntimeError)):
                    self.invoke(module)
                self.assertEqual(self.stops(), [])
            self.show_override = lambda unit, row: row.pop("Result", None)
            with self.assertRaises((ValueError, RuntimeError)):
                self.invoke(module)
        self.assertEqual(self.stops(), [])

    def test_foreign_process_identity_rejects_without_signalling_or_stopping(self):
        def foreign(unit, members):
            if unit == UNITS[-1]:
                members[0]["uid"] = os.getuid() + 1
        self.process_override = foreign
        with self.controller() as module, self.assertRaises((ValueError, RuntimeError)):
            self.invoke(module)
        self.assertEqual(self.stops(), [])

    def test_failed_timeout_or_forced_exit_cannot_be_reported_as_stopped(self):
        for result, code, status in (("timeout", "1", "0"), ("signal", "2", "9"),
                                     ("success", "2", "9"), ("success", "1", "1")):
            fixture = QuiesceFixture()
            fixture.setUp()
            self.addCleanup(fixture.doCleanups)
            def failed(units):
                fixture.units[UNITS[0]].update(Result=result, ExecMainCode=code, ExecMainStatus=status)
            fixture.after_stop = failed
            with self.subTest(result=result, code=code, status=status), fixture.controller() as module:
                with self.assertRaises((ValueError, RuntimeError)):
                    fixture.invoke(module)
            self.assertTrue(set(fixture.stops()).issubset(set(UNITS[:2])))
            self.assertEqual(json.loads(fixture.record.read_bytes())["state"], "closing")
            fixture.assert_preserved()

    def test_deadline_is_explicit_and_exhaustion_never_starts_the_database_stop(self):
        with self.controller() as module:
            for timeout in (None, True, 0, 89, 301, float("inf")):
                with self.subTest(timeout=timeout), self.assertRaises((ValueError, RuntimeError)):
                    self.invoke(module, timeout=timeout)
            self.assertEqual(self.calls, [])
            self.stop_cost = 241
            with self.assertRaises((ValueError, RuntimeError)):
                self.invoke(module)
        self.assertTrue(set(self.stops()).issubset(set(UNITS[:2])))
        self.assertEqual(json.loads(self.record.read_bytes())["state"], "closing")
        self.assert_preserved()

    def test_existing_deployment_lock_rejects_before_observation_or_fence_write(self):
        with self.controller() as module, journal.locked(self.root):
            with self.assertRaises((BlockingIOError, ValueError, RuntimeError)):
                self.invoke(module)
        self.assertEqual(self.calls, [])
        self.assertFalse(self.marker.exists())
