"""Crashes and uncertain manager acknowledgements never become a gate receipt."""
import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

from deploy_controller_gate_support import GateFixture, OTHER, REJECTIONS, UNITS, journal, node
from deploy_controller_launcher_support import close_child


CHILD = r'''
import os
from pathlib import Path
import sys

source, base = sys.argv[1:]
sys.path.insert(0, source)
from deploy_controller_gate_support import GateFixture

fixture = GateFixture()
fixture.adopt(Path(base))
count = 0

def crash(unit):
    global count
    count += 1
    if count == 2:
        os.write(1, b"exchanged=2\n")
        os._exit(86)

fixture.after_exchange = crash
with fixture.controller() as module:
    fixture.invoke(module)
raise RuntimeError("fixture missed partial gate exchange")
'''


class ControllerGateReentryTests(GateFixture):
    def test_process_death_after_two_exchanges_resumes_exact_operation_and_retained_inodes(self):
        command = [sys.executable, "-I", "-B", "-S", "-u", "-c", CHILD,
                   str(Path(__file__).resolve().parent), str(self.base)]
        child = subprocess.Popen(command, cwd=self.base, stdout=subprocess.PIPE,
                                 stderr=subprocess.PIPE, text=True, encoding="ascii")
        try:
            output, error = child.communicate(timeout=5)
            self.assertEqual((child.returncode, output, error), (86, "exchanged=2\n", ""))
        finally:
            close_child(child)
        self.assertEqual(self.record_value()["state"], "masking")
        masked = {unit for unit in UNITS if (self.units_directory / unit).is_symlink()}
        self.assertEqual(len(masked), 2)
        for unit in UNITS:
            original = self.slots / unit if unit in masked else self.units_directory / unit
            self.assertEqual(node(original), self.originals[unit])
        masks = {unit: node(self.units_directory / unit) for unit in masked}
        self.assertEqual(json.loads(self.manager_path.read_bytes()), self.units)
        before = self.snapshot()
        for arguments in ({"operation": OTHER}, {"expected_target_sha256": "0" * 64}):
            with self.controller() as module, self.assertRaises(REJECTIONS):
                self.invoke(module, **arguments)
            self.assertEqual(self.snapshot(), before)
        self.reset_observations()
        with self.controller() as module:
            result = self.invoke(module)
        self.assert_gated(result)
        self.assertEqual({event[1] for event in self.events if event[0] == "exchange"}, set(UNITS) - masked)
        self.assertEqual({unit: node(self.units_directory / unit) for unit in masked}, masks)

    def test_reload_failure_lost_acknowledgement_or_deadline_cannot_return_success(self):
        for behavior in ("before", "after", "late"):
            fixture = GateFixture()
            fixture.setUp()
            self.addCleanup(fixture.doCleanups)
            fixture.reload_behavior = behavior
            with self.subTest(behavior=behavior), fixture.controller() as module:
                with self.assertRaises(REJECTIONS) as failure:
                    fixture.invoke(module)
                self.assertNotIn(fixture.secret, str(failure.exception))
            self.assertEqual(fixture.record_value()["state"], "reload_pending")
            fixture.assert_masks()
            self.assertEqual(sum(event[0] == "reload" for event in fixture.events), 1)
            masks = {unit: node(fixture.units_directory / unit) for unit in UNITS}
            fixture.reload_behavior = None
            fixture.reset_observations()
            with fixture.controller() as module:
                result = fixture.invoke(module)
            fixture.assert_gated(result)
            self.assertFalse(any(event[0] == "exchange" for event in fixture.events))
            self.assertEqual(sum(event[0] == "reload" for event in fixture.events), 1)
            self.assertEqual({unit: node(fixture.units_directory / unit) for unit in UNITS}, masks)

    def test_uncertain_final_sync_requires_fresh_complete_manager_observation_on_reentry(self):
        def lose_sync(directory):
            self.sync_gate(directory)
            if Path(directory) == self.operation and self.record.exists() and self.record_value()["state"] == "gated":
                raise OSError(self.secret)

        with self.controller() as module:
            with patch.object(module, "sync_directory", side_effect=lose_sync), \
                    patch.object(journal, "sync_directory", side_effect=lose_sync):
                with self.assertRaises(REJECTIONS) as failure:
                    self.invoke(module)
                self.assertNotIn(self.secret, str(failure.exception))
        self.assertEqual(self.record_value()["state"], "gated")
        self.assert_masks()
        files = {unit: (node(self.units_directory / unit), node(self.slots / unit)) for unit in UNITS}
        self.reset_observations()

        def stale_manager(unit, row):
            if unit == UNITS[0] and ("reload",) in self.events:
                row.update(LoadState="loaded", UnitFileState="enabled")

        self.show_override = stale_manager
        with self.controller() as module:
            with self.assertRaises(REJECTIONS):
                self.invoke(module)
        self.assertTrue(any(event[0] == "show" for event in self.events))
        self.assertEqual(sum(event[0] == "reload" for event in self.events), 1)
        self.assertFalse(any(event[0] == "exchange" for event in self.events))
        self.assertEqual({unit: (node(self.units_directory / unit), node(self.slots / unit)) for unit in UNITS}, files)
        self.show_override = None
        self.reset_observations()
        with self.controller() as module:
            result = self.invoke(module)
        self.assert_gated(result)
        self.assertEqual(sum(event[0] == "reload" for event in self.events), 1)
        self.assertTrue(any(event[0] == "show" for event in self.events[self.events.index(("reload",)) + 1:]))
        self.assertEqual({unit: (node(self.units_directory / unit), node(self.slots / unit)) for unit in UNITS}, files)


if __name__ == "__main__":
    unittest.main()
