"""Persistent entry masks preserve originals and reject unowned filesystem state."""
import unittest

from deploy_controller_gate_support import GateFixture, REJECTIONS, UNITS, journal, node, put, tree


class ControllerGateTests(GateFixture):
    def test_durable_gate_preserves_originals_and_reloads_again_without_stopping_processes(self):
        with self.controller() as module:
            result = self.invoke(module)
        self.assert_gated(result)
        self.assertEqual(self.lock_entries, 1)
        self.assertEqual({event[1] for event in self.events if event[0] == "exchange"}, set(UNITS))
        self.assertEqual(sum(event[0] == "exchange" for event in self.events), 4)
        self.assertEqual(sum(event[0] == "reload" for event in self.events), 1)
        self.assertTrue(any(event[0] == "show" for event in self.events[self.events.index(("reload",)) + 1:]))
        files = {unit: (node(self.units_directory / unit), node(self.slots / unit)) for unit in UNITS}
        self.reset_observations()
        with self.controller() as module:
            result = self.invoke(module)
        self.assert_gated(result)
        self.assertEqual(self.lock_entries, 2)
        self.assertFalse(any(event[0] == "exchange" for event in self.events))
        self.assertEqual(sum(event[0] == "reload" for event in self.events), 1)
        self.assertTrue(any(event[0] == "show" for event in self.events[self.events.index(("reload",)) + 1:]))
        self.assertEqual({unit: (node(self.units_directory / unit), node(self.slots / unit)) for unit in UNITS}, files)

    def test_preflight_rejects_dropins_substitutions_foreign_masks_and_busy_lock_without_writes(self):
        for fault in ("disk-dropin", "loaded-dropin", "fragment", "foreign-mask", "wrong-pin", "busy"):
            fixture = GateFixture()
            fixture.setUp()
            self.addCleanup(fixture.doCleanups)
            fragment = fixture.units_directory / UNITS[1]
            if fault == "disk-dropin":
                put(fixture.units_directory / (UNITS[1] + ".d/override.conf"), b"[Service]\nRestart=always\n")
            elif fault == "loaded-dropin":
                fixture.units[UNITS[1]]["DropInPaths"] = "/foreign/override.conf"
            elif fault == "fragment":
                fragment.write_bytes(fragment.read_bytes() + b"# changed fragment\n")
            elif fault == "foreign-mask":
                fragment.rename(fixture.base / "foreign-original")
                fragment.symlink_to("/dev/null")
            before = fixture.snapshot()
            with self.subTest(fault=fault):
                if fault == "busy":
                    with journal.locked(fixture.root), fixture.controller() as module:
                        with self.assertRaises(REJECTIONS):
                            fixture.invoke(module)
                else:
                    with fixture.controller() as module:
                        arguments = {"expected_target_sha256": "0" * 64} if fault == "wrong-pin" else {}
                        with self.assertRaises(REJECTIONS):
                            fixture.invoke(module, **arguments)
                self.assertEqual(fixture.snapshot(), before)
                self.assertFalse(fixture.installation.exists())
                self.assertFalse(any(event[0] in ("exchange", "reload") for event in fixture.events))

    def test_reentry_rejects_foreign_mask_slot_or_unit_directory_without_removing_it(self):
        for fault in ("mask", "slot", "directory"):
            fixture = GateFixture()
            fixture.setUp()
            self.addCleanup(fixture.doCleanups)
            fixture.reload_behavior = "before"
            with fixture.controller() as module:
                with self.assertRaises(REJECTIONS):
                    fixture.invoke(module)
            fixture.assert_masks()
            fixture.reload_behavior = None
            fixture.reset_observations()
            if fault == "mask":
                path = fixture.units_directory / UNITS[0]
                path.rename(fixture.base / "retained-owned-mask")
                path.symlink_to("/dev/null")
            elif fault == "slot":
                path = fixture.slots / UNITS[0]
                path.rename(fixture.base / "retained-original")
                path.symlink_to(fixture.base / "retained-original")
            else:
                previous = fixture.base / "previous-unit-directory"
                fixture.units_directory.rename(previous)
                fixture.units_directory.mkdir(mode=0o700)
                for path in previous.iterdir():
                    path.rename(fixture.units_directory / path.name)
            before = tree(fixture.base)
            with self.subTest(fault=fault), fixture.controller() as module:
                with self.assertRaises(REJECTIONS):
                    fixture.invoke(module)
            self.assertEqual(tree(fixture.base), before)
            self.assertFalse(any(event[0] in ("exchange", "reload") for event in fixture.events))


if __name__ == "__main__":
    unittest.main()
