"""One locked controller installation preserves exact stop and publication evidence."""
import json

from deploy_controller_installation_support import InstallationFixture, UNITS
from deploy_controller_publication_support import inventory, put, tree


class ControllerInstallationTests(InstallationFixture):
    def test_ordered_durable_stop_gate_publication_and_approval_use_one_lock(self):
        with self.controller() as product:
            receipt = self.invoke(product)
        self.assertEqual(self.lock_entries, 1)
        self.assert_closed(receipt)
        kinds = [event[0] for event in self.events]
        self.assertLess(kinds.index("stop-durable"), kinds.index("mask"))
        self.assertLess(kinds.index("reload"), kinds.index("publish"))
        self.assertLess(kinds.index("publish"), kinds.index("approve"))
        self.assertEqual(sum(kind == "mask" for kind in kinds), 4)
        self.assertNotIn("start", kinds)
        stop = json.loads(self.stop_record.read_bytes())
        self.assertEqual(stop["state"], "stopped")
        for unit in UNITS:
            self.assertEqual(stop["units"][unit]["exit"]["code"], 1)
            self.assertEqual(stop["units"][unit]["exit"]["status"], 0)
        retained = tree(self.closed_record), tree(self.publication), tree(self.retained_cache)
        stop_count = len(self.stops())
        with self.controller() as product:
            self.assertEqual(self.invoke(product), receipt)
        self.assertEqual(len(self.stops()), stop_count)
        self.assertEqual((tree(self.closed_record), tree(self.publication), tree(self.retained_cache)), retained)
        before = tree(self.root)
        with self.controller() as product, self.assertRaises(RuntimeError) as error:
            self.invoke(product, expected_intent_sha256="0" * 64)
        self.assertNotIn(self.secret, str(error.exception))
        self.assertEqual(tree(self.root), before)
        with self.controller() as product, self.assertRaises(RuntimeError):
            self.invoke(product, operation="40000000-0000-4000-8000-000000000004")
        self.assertEqual(tree(self.root), before)
        self.assertFalse((self.root / "maintenance/restore").exists())

    def test_uncertain_stop_or_partial_gate_reentry_never_invents_normal_exit(self):
        for fault in ("stop_response", "mask", "restart"):
            fixture = InstallationFixture()
            fixture.setUp()
            self.addCleanup(fixture.doCleanups)
            fixture.fault = "mask" if fault == "restart" else fault
            with self.subTest(fault=fault), fixture.controller() as product:
                with self.assertRaises(RuntimeError) as error:
                    fixture.invoke(product)
            self.assertTrue(fixture.fault_reached)
            self.assertNotIn(fixture.secret, str(error.exception))
            self.assertFalse(fixture.references_active)
            self.assertEqual(fixture.python_inventory(fixture.root / "tools"), fixture.previous_files)
            self.assertEqual(tree(fixture.cache), fixture.cache_before)
            fixture.fault = None
            stopped = len(fixture.stops())
            stop_before = tree(fixture.stop_record)
            if fault == "stop_response":
                fixture.metadata_lost = True
                for row in fixture.units.values():
                    if row["ActiveState"] == "inactive":
                        row.update(ExecMainCode="0", ExecMainStatus="0")
            elif fault == "restart":
                fixture.units[UNITS[-1]].update(ActiveState="active", SubState="running", MainPID="9191",
                                                ExecMainCode="0", ExecMainStatus="0")
            with fixture.controller() as product:
                if fault == "mask":
                    result = fixture.invoke(product)
                    fixture.assert_closed(result)
                else:
                    with self.assertRaises(RuntimeError):
                        fixture.invoke(product)
                    self.assertFalse(fixture.publication.exists())
                    self.assertEqual(fixture.preserved(), fixture.protected)
            self.assertEqual(len(fixture.stops()), stopped)
            self.assertEqual(tree(fixture.stop_record), stop_before)
            self.assertFalse((fixture.root / "maintenance/restore").exists())

    def test_cache_and_source_exchange_lost_acknowledgements_reconcile_exact_orientation(self):
        for fault in ("cache_sync", "publication", "foreign_cache"):
            fixture = InstallationFixture()
            fixture.setUp()
            self.addCleanup(fixture.doCleanups)
            fixture.fault = fault
            if fault == "foreign_cache":
                put(fixture.retained_cache / "foreign.pyc", b"unrelated retained object\n")
                foreign = tree(fixture.retained_cache)
            with self.subTest(fault=fault), fixture.controller() as product, self.assertRaises(RuntimeError):
                fixture.invoke(product)
            if fault == "foreign_cache":
                self.assertEqual(tree(fixture.retained_cache), foreign)
                self.assertEqual(tree(fixture.cache), fixture.cache_before)
                self.assertEqual(fixture.stops(), [])
                continue
            self.assertTrue(fixture.fault_reached)
            self.assertEqual(tree(fixture.retained_cache), fixture.cache_before)
            self.assertFalse(fixture.cache.exists())
            retained = tree(fixture.retained_cache)
            if fault == "publication":
                self.assertEqual(inventory(fixture.root / "tools"), fixture.candidate_files)
                previous = tree(fixture.publication / "exchange")
            fixture.fault = None
            with fixture.controller() as product:
                result = fixture.invoke(product)
            fixture.assert_closed(result)
            self.assertEqual(tree(fixture.retained_cache), retained)
            self.assertEqual(sum(event[0] == "publish" for event in fixture.events), 1)
            if fault == "publication":
                self.assertEqual(tree(fixture.publication / "exchange"), previous)
            self.assertEqual(fixture.preserved(), fixture.protected)
