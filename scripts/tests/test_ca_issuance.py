"""Focal CA issuance contract: durable receipt, exact recovery and one signer."""
import copy
import unittest
from unittest.mock import patch

from ca_issuance_support import (
    OPERATION, OTHER_CSR, Executor, IssuanceFixture, digest, issuance, journal,
)


class CaIssuanceTests(IssuanceFixture):
    def test_pending_issuance_blocks_a_different_operation_until_resumed(self):
        self.interrupt("before")
        other = "10000000-0000-4000-8000-000000000002"
        persisted = self.state()
        restarted = Executor(self)
        with self.assertRaises(RuntimeError):
            issuance.issue(self.root, other, self.request, restarted)
        self.assertEqual(restarted.calls, [])
        self.assertEqual(self.state(), persisted)
        self.assertFalse((self.ca / "operations" / other).exists())
        self.assert_receipt(self.resume(restarted))
        self.assertEqual(restarted.signatures, ["1000"])

    def test_lost_final_response_returns_same_receipt_after_executor_restart(self):
        lost_response = self.issue()
        self.assert_receipt(lost_response)
        persisted = self.state()
        restarted = Executor(self)
        self.assertEqual(self.issue(executor=restarted), lost_response)
        self.assertEqual(self.resume(executor=restarted), lost_response)
        self.assertEqual(self.state(), persisted)
        self.assertEqual(len(self.executor.calls), 1)
        self.assertEqual(restarted.calls, [])

    def test_operation_identity_cannot_be_reused_with_changed_parameters(self):
        self.issue()
        persisted = self.state()
        for field, value in (("csr_pem", OTHER_CSR), ("purpose", "document-signing"),
                             ("holder", "Other Holder"), ("reference", "other-reference"),
                             ("criterion", "different review"),
                             ("operator_id", "20000000-0000-4000-8000-000000000002"),
                             ("expected_root_fingerprint", "0" * 64)):
            with self.subTest(field=field):
                changed = {**copy.deepcopy(self.request), field: value}
                with self.assertRaises(ValueError):
                    self.issue(changed)
                self.assertEqual(self.state(), persisted)
                self.assertEqual(len(self.executor.calls), 1)

    def test_lost_executor_response_recovers_reserved_serial_from_newcerts(self):
        self.interrupt("newcerts_only")
        self.assertEqual(list((self.ca / "certs").iterdir()), [])
        restarted = Executor(self)
        receipt = self.resume(restarted)
        self.assert_receipt(receipt)
        self.assertIn(self.certificate(), restarted.inspected)
        self.assertEqual(self.issue(executor=restarted), receipt)
        self.assertEqual(restarted.calls, [])
        self.assertEqual(len(self.executor.calls), 1)
        self.assertEqual((self.ca / "index.txt").read_text(), self.index_row())

    def test_interruption_before_signing_resumes_only_the_intact_saved_state(self):
        self.interrupt("before")
        self.assertEqual(self.executor.signatures, [])
        self.assertEqual((self.ca / "serial").read_bytes(), b"1000\n")
        self.assertEqual((self.ca / "index.txt").read_bytes(), b"")
        self.assertEqual(list((self.ca / "newcerts").iterdir()), [])
        self.assertEqual(list((self.ca / "certs").iterdir()), [])
        restarted = Executor(self)
        receipt = self.resume(restarted)
        self.assert_receipt(receipt)
        self.assertEqual(restarted.signatures, ["1000"])
        self.assertEqual(self.issue(executor=restarted), receipt)
        self.assertEqual(restarted.signatures, ["1000"])

    def rejected_certificate_fact(self, field, value):
        self.interrupt("newcerts_only")
        certificate_path = self.ca / "newcerts/1000.pem"
        certificate_path.write_bytes(self.certificate(**{field: value}))
        restarted = Executor(self)
        with self.assertRaises((ValueError, RuntimeError)):
            self.resume(restarted)
        self.assertEqual(restarted.calls, [])
        self.assertEqual(len(self.executor.calls), 1)
        self.assertEqual((self.ca / "serial").read_bytes(), b"1001\n")
        self.assertEqual((self.ca / "index.txt").read_text(), self.index_row())

    def test_recovery_rejects_a_different_csr_public_key(self):
        self.rejected_certificate_fact("spki_sha256", digest(b"different public key"))

    def test_recovery_rejects_a_different_profile(self):
        self.rejected_certificate_fact("purpose", "document-signing")

    def test_recovery_rejects_a_different_root(self):
        self.rejected_certificate_fact("root_fingerprint", "0" * 64)

    def test_recovery_rejects_a_certificate_outside_the_reserved_serial(self):
        self.rejected_certificate_fact("serial", "1001")

    def test_recovery_requires_the_reserved_serial_in_the_index(self):
        self.interrupt("newcerts_only")
        (self.ca / "index.txt").write_text(self.index_row("1001"))
        restarted = Executor(self)
        with self.assertRaises((ValueError, RuntimeError)):
            self.resume(restarted)
        self.assertEqual(restarted.calls, [])
        self.assertEqual((self.ca / "index.txt").read_text(), self.index_row("1001"))

    def test_partial_openssl_counter_without_certificate_never_reissues(self):
        self.interrupt("counter_only")
        restarted = Executor(self)
        for retry in (lambda: self.resume(restarted), lambda: self.issue(executor=restarted)):
            with self.assertRaises((ValueError, RuntimeError)):
                retry()
        self.assertEqual(restarted.calls, [])
        self.assertEqual((self.ca / "serial").read_bytes(), b"1001\n")
        self.assertEqual((self.ca / "index.txt").read_bytes(), b"")
        self.assertEqual(list((self.ca / "newcerts").iterdir()), [])

    def test_partial_certificate_without_index_never_becomes_a_receipt(self):
        self.interrupt("certificate_only")
        restarted = Executor(self)
        with self.assertRaises((ValueError, RuntimeError)):
            self.resume(restarted)
        self.assertEqual(restarted.calls, [])
        self.assertEqual((self.ca / "serial").read_bytes(), b"1000\n")
        self.assertEqual((self.ca / "index.txt").read_bytes(), b"")
        self.assertEqual((self.ca / "newcerts/1000.pem").read_bytes(), self.certificate())

    def test_partial_index_without_certificate_never_reissues(self):
        self.interrupt("index_only")
        restarted = Executor(self)
        with self.assertRaises((ValueError, RuntimeError)):
            self.resume(restarted)
        self.assertEqual(restarted.calls, [])
        self.assertEqual((self.ca / "index.txt").read_text(), self.index_row())
        self.assertEqual((self.ca / "serial").read_bytes(), b"1001\n")

    def test_shared_deploy_lock_blocks_before_journal_or_signer_effects(self):
        persisted = self.state()
        with journal.locked(self.root):
            with self.assertRaises(BlockingIOError):
                self.issue()
        self.assertEqual(self.state(), persisted)
        self.assertEqual(self.executor.calls, [])
        self.assertFalse(self.operation.exists())

    def test_initial_journal_failure_prevents_signing(self):
        with patch.object(journal, "write", side_effect=OSError("durability unavailable")):
            with self.assertRaises(OSError):
                self.issue()
        self.assertEqual(self.executor.calls, [])
        self.assertEqual((self.ca / "serial").read_bytes(), b"1000\n")
        self.assertEqual((self.ca / "index.txt").read_bytes(), b"")
        self.assertEqual(list((self.ca / "newcerts").iterdir()), [])

    def test_bounded_request_fields_are_rejected_before_any_filesystem_effect(self):
        persisted = self.state()
        invalid = [("csr_pem", "x" * (16 * 1024 + 1)),
                   ("csr_pem", "unverified request"),
                   ("purpose", "root"), ("operator_id", "not-a-uuid"),
                   ("expected_root_fingerprint", "g" * 64),
                   ("expected_root_fingerprint", "f" * 63),
                   ("expected_root_fingerprint", "0" * 64)]
        invalid.extend((field, "x" * 501) for field in ("holder", "reference", "criterion"))
        invalid.append(("holder", "\u00e9" * 251))
        for field, value in invalid:
            with self.subTest(field=field, length=len(value)):
                with self.assertRaises(ValueError):
                    self.issue({**self.request, field: value})
                self.assertEqual(self.executor.calls, [])
                self.assertEqual(self.state(), persisted)
                self.assertFalse(self.operation.exists())

    def test_operation_identity_is_a_canonical_non_nil_uuid(self):
        persisted = self.state()
        for operation in ("../outside", "10000000000040008000000000000001",
                          "00000000-0000-0000-0000-000000000000"):
            with self.subTest(operation=operation):
                with self.assertRaises(ValueError):
                    issuance.issue(self.root, operation, self.request, self.executor)
                self.assertEqual(self.executor.calls, [])
                self.assertEqual(self.state(), persisted)

    def test_crl_fence_blocks_issue_retry_and_pending_resume(self):
        self.interrupt("newcerts_only")
        journal.fenced(self.root, OPERATION)
        persisted = self.state()
        restarted = Executor(self)
        for action in (lambda: self.issue(executor=restarted), lambda: self.resume(restarted)):
            with self.assertRaises(RuntimeError):
                action()
            self.assertEqual(self.state(), persisted)
            self.assertEqual(restarted.calls, [])

    def test_restore_fence_blocks_issue_retry_and_pending_resume(self):
        self.interrupt("newcerts_only")
        directory = self.root / "maintenance/restore"
        directory.mkdir(mode=0o700, parents=True)
        journal.save(directory / "active.json", {"operation_id": OPERATION})
        persisted = self.state()
        restarted = Executor(self)
        for action in (lambda: self.issue(executor=restarted), lambda: self.resume(restarted)):
            with self.assertRaises(RuntimeError):
                action()
            self.assertEqual(self.state(), persisted)
            self.assertEqual(restarted.calls, [])


if __name__ == "__main__":
    unittest.main()
