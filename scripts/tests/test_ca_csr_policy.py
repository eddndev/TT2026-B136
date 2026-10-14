"""A proven subject-policy mismatch rejects only an intact reserved issuance."""
import copy
import json
from pathlib import Path
import sys
from unittest.mock import patch

from ca_issuance_support import Executor, IssuanceFixture, OPERATION, REPOSITORY
from ca_native_executor_support import NativeFixture
import crl_journal
import issuance
import issuance_material as material
from native_executor import NativeExecutor
import portal


class CaCsrPolicyTests(IssuanceFixture):
    def rejecting_executor(self, *, facts=None):
        executor = Executor(self)
        admitted = executor.admit(self.request["csr_pem"].encode(),
                                  (self.ca / "ca.crt.pem").read_bytes())
        if facts is not None:
            admitted.update(facts)
        def reject(csr, root):
            raise material.CsrSubjectPolicyMismatch(admitted)
        executor.admit = reject
        return executor

    def test_new_policy_mismatch_never_reserves_a_journal_or_serial(self):
        before = self.state()
        executor = self.rejecting_executor()
        with self.assertRaises(material.CsrSubjectPolicyMismatch):
            self.issue(executor=executor)
        self.assertEqual(self.state(), before)
        self.assertFalse(self.operation.exists())
        self.assertEqual(executor.calls, [])

    def test_pristine_legacy_request_gets_one_correlated_terminal_rejection(self):
        self.interrupt("before")
        original = issuance.load(self.operation)
        before = material.snapshot(self.ca)
        executor = self.rejecting_executor()
        with patch("issuance.time.time", return_value=original["recorded_at"] + 5):
            receipt = self.resume(executor)
        expected = {"state": "rejected", "reason": "csr_subject_policy",
            "operation_id": OPERATION, "recorded_at": original["recorded_at"],
            "rejected_at": original["recorded_at"] + 5,
            "csr_sha256": original["admitted"]["csr_sha256"],
            "root_fingerprint": original["admitted"]["root_fingerprint"],
            "reserved_serial": original["before"]["serial"],
            **{key: self.request[key] for key in (*issuance.HISTORY_FIELDS, "purpose")}}
        self.assertEqual(receipt, expected)
        stored = issuance.load(self.operation)
        self.assertEqual({**stored, "receipt": None}, original)
        self.assertEqual(material.snapshot(self.ca), before)
        self.assertEqual(executor.calls, [])
        self.assertEqual(self.resume(executor), receipt)
        self.assertEqual(self.issue(executor=executor), receipt)
        crl_journal.guard_mutation(self.root)
        with crl_journal.locked(self.root):
            issuance.guard_pending(self.root)
        row = portal.history(self.root)[0]
        self.assertEqual((row["state"], row["reason"]), ("rejected", "csr_subject_policy"))
        self.assertIsNone(row["serial"])
        self.assertIsNone(row["fingerprint"])

    def test_generic_inspection_failure_preserves_pending_operation(self):
        self.interrupt("before")
        before = self.state()
        for failure in (ValueError("invalid CSR"), RuntimeError("worker unavailable")):
            with patch.object(self.executor, "admit", side_effect=failure):
                with self.assertRaises(type(failure)):
                    self.resume()
            self.assertEqual(self.state(), before)
            with self.assertRaises(RuntimeError):
                crl_journal.guard_mutation(self.root)

    def test_policy_rejection_requires_all_pinned_admission_facts(self):
        self.interrupt("before")
        before = self.state()
        for name in ("csr_sha256", "spki_sha256", "root_fingerprint"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.resume(self.rejecting_executor(facts={name: "0" * 64}))
            self.assertEqual(self.state(), before)

    def test_any_mutated_openssl_material_keeps_policy_failure_pending(self):
        self.interrupt("before")
        mutations = [(self.ca / name, b"changed\n") for name in material.LIMITS]
        mutations.extend((path, b"certificate\n") for path in
                         material.leaf_paths(self.ca, OPERATION, "1000"))
        for path, changed in mutations:
            original = path.read_bytes() if path.exists() else None
            path.write_bytes(changed)
            before = self.state()
            with self.subTest(path=path.name), self.assertRaises((ValueError, RuntimeError)):
                self.resume(self.rejecting_executor())
            self.assertEqual(self.state(), before)
            if original is None:
                path.unlink()
            else:
                path.write_bytes(original)

    def test_rejected_receipt_validation_rejects_tampering_in_both_guards(self):
        self.interrupt("before")
        self.resume(self.rejecting_executor())
        record = issuance.load(self.operation)
        variants = [("state", "completed"),
            ("operation_id", "10000000-0000-4000-8000-000000000002"),
            ("operator_id", "20000000-0000-4000-8000-000000000002"),
            ("reason", "anything"), ("recorded_at", record["recorded_at"] + 1),
            ("rejected_at", record["recorded_at"] - 1), ("rejected_at", True),
            ("reserved_serial", "1001"), ("csr_sha256", "0" * 64),
            ("root_fingerprint", "0" * 64), ("purpose", "document-signing"),
            ("holder", "Someone else"), ("reference", "other"), ("criterion", "other")]
        for field, value in variants:
            changed = copy.deepcopy(record)
            changed["receipt"][field] = value
            with self.subTest(field=field):
                with self.assertRaises(ValueError):
                    issuance.completed(changed)
                crl_journal.save(self.operation / "journal.json", changed)
                with self.assertRaises(ValueError):
                    crl_journal.guard_mutation(self.root)
        no_state = {key: value for key, value in record["receipt"].items() if key != "state"}
        for receipt in ({"state": "rejected"}, {**record["receipt"], "extra": True}, no_state):
            changed = {**record, "receipt": receipt}
            with self.assertRaises(ValueError):
                issuance.completed(changed)
            crl_journal.save(self.operation / "journal.json", changed)
            with self.assertRaises(ValueError):
                crl_journal.guard_mutation(self.root)


class CaNativePolicyBoundaryTests(IssuanceFixture):
    def setUp(self):
        super().setUp()
        self.native = NativeExecutor(Path(sys.executable), Path(sys.executable), REPOSITORY / "pki")
        self.facts = {"csr_sha256": "a" * 64, "spki_sha256": "b" * 64,
                      "root_fingerprint": "c" * 64, "subject": "CN=Holder"}

    def test_valid_false_policy_has_distinct_error_with_verified_facts(self):
        output = json.dumps({**self.facts, "issuer_policy_matches": False}).encode()
        with patch.object(self.native, "_run", return_value=output):
            with self.assertRaises(material.CsrSubjectPolicyMismatch) as raised:
                self.native.admit(b"CSR", b"root")
        self.assertEqual(raised.exception.admitted,
                         {key: self.facts[key] for key in ("csr_sha256", "spki_sha256", "root_fingerprint")})

    def test_malformed_or_missing_policy_is_generic_and_cannot_reject_journal(self):
        outputs = [self.facts]
        outputs.extend({**self.facts, "issuer_policy_matches": value}
                       for value in (0, 1, "false", None))
        outputs.append({**self.facts, "csr_sha256": "bad", "issuer_policy_matches": False})
        for output in outputs:
            with self.subTest(output=output):
                with patch.object(self.native, "_run", return_value=json.dumps(output).encode()):
                    with self.assertRaises(RuntimeError):
                        self.native.admit(b"CSR", b"root")


class CaNativeSubjectRecoveryTests(NativeFixture):
    def test_cn_only_csr_is_not_reserved_and_pristine_legacy_journal_can_be_rejected(self):
        csr_path = self.scratch / "cn-only.csr.pem"
        self.command(["openssl", "req", "-new", "-sha256", "-key", self.holder_key,
                      "-out", csr_path, "-subj", "/CN=External Native Holder"])
        self.csr = csr_path.read_bytes()
        request = self.request("partner")
        before = material.prepare(self.ca, OPERATION)
        with self.assertRaises(material.CsrSubjectPolicyMismatch) as failure:
            self.issue("partner", operation=OPERATION)
        operation = self.ca / "operations" / OPERATION
        self.assertFalse(operation.exists())
        self.assertEqual(material.snapshot(self.ca), before)
        record = {"format": issuance.FORMAT, "operation_id": OPERATION,
            "request": request, "admitted": failure.exception.admitted,
            "before": before, "recorded_at": 1, "receipt": None}
        crl_journal.directory(operation.parent)
        crl_journal.directory(operation)
        crl_journal.save(operation / "journal.json", record)
        with patch.object(self.executor, "issue", side_effect=AssertionError("must not sign")):
            receipt = self.resume(OPERATION)
            self.assertEqual(self.resume(OPERATION), receipt)
        self.assertEqual((receipt["state"], receipt["reason"]), ("rejected", "csr_subject_policy"))
        self.assertEqual({**issuance.load(operation), "receipt": None}, record)
        self.assertEqual(material.snapshot(self.ca), before)
        self.assertTrue(all(material.absent(path) for path in
                            material.leaf_paths(self.ca, OPERATION, before["serial"])))
        crl_journal.guard_mutation(self.root)
