"""Quiesce retains exact manager exit evidence and always releases its references."""
import json

from deploy_restore_quiesce_refs_support import ReferenceFixture
from deploy_restore_quiesce_support import OPERATION, UNITS


class RestoreQuiesceReferenceTests(ReferenceFixture):
    def test_all_references_span_ordered_stop_exact_evidence_and_final_fsync(self):
        with self.controller() as module:
            result = self.invoke(module)
        self.assertEqual(result, {"operation_id": OPERATION, "target_sha256": self.target_digest, "state": "stopped"})
        self.assertEqual(self.lock_entries, 1)
        self.assert_released()
        self.assertIn(("stopped-sync-held",), self.reference_events)
        captured = json.loads(self.record.read_bytes())["units"]
        self.assertEqual(set(captured), set(UNITS))
        for unit in UNITS:
            pid = self.observed_pids[unit]
            self.assertEqual(captured[unit], {
                "manager": ":1.55",
                "process": {"pid": pid, "uid": self.target["root"]["uid"], "start_ticks": 12345,
                            "control_group": self.target["units"][unit]["control_group"]},
                "started_usec": pid * 1000,
                "exit": {"code": 1, "status": 0, "exited_usec": pid * 1000 + 500},
            })
            self.assertIn(("status", unit, 0), self.reference_events)
            self.assertIn(("status", unit, 1), self.reference_events)
        self.assert_preserved()

    def test_partial_reference_acquisition_releases_only_owned_refs_before_any_stop(self):
        self.refusal_at = UNITS[2]
        with self.controller() as module, self.assertRaises(RuntimeError) as error:
            self.invoke(module)
        self.assertNotIn(self.secret, str(error.exception))
        self.assertEqual(self.stops(), [])
        self.assert_released(count=2)

    def test_lost_or_mismatched_exit_identity_never_advances_to_database_stop(self):
        mutations = (
            lambda value: value.update(pid=0, code=0, started_usec=0, exited_usec=0),
            lambda value: value.update(pid=value["pid"] + 1),
            lambda value: value.update(started_usec=value["started_usec"] + 1),
            lambda value: value.update(exited_usec=0),
        )
        for change in mutations:
            fixture = ReferenceFixture()
            fixture.setUp()
            self.addCleanup(fixture.doCleanups)

            def changed(unit, value):
                if unit == UNITS[0] and fixture.units[unit]["MainPID"] == "0":
                    change(value)

            fixture.status_override = changed
            with self.subTest(change=change), fixture.controller() as module, self.assertRaises(RuntimeError):
                fixture.invoke(module)
            self.assertTrue(set(fixture.stops()).issubset(set(UNITS[:2])))
            self.assertEqual(json.loads(fixture.record.read_bytes())["state"], "closing")
            fixture.assert_released()

    def test_stop_deadline_manager_loss_and_fsync_failure_always_release_references(self):
        for failure in ("stop", "deadline", "manager", "fsync"):
            fixture = ReferenceFixture()
            fixture.setUp()
            self.addCleanup(fixture.doCleanups)
            if failure == "stop":
                def failed(_units):
                    raise RuntimeError(fixture.secret)
                fixture.after_stop = failed
            elif failure == "deadline":
                fixture.stop_cost = 241
            elif failure == "manager":
                fixture.after_stop = lambda _units: setattr(fixture, "owner_changed", True)
            else:
                fixture.fail_stopped_sync = True
            with self.subTest(failure=failure), fixture.controller() as module, self.assertRaises(RuntimeError) as error:
                fixture.invoke(module)
            self.assertNotIn(fixture.secret, str(error.exception))
            fixture.assert_released()
            if failure != "fsync":
                self.assertTrue(set(fixture.stops()).issubset(set(UNITS[:2])))
                self.assertEqual(json.loads(fixture.record.read_bytes())["state"], "closing")
            fixture.assert_preserved()

    def test_exact_saved_exit_survives_manager_gc_but_reentry_reobserves_absence(self):
        with self.controller() as module:
            first = self.invoke(module)
        evidence = json.loads(self.record.read_bytes())["units"]
        self.metadata_lost = True
        for row in self.units.values():
            row.update(ExecMainCode="0", ExecMainStatus="0")
        self.calls, self.events, self.preflight, self.reference_events = [], [], set(), []
        with self.controller() as module:
            second = self.invoke(module)
        self.assertEqual(second, first)
        self.assertEqual(self.stops(), [])
        self.assertEqual(json.loads(self.record.read_bytes())["units"], evidence)
        for unit in UNITS:
            self.assertIn(("processes", unit, 0), self.events)
        self.assertTrue(any(event[0] == "listeners" for event in self.events))
        self.assert_released()

    def test_inactive_without_captured_exit_is_not_normal_stop_evidence(self):
        self.metadata_lost = True
        for row in self.units.values():
            row.update(ActiveState="inactive", SubState="dead", MainPID="0", ExecMainCode="0", ExecMainStatus="0")
        with self.controller() as module, self.assertRaises(RuntimeError):
            self.invoke(module)
        self.assertEqual(self.stops(), [])
        self.assert_released()
