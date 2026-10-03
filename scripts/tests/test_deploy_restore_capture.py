"""Capture real boundary responses into the existing pinned admission format."""
import copy
import json
import os
import stat
import unittest
from unittest.mock import patch

from deploy_restore_capture_support import CaptureFixture, fingerprint
import crl_journal
import restore_admission


class RestoreCaptureTests(CaptureFixture):
    def test_capture_binds_valkey_facts_full_audit_and_exact_existing_inventory(self):
        for empty in (False, True):
            with self.subTest(empty_audit=empty):
                if empty:
                    self.audit = []
                    self.descriptor_path.unlink()
                result = self.invoke_capture()
                self.assertEqual(set(result), {"backup", "descriptor_sha256"})
                descriptor = json.loads(self.descriptor_path.read_bytes())
                self.assertEqual(stat.S_IMODE(self.descriptor_path.stat().st_mode), 0o600)
                self.assertEqual(result["descriptor_sha256"], fingerprint(
                    self.descriptor_path.read_bytes())["sha256"])
                facts = self.expected_facts()
                self.assertEqual(descriptor["postgres"], facts["postgres"])
                self.assertEqual(descriptor["redis"], facts["redis"])
                expected = None if empty else {"sequence": 0, "head": "a" * 64}
                admitted = restore_admission.admit(
                    self.root, result["backup"], self.descriptor_path,
                    expected_descriptor_sha256=result["descriptor_sha256"],
                    controller_directory=self.controller, observed=facts)
                self.assertEqual(admitted["audit_predecessor"], expected)
                self.assertTrue(self.verified_exports, "capture skipped audit verification")
                self.assertTrue(all(not path.exists() for path in self.verified_exports))
                self.assertFalse(self.marker.exists(), "capture changed the restore fence")

    def test_unsafe_runtime_target_or_unrepresented_facts_reject_before_capture(self):
        original = copy.deepcopy(self.catalog)
        cases = [
            ("other-client", lambda: self.catalog.update(other_clients=[{"pid": 777, "role": "qadra_runtime"}])),
            ("role-setting", lambda: self.catalog.update(role_settings=[{"role": "qadra_runtime", "settings": ["search_path=other"]}])),
            ("database-identity", lambda: self.catalog.update(database="other")),
            ("runtime-authority", lambda: self.catalog["roles"]["qadra_runtime"].update(superuser=True)),
            ("engine-identity", lambda: self.redis.info_override.update(server_name="redis")),
            ("catalog-size", lambda: setattr(self, "catalog_prefix", " " * (64 * 1024 + 1))),
        ]
        for name, change in cases:
            with self.subTest(reason=name):
                self.catalog = copy.deepcopy(original)
                self.redis.info_override = {}
                self.catalog_prefix = ""
                change()
                before = self.snapshot()
                self.rejected_capture()
                self.assertEqual(self.backups_started, 0)
                self.assertFalse(self.descriptor_path.exists())
                after = self.snapshot()
                # Acquiring the existing deployment lock may create only its own file.
                before.pop("deploy.lock", None)
                after.pop("deploy.lock", None)
                self.assertTrue(after == before, "preflight rejection changed deployment state")

    def test_source_drift_after_dump_retains_unqualified_backup_without_descriptor(self):
        for name in ("audit", "role"):
            with self.subTest(changed=name):
                audit, catalog = copy.deepcopy(self.audit), copy.deepcopy(self.catalog)
                def change():
                    if name == "audit":
                        self.audit.append({**self.audit[-1], "sequence": 1, "chain": "b" * 64})
                    else:
                        self.catalog["roles"]["qadra_admin"]["connection_limit"] = 17
                self.after_dump = change
                self.rejected_capture()
                self.assertFalse(self.descriptor_path.exists())
                self.assertEqual(len(list((self.root / "backups").glob("*/COMPLETE"))),
                                 self.backups_started, "complete but unqualified capture was removed")
                self.audit, self.catalog = audit, catalog
                with crl_journal.locked(self.root):
                    pass

    def test_bad_verifier_receipt_existing_destination_and_uncertain_fsync_never_succeed(self):
        for valid, delta in ((False, 0), (True, 1)):
            with self.subTest(verifier_valid=valid, wrong_count=delta != 0):
                self.audit_valid, self.audit_count_delta = valid, delta
                self.rejected_capture()
                self.assertFalse(self.descriptor_path.exists())
                self.assertEqual(self.backups_started, 0)
        self.audit_valid, self.audit_count_delta = True, 0
        previous = b"separately retained capture receipt\n"
        self.descriptor_path.write_bytes(previous)
        self.descriptor_path.chmod(0o600)
        self.rejected_capture()
        self.assertTrue(self.descriptor_path.read_bytes() == previous, "previous receipt was replaced")
        self.assertEqual(self.backups_started, 0)
        self.descriptor_path.unlink()
        native_fsync = os.fsync
        parent = self.descriptor_path.parent.stat()
        reached = []
        def fail_receipt_directory(descriptor):
            current = os.fstat(descriptor)
            if (current.st_dev, current.st_ino) == (parent.st_dev, parent.st_ino):
                reached.append(True)
                raise OSError("private-fsync-fixture-sentinel")
            return native_fsync(descriptor)
        with patch.object(os, "fsync", side_effect=fail_receipt_directory):
            self.rejected_capture()
        self.assertTrue(reached, "receipt directory was never durably published")


if __name__ == "__main__":
    unittest.main()
