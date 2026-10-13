"""A CRL pre-change backup restores the original state without a dangling fence."""
import copy
import json
from pathlib import Path
import subprocess
import tarfile
import unittest
from unittest.mock import patch

from test_deploy_crl_support import CrlFixture, renew_crl
import runtime


class CrlBackupSnapshotTests(CrlFixture):
    def setUp(self):
        super().setUp()
        (self.root / "backups").mkdir(mode=0o700)
        self.config = {"postgres_port": 15486, "redis_port": 16386,
                       "admin_password": "fixture-admin", "runtime_password": "fixture-user",
                       "redis_password": "fixture-redis", "kek": "fixture-kek"}
        settings = self.root / "config/settings.json"
        settings.write_text(json.dumps(self.config))
        settings.chmod(0o600)
        self.native_backup = runtime.Runtime(self.root)
        self.fail_capture = False
        self.captured_head = None
        self.backup_path = None
        self.runtime.backup = self.capture

    def external(self, arguments, **kwargs):
        arguments = [str(value) for value in arguments]
        if arguments[0] == "systemctl":
            if "--property=MainPID" in arguments:
                output = "123\n"
            else:
                output = "active\nactive\n" if self.runtime.running else "inactive\ninactive\n"
            return subprocess.CompletedProcess(arguments, 0, stdout=output)
        if arguments[0] == "pg_dump":
            self.captured_head = copy.deepcopy(self.head)
            Path(arguments[arguments.index("--file") + 1]).write_bytes(b"synthetic SQL snapshot")
        elif arguments[0] == "redis-cli":
            if "INFO" in arguments:
                return subprocess.CompletedProcess(arguments, 0, stdout="process_id:123\r\n")
            if "CONFIG" in arguments:
                return subprocess.CompletedProcess(arguments, 0, stdout=f"dir\n{self.root / 'data'}\n")
            Path(arguments[arguments.index("--rdb") + 1]).write_bytes(b"synthetic Redis snapshot")
        elif arguments[0] != "redis-check-rdb":
            self.fail("unexpected external command")
        return subprocess.CompletedProcess(arguments, 0, stdout=b"")

    def capture(self, *, ca_operation=None):
        self.runtime.events.append("backup")
        self.assertFalse(self.runtime.running)
        if self.fail_capture:
            raise RuntimeError("backup unavailable")
        with patch.object(runtime, "run", side_effect=self.external):
            captured = self.native_backup.backup(ca_operation=ca_operation)
        self.backup_path = self.root / "backups/original"
        captured.rename(self.backup_path)
        return self.backup_path

    def assert_original_snapshot_restorable(self):
        self.assertEqual(self.captured_head, self.baseline)
        self.assertTrue((self.backup_path / "COMPLETE").is_file())
        restored = self.root.parent / "restored"
        restored.mkdir(mode=0o700)
        with tarfile.open(self.backup_path / "private-state.tar.gz", "r:gz") as archive:
            self.assertFalse(any(name.startswith("maintenance/") for name in archive.getnames()))
            archive.extractall(restored, filter="data")
        self.assertFalse((restored / "config/crl-maintenance.json").exists())
        self.assertEqual(json.loads((restored / "config/settings.json").read_text()), self.config)
        self.assertEqual((restored / "config/schema").read_text().strip(), self.metadata["schema"])
        for name, original in self.original.items():
            self.assertEqual((restored / "data" / name).read_bytes(), original, name)
        self.assertEqual(self.publications, [3])
        self.assertTrue(self.runtime.running)
        self.assert_keys_preserved()

    def prepared_portal_publication(self):
        identifier = "10000000-0000-4000-8000-000000000001"
        portal = self.root / "data/ca/publications" / identifier
        portal.mkdir(parents=True)
        renew_crl.journal.save(portal / "journal.json", {"receipt": None})
        operation, _ = renew_crl.prepared(self.root, self.target, self.metadata, 3,
                                        identifier=identifier)
        self.runtime.stop()
        return operation

    def portal_snapshot(self, operation):
        with patch.object(runtime, "run", side_effect=self.external):
            return self.native_backup.backup(ca_operation=("publications", operation.name))

    def test_portal_archive_retains_only_exact_public_crl_recovery_evidence(self):
        operation = self.prepared_portal_publication()
        staged = operation / "ca/private"
        staged.mkdir(parents=True)
        (staged / "ca.key.pem").write_bytes(b"excluded staging key")
        (operation / "unrelated.txt").write_bytes(b"excluded unrelated material")
        required = ("journal.json", "original-crl.pem", "candidate.pem")
        expected = {"maintenance/crl/" + operation.name + "/" + name:
                    (operation / name).read_bytes() for name in required}
        backup = self.portal_snapshot(operation)
        with tarfile.open(backup / "private-state.tar.gz", "r:gz") as archive:
            retained = {name: archive.extractfile(name).read() for name in archive.getnames()
                        if name.startswith("maintenance/")}
            self.assertEqual(retained, expected)
            portal = archive.extractfile("data/ca/publications/" + operation.name + "/journal.json")
            self.assertIsNone(json.load(portal)["receipt"])
        self.assertEqual(json.loads(retained["maintenance/crl/" + operation.name + "/journal.json"])
                         ["state"], "prepared")
        self.assertEqual(self.captured_head, self.baseline)
        self.assertTrue((backup / "COMPLETE").is_file())

    def test_portal_recovery_evidence_missing_or_linked_cannot_complete_backup(self):
        operation = self.prepared_portal_publication()
        for name in ("journal.json", "original-crl.pem", "candidate.pem"):
            path = operation / name
            original = path.read_bytes()
            for kind in ("missing", "linked"):
                path.unlink()
                if kind == "linked":
                    path.symlink_to(self.active_crl)
                with self.subTest(name=name, kind=kind):
                    with self.assertRaises((OSError, ValueError, RuntimeError)):
                        self.portal_snapshot(operation)
                    self.assertEqual(list((self.root / "backups").glob("*/COMPLETE")), [])
                if path.is_symlink():
                    path.unlink()
                path.write_bytes(original)
                path.chmod(0o600)

    def test_real_private_archive_restores_original_pki_without_an_external_journal_reference(self):
        self.assertEqual(self.renew()["revision"], 4)
        self.assert_original_snapshot_restorable()

    def first_capture_fails(self):
        self.fail_capture = True
        with self.assertRaisesRegex(RuntimeError, "backup unavailable"):
            self.renew()
        self.assertTrue(self.fence.is_file())
        self.assertFalse(self.runtime.running)
        self.assertEqual(self.publications, [])
        self.assertEqual(self.counter.read_bytes(), b"1001\n")
        self.fail_capture = False

    def test_resume_without_backup_can_capture_only_the_unchanged_original_state(self):
        self.first_capture_fails()
        self.assertEqual(self.resume()["revision"], 4)
        self.assert_original_snapshot_restorable()

    def assert_advanced_state_cannot_become_the_original_backup(self):
        state = copy.deepcopy(self.head)
        counter = self.counter.read_bytes()
        crl = self.active_crl.read_bytes()
        with self.assertRaises((ValueError, RuntimeError)):
            self.resume()
        self.assertTrue(self.fence.is_file())
        self.assertFalse(self.runtime.running)
        self.assertEqual(self.runtime.events.count("backup"), 1)
        self.assertEqual(self.head, state)
        self.assertEqual(self.counter.read_bytes(), counter)
        self.assertEqual(self.active_crl.read_bytes(), crl)
        self.assertEqual(self.publications, [])
        self.assertIsNone(self.captured_head)
        self.assertEqual(list((self.root / "backups").iterdir()), [])

    def test_missing_backup_and_advanced_counter_keep_the_fence_and_refuse_capture(self):
        self.first_capture_fails()
        self.counter.write_text("1002\n")
        self.assert_advanced_state_cannot_become_the_original_backup()

    def test_missing_backup_and_installed_candidate_keep_the_fence_and_refuse_capture(self):
        self.first_capture_fails()
        self.active_crl.write_bytes(b"candidate CRL")
        self.assert_advanced_state_cannot_become_the_original_backup()

    def test_missing_backup_after_sql_commit_keeps_the_fence_without_creating_a_new_snapshot(self):
        self.first_capture_fails()
        self.head = {**copy.deepcopy(self.candidate), "revision": 4}
        self.active_crl.write_bytes(b"candidate CRL")
        self.counter.write_text("1002\n")
        self.assert_advanced_state_cannot_become_the_original_backup()


if __name__ == "__main__":
    unittest.main()
