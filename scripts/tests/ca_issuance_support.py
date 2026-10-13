"""Owned public-material fixtures; no OpenSSL process or external service."""
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
import unittest

REPOSITORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY / "ops/deploy"))
sys.path.insert(0, str(REPOSITORY / "ops/ca"))
import crl_journal as journal
import issuance


OPERATION = "10000000-0000-4000-8000-000000000001"
OPERATOR = "20000000-0000-4000-8000-000000000001"
ROOT = b"synthetic public root\n"
CSR = "synthetic public CSR A\n"
OTHER_CSR = "synthetic public CSR B\n"


def digest(value):
    return hashlib.sha256(value).hexdigest()


def encoded(value):
    return (json.dumps(value, sort_keys=True) + "\n").encode("ascii")


class Interrupted(BaseException):
    """An executor disappears after filesystem effects, before returning."""


class Executor:
    def __init__(self, owner, effect="complete"):
        self.owner = owner
        self.effect = effect
        self.calls = []
        self.signatures = []
        self.inspected = []

    def admit(self, csr_pem, root_pem):
        if csr_pem not in (CSR.encode("ascii"), OTHER_CSR.encode("ascii")):
            raise ValueError("request proof is invalid")
        if root_pem != ROOT:
            raise ValueError("root material is invalid")
        key = b"public key A" if csr_pem == CSR.encode("ascii") else b"public key B"
        return {"csr_sha256": digest(csr_pem), "spki_sha256": digest(key),
                "root_fingerprint": digest(ROOT)}

    def inspect_certificate(self, certificate_pem, root_pem):
        if root_pem != ROOT:
            raise ValueError("certificate signature does not match the root")
        facts = json.loads(certificate_pem)
        self.inspected.append(bytes(certificate_pem))
        return {**facts, "fingerprint": digest(certificate_pem)}

    def issue(self, ca_dir, output_name, csr_pem, purpose, *, lock_fd):
        ca = Path(ca_dir)
        self.owner.assertEqual(ca, self.owner.ca)
        held = os.fstat(lock_fd)
        expected = (self.owner.root / "deploy.lock").stat()
        self.owner.assertEqual((held.st_dev, held.st_ino), (expected.st_dev, expected.st_ino))
        self.owner.assertRegex(output_name, r"^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$")
        self.owner.assertEqual(csr_pem, CSR.encode("ascii"))
        self.owner.assertEqual(purpose, "partner")
        record = self.owner.operation / "journal.json"
        self.owner.assertTrue(record.is_file(), "journal must precede signing")
        self.owner.assertEqual(stat.S_IMODE(record.stat().st_mode), 0o600)
        with self.owner.assertRaises(BlockingIOError), journal.locked(self.owner.root):
            pass
        serial = (ca / "serial").read_text().strip()
        self.calls.append((output_name, serial))
        if self.effect == "before":
            raise Interrupted("executor disappeared before signing")
        self.signatures.append(serial)
        certificate = self.owner.certificate(serial=serial, purpose=purpose)
        if self.effect in ("complete", "newcerts_only", "certificate_only"):
            (ca / "newcerts" / (serial + ".pem")).write_bytes(certificate)
        if self.effect == "complete":
            (ca / "certs" / (output_name + ".crt.pem")).write_bytes(certificate)
        if self.effect in ("complete", "newcerts_only", "index_only"):
            (ca / "index.txt").write_text(self.owner.index_row(serial))
        if self.effect in ("complete", "newcerts_only", "counter_only", "index_only"):
            (ca / "serial").write_text(format(int(serial, 16) + 1, "X") + "\n")
        if self.effect != "complete":
            raise Interrupted("executor response unavailable")


class IssuanceFixture(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name) / "qadra"
        self.ca = self.root / "data/ca"
        for path in (self.root / "config", self.ca / "certs", self.ca / "newcerts"):
            path.mkdir(mode=0o700, parents=True)
        for name, value in (("ca.crt.pem", ROOT), ("index.txt", b""),
                            ("serial", b"1000\n"), ("crlnumber", b"1001\n")):
            path = self.ca / name
            path.write_bytes(value)
            path.chmod(0o600)
        self.operation = self.ca / "operations" / OPERATION
        self.request = {"csr_pem": CSR, "purpose": "partner", "holder": "Declared Holder",
                        "reference": "case-reference-1", "criterion": "holder reviewed",
                        "operator_id": OPERATOR, "expected_root_fingerprint": digest(ROOT)}
        self.executor = Executor(self)

    def certificate(self, **overrides):
        facts = {"serial": "1000", "spki_sha256": digest(b"public key A"),
                 "root_fingerprint": digest(ROOT), "purpose": "partner"}
        facts.update(overrides)
        return encoded(facts)

    @staticmethod
    def index_row(serial="1000"):
        return "V\t301012000000Z\t\t" + serial + "\tunknown\t/CN=Declared Holder\n"

    def issue(self, request=None, executor=None):
        return issuance.issue(self.root, OPERATION, self.request if request is None else request,
                              self.executor if executor is None else executor)

    def resume(self, executor=None):
        return issuance.resume(self.root, OPERATION,
                               self.executor if executor is None else executor)

    def interrupt(self, effect):
        self.executor.effect = effect
        with self.assertRaises(Interrupted):
            self.issue()
        self.assertEqual(len(self.executor.calls), 1)
        self.assertTrue((self.operation / "journal.json").is_file())

    def assert_receipt(self, receipt):
        certificate = self.certificate()
        self.assertIs(type(receipt["recorded_at"]), int)
        self.assertGreater(receipt["recorded_at"], 0)
        self.assertEqual(receipt, {"operation_id": OPERATION, "serial": "1000",
                                  "fingerprint": digest(certificate), "purpose": "partner",
                                  "certificate_pem": certificate.decode("ascii"),
                                  "operator_id": self.request["operator_id"],
                                  "holder": self.request["holder"],
                                  "reference": self.request["reference"],
                                  "criterion": self.request["criterion"],
                                  "recorded_at": receipt["recorded_at"]})
        self.assertEqual((self.ca / "serial").read_bytes(), b"1001\n")
        self.assertEqual((self.ca / "crlnumber").read_bytes(), b"1001\n")
        self.assertEqual((self.ca / "ca.crt.pem").read_bytes(), ROOT)

    def state(self):
        return {path.relative_to(self.ca).as_posix(): path.read_bytes()
                for path in self.ca.rglob("*") if path.is_file()}
