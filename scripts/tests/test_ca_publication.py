"""Portal CRL publication reuses the established durable maintenance workflow."""
import json
from pathlib import Path
import stat
import unittest
from unittest.mock import patch

from ca_issuance_support import OPERATION, OPERATOR, Interrupted, journal
from test_deploy_crl_support import CrlFixture, renew_crl
import publication


OTHER_OPERATION = "10000000-0000-4000-8000-000000000002"


class PublicationFixture(CrlFixture):
    def setUp(self):
        super().setUp()
        self.operation = self.root / "data/ca/publications" / OPERATION
        self.maintenance = self.root / "maintenance/crl" / OPERATION
        self.request = {"operator_id": OPERATOR, "expected_revision": 3,
            "acknowledge_maintenance": True, "expected_root_fingerprint": self.head["root_fingerprint"]}
        self.generation_interruption = None
        runtime = patch.object(renew_crl, "Runtime", return_value=self.runtime)
        runtime.start()
        self.addCleanup(runtime.stop)

    def generate(self, root, target, operation):
        path = self.operation / "journal.json"
        self.assertTrue(path.is_file(), "portal journal must precede candidate generation")
        self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
        with self.assertRaises(BlockingIOError), journal.locked(root):
            pass
        self.assertEqual(operation, self.maintenance)
        result = super().generate(root, target, operation)
        staged = operation / "ca"
        staged.mkdir(mode=0o700)
        for name in ("index.txt", "serial", "crlnumber", "ca.crt.pem", "index.txt.attr"):
            (staged / name).write_bytes((root / "data/ca" / name).read_bytes())
            (staged / name).chmod(0o600)
        (staged / "crlnumber.old").write_bytes((staged / "crlnumber").read_bytes())
        if self.generation_interruption != "partial":
            (staged / "crlnumber").write_text(result["next_counter"] + "\n", encoding="ascii")
        if self.generation_interruption:
            raise Interrupted("candidate generated before maintenance journal response")
        return result

    def portal_publish(self, request=None, operation=OPERATION):
        return publication.publish(self.root, operation, self.request if request is None else request)

    def portal_resume(self):
        return publication.resume(self.root, OPERATION)

    def assert_receipt(self, receipt):
        self.assertIs(type(receipt["recorded_at"]), int)
        self.assertEqual(receipt, {"operation_id": OPERATION, "operator_id": OPERATOR,
            "recorded_at": receipt["recorded_at"], "expected_revision": 3, "revision": 4,
            "crl_number": 4097, "root_fingerprint": self.request["expected_root_fingerprint"],
            "state": "published"})
        self.assertGreater(receipt["recorded_at"], 0)
        self.assertEqual(json.loads((self.maintenance / "journal.json").read_text())["state"], "accepted")
        self.assertEqual(self.head["revision"], 4)
        self.assertFalse(self.fence.exists())


