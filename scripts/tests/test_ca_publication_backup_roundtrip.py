"""Restore a prepared portal publication from its real private backup archive."""
import copy
import json
import shutil
import tarfile
import unittest
from unittest.mock import patch

from ca_issuance_support import OPERATION, Interrupted
from test_ca_publication import PublicationFixture
import test_deploy_crl_backup_snapshot as snapshot
from test_deploy_crl_support import renew_crl
import runtime


class CaPublicationBackupRoundTripTests(PublicationFixture):
    external = snapshot.CrlBackupSnapshotTests.external
    capture = snapshot.CrlBackupSnapshotTests.capture

    def test_restored_prepared_publication_reuses_uuid_candidate_and_private_bytes(self):
        (self.root / "backups").mkdir(mode=0o700)
        self.config = {"postgres_port": 15486, "redis_port": 16386,
                       "admin_password": "fixture-admin", "runtime_password": "fixture-user",
                       "redis_password": "fixture-redis", "kek": "fixture-kek"}
        settings = self.root / "config/settings.json"
        settings.write_text(json.dumps(self.config), encoding="ascii")
        settings.chmod(0o600)
        ca_config = self.root / "config/ca-service.json"
        ca_config.write_text(json.dumps({"operator_id": "fixture-operator",
            "kek_base64": "fixture-distinct-ca-kek"}), encoding="ascii")
        ca_config.chmod(0o600)
        self.native_backup = runtime.Runtime(self.root)
        self.fail_capture = False
        self.captured_head = self.backup_path = None

        def capture_then_interrupt(*, ca_operation=None):
            self.assertEqual(ca_operation, ("publications", OPERATION))
            self.capture(ca_operation=ca_operation)
            raise Interrupted("backup captured before publication advanced")

        self.runtime.backup = capture_then_interrupt
        with self.assertRaises(Interrupted):
            self.portal_publish()
        self.assertEqual(self.generated, [self.maintenance])
        self.assertEqual(self.publications, [])
        self.assertEqual(self.captured_head, self.baseline)
        self.assertFalse(self.runtime.running)
        self.assertTrue((self.backup_path / "COMPLETE").is_file())

        source_root, source_target = self.root, self.target
        source_maintenance = self.maintenance
        expected = {"data/" + name: value for name, value in self.original.items()}
        for name in ("config/schema", "config/settings.json", "config/ca-service.json",
                     "data/ca/publications/" + OPERATION + "/journal.json"):
            expected[name] = (source_root / name).read_bytes()
        for name in ("journal.json", "original-crl.pem", "candidate.pem"):
            relative = "maintenance/crl/" + OPERATION + "/" + name
            expected[relative] = (source_root / relative).read_bytes()
        portal_before = json.loads(expected["data/ca/publications/" + OPERATION + "/journal.json"])
        maintained_before = json.loads(expected["maintenance/crl/" + OPERATION + "/journal.json"])
        self.assertIsNone(portal_before["receipt"])
        self.assertEqual(maintained_before["state"], "prepared")
        self.assertIsNone(maintained_before["backup"])

        restored = source_root.parent / "restored"
        restored.mkdir(mode=0o700)
        with tarfile.open(self.backup_path / "private-state.tar.gz", "r:gz") as archive:
            actual = {member.name: archive.extractfile(member).read()
                      for member in archive.getmembers() if member.isfile()}
            self.assertEqual(actual, expected)
            archive.extractall(restored, filter="data")
        for name, value in expected.items():
            self.assertEqual((restored / name).read_bytes(), value, name)
        self.assertFalse((restored / "config/crl-maintenance.json").exists())
        self.assertEqual(json.loads((restored / "config/settings.json").read_text()), self.config)

        # Release installation and the captured SQL head are controlled boundaries.
        self.root = restored
        self.target = restored / "releases" / source_target.name
        shutil.copytree(source_target, self.target)
        (restored / "current").symlink_to(self.target)
        (restored / "backups").mkdir(mode=0o700)
        self.head = copy.deepcopy(self.captured_head)
        self.fence = restored / "config/crl-maintenance.json"
        self.active_crl = restored / "data/ca/crl/crl.pem"
        self.counter = restored / "data/ca/crlnumber"
        self.operation = restored / "data/ca/publications" / OPERATION
        self.maintenance = restored / "maintenance/crl" / OPERATION
        self.native_backup = runtime.Runtime(restored)
        self.runtime.events.clear()
        self.runtime.backup = self.capture

        with patch.object(renew_crl.material, "generate",
                          side_effect=AssertionError("restoration must reuse the saved candidate")):
            receipt = self.portal_resume()
            self.assert_receipt(receipt)
            events = list(self.runtime.events)
            self.assertEqual(self.portal_resume(), receipt)
            self.assertEqual(self.runtime.events, events)

        self.assertEqual(self.runtime.events, ["stop", "backup", "stop", "start", "check"])
        self.assertTrue(self.runtime.running)
        self.assertEqual(self.generated, [source_maintenance])
        self.assertEqual(self.publications, [3])
        self.assertEqual(self.counter.read_bytes(), b"1002\n")
        self.assertEqual(self.active_crl.read_bytes(), (self.maintenance / "candidate.pem").read_bytes())
        self.assert_keys_preserved()
        for name in ("config/schema", "config/settings.json", "config/ca-service.json"):
            self.assertEqual((restored / name).read_bytes(), expected[name], name)
        for name in ("original-crl.pem", "candidate.pem"):
            relative = "maintenance/crl/" + OPERATION + "/" + name
            self.assertEqual((restored / relative).read_bytes(), expected[relative], name)
        self.assertEqual(json.loads((self.operation / "journal.json").read_text()),
                         {**portal_before, "receipt": receipt})
        maintained = json.loads((self.maintenance / "journal.json").read_text())
        for name in set(maintained_before) - {"state", "backup"}:
            self.assertEqual(maintained[name], maintained_before[name], name)
        self.assertEqual(maintained["backup"]["path"], str(restored / "backups/original"))
        self.assertEqual(list(self.operation.parent.iterdir()), [self.operation])
        self.assertEqual(list(self.maintenance.parent.iterdir()), [self.maintenance])


if __name__ == "__main__":
    unittest.main()
