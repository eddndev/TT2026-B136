"""Two complete installations preserve pinned predecessor units and reentry."""
import json
from pathlib import Path

from deploy_controller_installation_support import InstallationFixture, UNITS, owned
from deploy_controller_publication_support import digest, encoded, inventory, put, tree
from deploy_restore_quiesce_support import fingerprint


UPGRADE = "40000000-0000-4000-8000-000000000004"


class ControllerInstallationUpgradeTests(InstallationFixture):
    def rename(self, source, destination, flags, check):
        source, destination = Path(source), Path(destination)
        unit = next((path for path in (source, destination)
                     if path.parent == self.units_directory), None)
        if self.intent["cache"] is None and unit is not None and not unit.is_symlink():
            # The first installation retained the only legacy bytecode cache.
            self.assertEqual(flags, 2)
            self.assertTrue(self.stop_durable)
            self.assertEqual(self.python_inventory(self.root / "tools"), self.previous_files)
            self.assertFalse(self.cache.exists())
            result = self.real_rename(source, destination, flags, check)
            self.units[unit.name]["NeedDaemonReload"] = "yes"
            self.events.append(("mask", unit.name))
            return result
        return super().rename(source, destination, flags, check)

    def authorize(self, receipt):
        put(self.authority_path, encoded({
            "format": "qadra-controller-reopen", "version": 1,
            "operation_id": receipt["operation_id"], "root": self.intent["root"],
            "intent_sha256": self.intent_sha, "closed_sha256": receipt["closed_sha256"],
        }))

    def prepare_upgrade(self, installed_units, predecessor):
        self.original_units = installed_units
        self.previous_files = inventory(self.root / "tools")
        self.stage = self.base / "upgrade-candidate"
        self.generation(self.stage / "sources", "C")
        self.candidate_files = inventory(self.stage / "sources")
        self.candidate_sha = digest(encoded(self.candidate_files))
        put(self.stage / "manifest.json", encoded({
            "format": "qadra-controller-publication", "version": 1,
            "source_revision": "c" * 40, "files": self.candidate_files,
        }))
        self.operation = self.root / "maintenance/controller-installation" / UPGRADE
        self.publication = self.root / "maintenance/controllers" / UPGRADE
        self.stop_record = self.operation / "stop.json"
        self.intent_record = self.operation / "installation.json"
        self.closed_record = self.operation / "closed.json"
        self.reopen_record = self.operation / "reopen.json"
        self.retained_cache = self.operation / "retained-cache"
        self.candidates = self.operation / "candidates"
        self.record = self.stop_record
        self.target_path = self.root / "receipts/upgrade-target.json"
        self.target["target_id"] = UPGRADE
        for unit in UNITS:
            self.target["units"][unit]["fragment"] = fingerprint(self.units_directory / unit)
        self.pin_target()
        self.intent_path = self.root / "receipts/upgrade-intent.json"
        self.authority_path = self.root / "receipts/upgrade-reopen.json"
        self.intent = {**self.intent, "operation_id": UPGRADE,
            "target": fingerprint(self.target_path),
            "previous_sources": {"identity": owned(self.root / "tools"),
                                 "sha256": digest(encoded(self.previous_files))},
            "cache": None, "legacy_absent_prestarts": [],
            "previous_approval_sha256": predecessor,
            "candidate": {"path": str(self.stage),
                          "manifest_sha256": digest((self.stage / "manifest.json").read_bytes()),
                          "installed_sha256": self.candidate_sha}}
        self.pin_intent()
        self.stop_durable = False
        self.preflight.clear()

    def test_second_installation_reopens_pinned_units_and_reentry_preserves_both_generations(self):
        with self.controller() as product:
            first_closed = self.invoke(product)
        self.assert_closed(first_closed)
        self.authorize(first_closed)
        with self.controller() as product:
            first_installed = self.invoke(product, reopen=True)
        self.assertEqual(first_installed, {**first_closed, "state": "installed"})
        first_paths = (self.operation, self.publication)
        first_evidence = [tree(path) for path in first_paths]
        installed_units = {unit: (self.units_directory / unit).read_bytes() for unit in UNITS}
        previous_pin = self.candidate_sha
        predecessor = digest(self.current_approval.read_bytes())
        self.assertFalse(self.cache.exists())
        self.prepare_upgrade(installed_units, predecessor)
        self.assertNotEqual(previous_pin, self.candidate_sha)
        expected_units = {}
        for unit, raw in installed_units.items():
            self.assertEqual(raw.count(previous_pin.encode()), 1)
            expected_units[unit] = raw.replace(previous_pin.encode(), self.candidate_sha.encode())

        with self.controller() as product:
            second_closed = self.invoke(product, operation=UPGRADE)
        self.assertEqual(second_closed["operation_id"], UPGRADE)
        self.assertEqual(second_closed["state"], "installed_closed")
        self.assertEqual(inventory(self.root / "tools"), self.candidate_files)
        self.assertFalse(self.cache.exists())
        self.assertFalse(self.retained_cache.exists())
        for unit in UNITS:
            self.assertTrue((self.units_directory / unit).is_symlink())
            self.assertEqual((self.operation / "slots" / unit).read_bytes(), installed_units[unit])
            self.assertEqual((self.candidates / unit).read_bytes(), expected_units[unit])
        self.authorize(second_closed)
        with self.controller() as product:
            second_installed = self.invoke(product, operation=UPGRADE, reopen=True)
        self.assertEqual(second_installed, {**second_closed, "state": "installed"})
        self.assertEqual(json.loads(self.current_approval.read_bytes())["previous_approval_sha256"], predecessor)
        for unit in UNITS:
            self.assertEqual((self.units_directory / unit).read_bytes(), expected_units[unit])
            self.assertEqual(self.units[unit]["ActiveState"], "active")
        self.assertEqual([tree(path) for path in first_paths], first_evidence)
        self.assertEqual(self.preserved(), self.protected)
        self.assertEqual(sum(event[0] == "publish" for event in self.events), 2)
        self.assertEqual(sum(event[0] == "start" for event in self.events), 8)

        before = tree(self.root), tree(self.home)
        changes = {"stop", "mask", "publish", "approve", "reopen", "start"}
        events = [event for event in self.events if event[0] in changes]
        with self.controller() as product:
            repeated = self.invoke(product, operation=UPGRADE, reopen=True)
        self.assertEqual(repeated, second_installed)
        self.assertEqual((tree(self.root), tree(self.home)), before)
        self.assertEqual([event for event in self.events if event[0] in changes], events)
        self.assertFalse(self.references_active)
