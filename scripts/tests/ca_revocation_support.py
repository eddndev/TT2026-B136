"""Owned synthetic public certificates and an observable revocation executor."""
import json
import os
import stat

from ca_issuance_support import IssuanceFixture, OPERATION, OPERATOR, ROOT, Interrupted
from ca_issuance_support import digest, encoded, journal


class RevocationExecutor:
    def __init__(self, owner):
        self.owner = owner
        self.calls = 0
        self.mutations = 0
        self.effect = "complete"

    def describe_certificate(self, certificate_pem, root_pem):
        if root_pem != ROOT:
            raise ValueError("certificate signature does not match adopted root")
        facts = json.loads(certificate_pem)
        return {**facts, "fingerprint": digest(encoded(facts))}

    def revoke(self, ca, certificate_pem, *, lock_fd):
        self.owner.assertEqual(ca, self.owner.ca)
        self.owner.assertEqual(certificate_pem, self.owner.leaf)
        expected, held = (self.owner.root / "deploy.lock").stat(), os.fstat(lock_fd)
        self.owner.assertEqual((held.st_dev, held.st_ino), (expected.st_dev, expected.st_ino))
        record = self.owner.revocation / "journal.json"
        self.owner.assertTrue(record.is_file(), "journal must precede revocation")
        self.owner.assertEqual(stat.S_IMODE(record.stat().st_mode), 0o600)
        with self.owner.assertRaises(BlockingIOError), journal.locked(self.owner.root):
            pass
        self.calls += 1
        if self.effect == "before":
            raise Interrupted("before revocation")
        index = ca / "index.txt"
        original = index.read_bytes()
        revoked = original.replace(b"V\t", b"R\t", 1).replace(
            b"Z\t\t", b"Z\t261012000000Z\t", 1)
        (ca / "index.txt.old").write_bytes(original)
        if self.effect == "partial":
            (ca / "index.txt.new").write_bytes(revoked)
        else:
            index.write_bytes(revoked)
            self.mutations += 1
        if self.effect != "complete":
            raise Interrupted("revocation response lost")


class RevocationFixture(IssuanceFixture):
    def setUp(self):
        super().setUp()
        self.leaf = self.certificate(subject="CN=Declared Holder", not_before=100, not_after=200)
        (self.ca / "newcerts/1000.pem").write_bytes(self.leaf)
        (self.ca / "certs/holder.crt.pem").write_bytes(self.leaf)
        (self.ca / "index.txt").write_text(self.index_row(), encoding="ascii")
        (self.ca / "serial").write_bytes(b"1001\n")
        self.revocation = self.ca / "revocations" / OPERATION
        self.revoke_request = {"serial": "1000", "fingerprint": digest(self.leaf),
            "reason": "External holder requested cancellation", "operator_id": OPERATOR,
            "expected_root_fingerprint": digest(ROOT)}
        self.executor = RevocationExecutor(self)

    def revoke(self, *, request=None, operation=OPERATION):
        import revocation
        return revocation.revoke(self.root, operation,
            self.revoke_request if request is None else request, self.executor)

    def interrupted_revocation(self, effect):
        self.executor.effect = effect
        with self.assertRaises(Interrupted):
            self.revoke()
        self.assertEqual(self.executor.calls, 1)

    def assert_revocation_receipt(self, receipt):
        self.assertIs(type(receipt["recorded_at"]), int)
        self.assertGreater(receipt["recorded_at"], 0)
        self.assertEqual(receipt, {"operation_id": OPERATION, "serial": "1000",
            "fingerprint": self.revoke_request["fingerprint"], "reason": self.revoke_request["reason"],
            "operator_id": OPERATOR, "recorded_at": receipt["recorded_at"],
            "local_status": "revoked", "published_status": "pending"})
