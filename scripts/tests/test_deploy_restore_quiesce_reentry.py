"""Uncertain stop completion never replaces the target or opens admission."""
import json
from unittest.mock import patch

from deploy_restore_quiesce_support import OPERATION, OTHER, UNITS, QuiesceFixture, fingerprint, journal


class RestoreQuiesceReentryTests(QuiesceFixture):
    def test_partial_stop_reenters_same_target_and_reobserves_before_finishing(self):
        def interrupted(units):
            if UNITS[0] in units or UNITS[1] in units:
                raise RuntimeError(self.secret)
        self.after_stop = interrupted
        with self.controller() as module:
            with self.assertRaises(RuntimeError) as error:
                self.invoke(module)
            self.assertNotIn(self.secret, str(error.exception))
        self.assertEqual(json.loads(self.record.read_bytes())["state"], "closing")
        self.assertTrue(set(self.stops()).issubset(set(UNITS[:2])))
        self.assert_preserved()
        target_before, fence_before = self.target_path.read_bytes(), self.marker.read_bytes()
        self.after_stop = None
        self.calls, self.events, self.preflight = [], [], set()
        with self.controller() as module:
            result = self.invoke(module)
        self.assertEqual(result["state"], "stopped")
        self.assertEqual(self.preflight, set(UNITS))
        self.assertTrue(set(UNITS[2:]).issubset(set(self.stops())))
        self.assertEqual(self.target_path.read_bytes(), target_before)
        self.assertEqual(self.marker.read_bytes(), fence_before)
        self.assert_preserved()

    def test_stopped_receipt_always_reobserves_and_closes_restarted_owned_service(self):
        with self.controller() as module:
            self.invoke(module)
        self.units[UNITS[0]].update(ActiveState="active", SubState="running", MainPID="5100",
                                   ExecMainCode="0", ExecMainStatus="0")
        self.calls, self.events, self.preflight = [], [], set()
        with self.controller() as module:
            result = self.invoke(module)
        self.assertEqual(result["operation_id"], OPERATION)
        self.assertEqual(self.preflight, set(UNITS))
        self.assertIn(UNITS[0], self.stops())
        self.assertIn(("processes", UNITS[0], 0), self.events)
        self.assert_preserved()

    def test_other_operation_or_changed_target_cannot_adopt_existing_journal(self):
        with self.controller() as module:
            self.invoke(module)
        original = self.record.read_bytes()
        self.calls, self.events = [], []
        with self.controller() as module:
            with self.assertRaises((ValueError, RuntimeError)):
                self.invoke(module, operation=OTHER)
            self.settings["api_port"] += 1
            self.private_json(self.config, self.settings)
            self.target["settings"] = fingerprint(self.config)
            self.target["ports"][UNITS[1]] = self.settings["api_port"]
            self.pin_target()
            with self.assertRaises((ValueError, RuntimeError)):
                self.invoke(module)
        self.assertEqual(self.stops(), [])
        self.assertEqual(self.record.read_bytes(), original)
        self.assertEqual(json.loads(self.marker.read_bytes()), {"operation_id": OPERATION})

    def test_inactive_unit_with_surviving_process_or_listener_keeps_databases_running(self):
        for survivor in ("process", "listener"):
            fixture = QuiesceFixture()
            fixture.setUp()
            self.addCleanup(fixture.doCleanups)
            if survivor == "process":
                fixture.leftovers[UNITS[0]] = [{"pid": 9191, "uid": fixture.target["root"]["uid"],
                    "start_ticks": 8080, "control_group": fixture.units[UNITS[0]]["ControlGroup"]}]
            else:
                fixture.foreign_ports.add(fixture.settings["web_port"])
            with self.subTest(survivor=survivor), fixture.controller() as module:
                with self.assertRaises((ValueError, RuntimeError)):
                    fixture.invoke(module)
            self.assertTrue(set(fixture.stops()).issubset(set(UNITS[:2])))
            self.assertEqual(json.loads(fixture.record.read_bytes())["state"], "closing")
            fixture.assert_preserved()

    def test_uncertain_final_directory_sync_does_not_return_success_and_retry_rechecks(self):
        def interrupted(directory):
            self.sync(directory)
            if self.record.exists() and json.loads(self.record.read_bytes())["state"] == "stopped":
                raise OSError(self.secret)
        with self.controller() as module:
            with patch.object(journal, "sync_directory", side_effect=interrupted):
                with self.assertRaises(RuntimeError) as error:
                    self.invoke(module)
                self.assertNotIn(self.secret, str(error.exception))
        self.assertEqual(json.loads(self.record.read_bytes())["state"], "stopped")
        self.assert_preserved()
        self.calls, self.events, self.preflight = [], [], set()
        with self.controller() as module:
            result = self.invoke(module)
        self.assertEqual(result["state"], "stopped")
        self.assertEqual(self.preflight, set(UNITS))
        self.assertIn(("sync", self.operation, "stopped"), self.events)
        self.assert_preserved()
