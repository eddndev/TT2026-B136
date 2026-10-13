"""Exact local revocation and conservative recovery, independent of CRL publish."""
import json
import unittest
from unittest.mock import patch

from ca_issuance_support import OPERATION, ROOT, journal
from ca_revocation_support import RevocationFixture
import revocation


OTHER_OPERATION = "10000000-0000-4000-8000-000000000002"


class CaRevocationTests(RevocationFixture):
    def test_revokes_once_and_returns_identical_durable_receipt_without_publishing(self):
        before = {name: (self.ca / name).read_bytes() for name in ("serial", "crlnumber", "ca.crt.pem")}
        with patch.object(revocation.time, "time", return_value=1791763200):
            receipt = self.revoke()
        self.assert_revocation_receipt(receipt)
        self.assertEqual(receipt["recorded_at"], 1791763200)
        self.assertEqual(self.revoke(), receipt)
        self.assertEqual((self.executor.calls, self.executor.mutations), (1, 1))
        self.assertEqual({name: (self.ca / name).read_bytes() for name in before}, before)
        self.assertFalse((self.ca / "crl/crl.pem").exists())
        self.assertEqual(json.loads((self.revocation / "journal.json").read_bytes())["receipt"], receipt)

    def test_lost_response_recovers_exact_revoked_leaf_without_second_mutation(self):
        self.interrupted_revocation("lost")
        before = self.state()
        receipt = self.revoke()
        self.assert_revocation_receipt(receipt)
        self.assertEqual((self.executor.calls, self.executor.mutations), (1, 1))
        before.pop("revocations/" + OPERATION + "/journal.json")
        after = self.state()
        after.pop("revocations/" + OPERATION + "/journal.json")
        self.assertEqual(after, before)

    def test_interruption_before_effect_retries_only_completely_intact_baseline(self):
        self.interrupted_revocation("before")
        recorded_at = json.loads((self.revocation / "journal.json").read_bytes())["recorded_at"]
        self.executor.effect = "complete"
        receipt = self.revoke()
        self.assertEqual(receipt["recorded_at"], recorded_at)
        self.assertEqual((self.executor.calls, self.executor.mutations), (2, 1))

    def test_same_operation_rejects_changed_parameters_after_success_or_interruption(self):
        self.revoke()
        for field, value in (("reason", "Different reason"), ("fingerprint", "0" * 64),
                             ("serial", "1001"), ("expected_root_fingerprint", "0" * 64),
                             ("operator_id", OTHER_OPERATION)):
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.revoke(request={**self.revoke_request, field: value})
        self.assertEqual(self.executor.calls, 1)

    def test_request_requires_exact_identity_and_bounded_review_fields_before_effect(self):
        for changed in ({"fingerprint": "0" * 64}, {"expected_root_fingerprint": "0" * 64},
                        {"serial": "1001"}, {"serial": "../1000"}, {"reason": ""},
                        {"reason": "x" * 501}, {"reason": "line\nbreak"},
                        {"operator_id": "not-uuid"}, {"extra": "excluded"}):
            with self.subTest(changed=changed), self.assertRaises((ValueError, LookupError)):
                self.revoke(request={**self.revoke_request, **changed})
        self.assertFalse(self.revocation.exists())
        self.assertEqual(self.executor.calls, 0)

    def test_partial_openssl_files_block_recovery_without_repair(self):
        self.interrupted_revocation("partial")
        before = self.state()
        with self.assertRaises(RuntimeError):
            self.revoke()
        self.assertEqual(self.state(), before)
        self.assertEqual(self.executor.calls, 1)

    def test_changed_baseline_before_effect_never_retries(self):
        self.interrupted_revocation("before")
        (self.ca / "serial").write_bytes(b"1002\n")
        before = self.state()
        with self.assertRaises(RuntimeError):
            self.revoke()
        self.assertEqual(self.state(), before)
        self.assertEqual(self.executor.calls, 1)

    def test_revoked_row_without_date_or_with_changed_subject_is_not_recoverable(self):
        self.interrupted_revocation("lost")
        index = self.ca / "index.txt"
        valid = index.read_bytes()
        for invalid in (valid.replace(b"261012000000Z", b""),
                        valid.replace(b"Declared Holder", b"Another Holder")):
            index.write_bytes(invalid)
            with self.assertRaises(RuntimeError):
                self.revoke()
        self.assertEqual(self.executor.calls, 1)

    def test_recovery_rechecks_root_and_exact_certificate(self):
        self.interrupted_revocation("lost")
        (self.ca / "ca.crt.pem").write_bytes(b"different public root\n")
        with self.assertRaises((ValueError, RuntimeError)):
            self.revoke()
        (self.ca / "ca.crt.pem").write_bytes(ROOT)
        changed = json.loads(self.leaf)
        changed["spki_sha256"] = "a" * 64
        for path in (self.ca / "newcerts/1000.pem", self.ca / "certs/holder.crt.pem"):
            path.write_text(json.dumps(changed), encoding="ascii")
        with self.assertRaises(ValueError):
            self.revoke()
        self.assertEqual(self.executor.calls, 1)

    def test_an_existing_exact_revocation_is_observed_without_invoking_executor(self):
        self.revoke()
        with patch.object(self.executor, "revoke", side_effect=AssertionError("already revoked")):
            receipt = self.revoke(operation=OTHER_OPERATION)
        self.assertEqual(receipt["operation_id"], OTHER_OPERATION)
        self.assertEqual(receipt["local_status"], "revoked")
        self.assertEqual(receipt["published_status"], "pending")

    def test_pending_revocation_blocks_different_operation_but_allows_exact_resume(self):
        self.interrupted_revocation("before")
        with self.assertRaises(RuntimeError):
            self.revoke(operation=OTHER_OPERATION)
        self.assertFalse((self.ca / "revocations" / OTHER_OPERATION).exists())
        with journal.locked(self.root), self.assertRaises(RuntimeError):
            revocation.guard_pending(self.root)
        self.executor.effect = "complete"
        self.assert_revocation_receipt(self.revoke())
        with journal.locked(self.root):
            revocation.guard_pending(self.root)

    def test_shared_lock_restore_crl_and_pending_issuance_fences_precede_effect(self):
        with journal.locked(self.root), self.assertRaises(BlockingIOError):
            self.revoke()
        for relative in ("config/crl-maintenance.json", "maintenance/restore/active.json"):
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"{}")
            with self.assertRaises(RuntimeError):
                self.revoke()
            path.unlink()
        with patch.object(revocation.issuance, "guard_pending", side_effect=RuntimeError("issuance pending")):
            with self.assertRaises(RuntimeError):
                self.revoke()
        self.assertEqual(self.executor.calls, 0)
        self.assertFalse(self.revocation.exists())


if __name__ == "__main__":
    unittest.main()
