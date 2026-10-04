"""Externally approved unit candidates are reopened only with exact durable authority."""
import json

from deploy_controller_installation_support import InstallationFixture, UNITS
from deploy_controller_publication_support import digest, encoded, inventory, put, tree


class ControllerInstallationReopenTests(InstallationFixture):
    def test_predecessor_cas_preflight_and_uncertain_approval_keep_exact_generation(self):
        self.bootstrap_approval()
        predecessor = self.intent["previous_approval_sha256"]
        self.intent["previous_approval_sha256"] = "0" * 64
        self.pin_intent()
        before = tree(self.root)
        with self.controller() as product, self.assertRaises(RuntimeError):
            self.invoke(product)
        self.assertEqual(self.stops(), [])
        self.assertEqual(tree(self.root), before)
        self.intent["previous_approval_sha256"] = predecessor
        self.pin_intent()
        self.fault = "approval"
        with self.controller() as product, self.assertRaises(RuntimeError) as error:
            self.invoke(product)
        self.assertTrue(self.fault_reached)
        self.assertNotIn(self.secret, str(error.exception))
        history = tree(self.publication / "approval.json")
        current = tree(self.current_approval)
        self.assertEqual(json.loads(self.current_approval.read_bytes())["previous_approval_sha256"], predecessor)
        self.assertEqual(inventory(self.root / "tools"), self.candidate_files)
        self.fault = None
        with self.controller() as product:
            result = self.invoke(product)
        self.assert_closed(result)
        self.assertEqual(tree(self.publication / "approval.json"), history)
        self.assertEqual(tree(self.current_approval), current)
        self.assertEqual(sum(event[0] == "publish" for event in self.events), 1)

    def test_legacy_units_and_launcher_are_preflighted_and_render_the_literal_approved_generation(self):
        for fault in ("foreign_launcher", "unapproved_prestart_absence", "changed_cache", "unknown_command"):
            fixture = InstallationFixture()
            fixture.setUp()
            self.addCleanup(fixture.doCleanups)
            if fault == "foreign_launcher":
                put(fixture.launcher, b"unrelated launcher must survive\n")
            elif fault == "unapproved_prestart_absence":
                fixture.intent["legacy_absent_prestarts"] = []
                fixture.pin_intent()
            elif fault == "changed_cache":
                put(fixture.cache / "foreign.pyc", b"cache added after approval\n")
            else:
                path = fixture.units_directory / UNITS[1]
                put(path, fixture.original_units[UNITS[1]].replace(b"[Service]\n", b"[Service]\nExecStop=/bin/true\n"))
                from deploy_restore_quiesce_support import fingerprint
                fixture.target["units"][UNITS[1]]["fragment"] = fingerprint(path)
                fixture.pin_target()
                fixture.intent["target"] = fingerprint(fixture.target_path)
                fixture.pin_intent()
            before = tree(fixture.root), tree(fixture.home)
            with self.subTest(fault=fault), fixture.controller() as product, self.assertRaises(RuntimeError):
                fixture.invoke(product)
            self.assertEqual(fixture.stops(), [])
            self.assertEqual((tree(fixture.root), tree(fixture.home)), before)
        (self.root / "current").unlink()
        (self.root / "current").symlink_to(self.release)
        self.intent["release"]["link"] = str(self.release)
        self.pin_intent()
        self.protected = self.preserved()
        with self.controller() as product:
            receipt = self.invoke(product)
        self.assert_closed(receipt)
        for unit in UNITS:
            actual = (self.candidates / unit).read_bytes()
            self.assertIn(self.candidate_sha.encode(), actual)
            self.assertIn(b"CPUQuota=175%\n", actual)
            self.assertTrue(actual.endswith(b"# preserved private unit tail\n"))
            self.assertEqual(actual.count(b"ExecStartPre="), 0 if unit == UNITS[1] else 1)
        self.run_candidate()

    def test_reopening_requires_exact_authority_and_reconciles_partial_reload_and_readiness(self):
        for fault in ("reopen", "reopen_reload", "readiness", "readiness_deadline"):
            fixture = InstallationFixture()
            fixture.setUp()
            self.addCleanup(fixture.doCleanups)
            with fixture.controller() as product:
                closed = fixture.invoke(product)
                self.assertEqual(fixture.invoke(product), closed)
            self.assertFalse(any(event[0] == "start" for event in fixture.events))
            fixture.authorize(closed)
            wrong = json.loads(fixture.authority_path.read_bytes())
            wrong["closed_sha256"] = "0" * 64
            put(fixture.authority_path, encoded(wrong))
            before = tree(fixture.units_directory)
            with fixture.controller() as product, self.assertRaises(RuntimeError):
                fixture.invoke(product, reopen=True)
            self.assertEqual(tree(fixture.units_directory), before)
            self.assertFalse(any(event[0] == "start" for event in fixture.events))
            fixture.authorize(closed)
            fixture.fault = fault
            with self.subTest(fault=fault), fixture.controller() as product, self.assertRaises(RuntimeError) as error:
                fixture.invoke(product, reopen=True)
            self.assertTrue(fixture.fault_reached)
            self.assertNotIn(fixture.secret, str(error.exception))
            self.assertNotEqual(json.loads(fixture.intent_record.read_bytes())["state"], "installed")
            after_failure = tree(fixture.units_directory)
            with fixture.controller() as product, self.assertRaises(RuntimeError):
                fixture.invoke(product)
            self.assertEqual(tree(fixture.units_directory), after_failure)
            fixture.fault = None
            with fixture.controller() as product:
                installed = fixture.invoke(product, reopen=True)
            self.assertEqual(installed, {**closed, "state": "installed"})
            self.assertEqual(digest(fixture.closed_record.read_bytes()), closed["closed_sha256"])
            self.assertEqual(fixture.preserved(), fixture.protected)
            self.assertEqual(tree(fixture.retained_cache), fixture.cache_before)
            self.assertFalse((fixture.root / "maintenance/restore").exists())
            for unit in UNITS:
                self.assertEqual((fixture.units_directory / unit).read_bytes(), fixture.expected_unit(unit))
                self.assertEqual((fixture.operation / "slots" / unit).read_bytes(), fixture.original_units[unit])
                self.assertEqual(sum(event == ("start", unit) for event in fixture.events), 1)
            starts = [event[1] for event in fixture.events if event[0] == "start"]
            self.assertLess(max(starts.index(unit) for unit in UNITS[2:]), starts.index(UNITS[1]))
            self.assertLess(starts.index(UNITS[1]), starts.index(UNITS[0]))