class CaPublicationTests(PublicationFixture):
    def test_publication_keeps_same_uuid_and_existing_backup_stop_start_check_workflow(self):
        receipt = self.portal_publish()
        self.assert_receipt(receipt)
        self.assertEqual(self.generated, [self.maintenance])
        self.assertEqual(self.publications, [3])
        self.assertEqual(self.runtime.events, ["stop", "backup", "stop", "start", "check"])
        self.assert_keys_preserved()
        before = list(self.runtime.events)
        self.assertEqual(self.portal_publish(), receipt)
        self.assertEqual(self.portal_resume(), receipt)
        self.assertEqual(self.runtime.events, before)
        self.assertEqual(self.generated, [self.maintenance])

    def test_actor_request_or_revision_cannot_be_changed_for_existing_operation(self):
        self.portal_publish()
        for field, value in (("operator_id", OTHER_OPERATION), ("expected_revision", 4),
                             ("expected_root_fingerprint", "0" * 64)):
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.portal_publish({**self.request, field: value})
        self.assertEqual(len(self.generated), 1)

    def test_invalid_acknowledgement_revision_root_or_extra_fields_have_no_effect(self):
        for values in ({"acknowledge_maintenance": False}, {"acknowledge_maintenance": 1},
                       {"expected_revision": 2}, {"expected_revision": True},
                       {"expected_root_fingerprint": "0" * 64}, {"operator_id": "bad"},
                       {"extra": "excluded"}):
            with self.subTest(values=values), self.assertRaises(ValueError):
                self.portal_publish({**self.request, **values})
        self.assertEqual(self.generated, [])
        self.assertEqual(self.runtime.events, [])
        self.assertFalse(self.operation.exists())

    def test_lost_sql_response_resumes_same_candidate_without_second_generation_or_commit(self):
        self.publish_error = "after"
        with self.assertRaises(RuntimeError):
            self.portal_publish()
        self.assertFalse(self.runtime.running)
        self.assertIsNone(json.loads((self.operation / "journal.json").read_text())["receipt"])
        self.assert_receipt(self.portal_resume())
        self.assertEqual(self.publications, [3])
        self.assertEqual(self.generated, [self.maintenance])
        self.assertEqual(self.runtime.events.count("backup"), 1)

    def test_lost_portal_receipt_after_accepted_recovers_without_touching_runtime(self):
        original = journal.save
        def save(path, value):
            if Path(path) == self.operation / "journal.json" and value.get("receipt"):
                raise Interrupted("portal receipt response lost")
            return original(path, value)
        with patch.object(journal, "save", side_effect=save), self.assertRaises(Interrupted):
            self.portal_publish()
        self.assertFalse(self.fence.exists())
        before = list(self.runtime.events)
        self.assert_receipt(self.portal_resume())
        self.assertEqual(self.runtime.events, before)
        self.assertEqual(self.generated, [self.maintenance])
        self.assertEqual(self.publications, [3])

    def test_health_failure_keeps_pending_receipt_and_reuses_committed_trust(self):
        self.runtime.fail_start = True
        with self.assertRaises(RuntimeError):
            self.portal_publish()
        self.assertTrue(self.fence.exists())
        self.assertFalse(self.runtime.running)
        self.assertIsNone(json.loads((self.operation / "journal.json").read_text())["receipt"])
        self.runtime.fail_start = False
        self.assert_receipt(self.portal_resume())
        self.assertEqual(self.publications, [3])
        self.assertEqual(self.generated, [self.maintenance])

    def test_complete_staging_without_maintenance_journal_is_recovered_without_generating(self):
        self.generation_interruption = "complete"
        with self.assertRaises(Interrupted):
            self.portal_publish()
        self.assertFalse((self.maintenance / "journal.json").exists())
        self.assertEqual(self.counter.read_bytes(), b"1001\n")
        self.assertEqual(self.runtime.events, [])
        self.assert_receipt(self.portal_resume())
        self.assertEqual(self.generated, [self.maintenance])
        self.assertEqual(self.publications, [3])

    def test_no_effect_interruption_before_prepared_can_resume_once(self):
        with patch.object(renew_crl, "prepared", side_effect=Interrupted("before preparation")):
            with self.assertRaises(Interrupted):
                self.portal_publish()
        self.assertFalse(self.maintenance.exists())
        self.assert_receipt(self.portal_resume())
        self.assertEqual(self.generated, [self.maintenance])

    def test_partial_staging_never_regenerates_or_repairs(self):
        self.generation_interruption = "partial"
        with self.assertRaises(Interrupted):
            self.portal_publish()
        before = {path.relative_to(self.maintenance): path.read_bytes()
                  for path in self.maintenance.rglob("*") if path.is_file()}
        with self.assertRaises((ValueError, RuntimeError)):
            self.portal_resume()
        self.assertEqual({path.relative_to(self.maintenance): path.read_bytes()
                          for path in self.maintenance.rglob("*") if path.is_file()}, before)
        self.assertEqual(self.generated, [self.maintenance])
        self.assertEqual(self.runtime.events, [])

    def test_staged_recovery_rejects_changed_candidate_root_counter_and_baseline(self):
        self.generation_interruption = "complete"
        with self.assertRaises(Interrupted):
            self.portal_publish()
        for path, bad in ((self.maintenance / "candidate.pem", b"unrelated CRL"),
                          (self.maintenance / "ca/ca.crt.pem", b"unrelated CA"),
                          (self.maintenance / "ca/crlnumber", b"1003\n"),
                          (self.root / "data/ca/index.txt", b"changed index\n")):
            original = path.read_bytes()
            path.write_bytes(bad)
            with self.subTest(path=path.name), self.assertRaises((ValueError, RuntimeError)):
                self.portal_resume()
            path.write_bytes(original)
        self.assertEqual(self.generated, [self.maintenance])
        self.assertEqual(self.runtime.events, [])

    def test_accepted_journal_without_exact_successor_head_cannot_create_receipt(self):
        original = journal.save
        def save(path, value):
            if Path(path) == self.operation / "journal.json" and value.get("receipt"):
                raise Interrupted("portal receipt not saved")
            return original(path, value)
        with patch.object(journal, "save", side_effect=save), self.assertRaises(Interrupted):
            self.portal_publish()
        self.head["revision"] = 5
        before = list(self.runtime.events)
        with self.assertRaises((ValueError, RuntimeError)):
            self.portal_resume()
        self.assertEqual(self.runtime.events, before)
        self.assertEqual(self.generated, [self.maintenance])

    def test_lock_restore_foreign_crl_and_other_portal_pending_block_before_preparation(self):
        with journal.locked(self.root), self.assertRaises(BlockingIOError):
            self.portal_publish()
        for relative, value in (("maintenance/restore/active.json", {}),
                                ("config/crl-maintenance.json", {"operation_id": OTHER_OPERATION}),
                                ("data/ca/operations/" + OTHER_OPERATION + "/journal.json", {"receipt": None}),
                                ("data/ca/revocations/" + OTHER_OPERATION + "/journal.json", {"receipt": None})):
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            journal.save(path, value)
            with self.assertRaises((ValueError, RuntimeError)):
                self.portal_publish()
            path.unlink()
            if "/journal.json" in relative:
                path.parent.rmdir()
        self.assertEqual(self.generated, [])
        self.assertEqual(self.runtime.events, [])
        self.assertFalse(self.operation.exists())


if __name__ == "__main__":
    unittest.main()
