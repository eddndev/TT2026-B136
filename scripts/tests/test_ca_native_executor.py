"""Native CA boundary checks; real crypto cases require explicit built binaries."""
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
import unittest
from unittest.mock import patch

from ca_issuance_support import OPERATION, REPOSITORY, journal
from ca_native_executor_support import NativeFixture
from native_executor import NativeExecutor


class NativeExecutorBoundaryTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.ca = self.root / "data/ca"
        self.ca.mkdir(parents=True)
        self.executor = NativeExecutor(Path(sys.executable), Path(sys.executable), REPOSITORY / "pki")
        self.request = {"csr_sha256": "a" * 64, "spki_sha256": "b" * 64,
                        "root_fingerprint": "c" * 64}
        self.certificate = {"serial": "1000", "fingerprint": "d" * 64,
                            "spki_sha256": "b" * 64, "root_fingerprint": "c" * 64,
                            "purpose": "partner", "subject": "CN=Public Holder",
                            "not_before": 1, "not_after": 2}

    def test_request_uses_fixed_inspector_command_and_only_returns_admission_fields(self):
        def run(args, **kwargs):
            self.assertEqual(args, [Path(sys.executable).resolve(), "inspect-public"])
            self.assertEqual(json.loads(kwargs["input_bytes"]),
                             {"kind": "request", "material_pem": "public CSR", "root_pem": "public root"})
            return json.dumps({**self.request, "subject": "CN=Public Holder",
                               "issuer_policy_matches": True}).encode()
        with patch.object(self.executor, "_run", side_effect=run):
            self.assertEqual(self.executor.admit(b"public CSR", b"public root"), self.request)
            self.assertEqual(self.executor.describe_request(b"public CSR", b"public root"),
                             {**self.request, "subject": "CN=Public Holder"})

    def test_certificate_contract_filters_history_fields_without_losing_inventory_metadata(self):
        with patch.object(self.executor, "_run", return_value=json.dumps(self.certificate).encode()):
            self.assertEqual(self.executor.describe_certificate(b"public leaf", b"public root"), self.certificate)
            self.assertEqual(self.executor.inspect_certificate(b"public leaf", b"public root"),
                             {key: self.certificate[key] for key in
                              ("serial", "fingerprint", "spki_sha256", "root_fingerprint", "purpose")})

    def test_issuance_uses_private_temporary_public_csr_and_inherited_lock(self):
        retained = []
        with journal.locked(self.root) as lock_fd:
            def run(args, **kwargs):
                self.assertEqual(args[:7], [Path(sys.executable).resolve(), "--json", "pki",
                    "--scripts-dir", (REPOSITORY / "pki").resolve(), "issue-request", "--csr"])
                path = Path(args[7])
                retained.append(path)
                self.assertEqual(path.read_bytes(), b"public CSR")
                self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
                self.assertEqual(stat.S_IMODE(path.parent.stat().st_mode), 0o700)
                self.assertEqual(args[8:], ["--name", OPERATION, "--purpose", "partner"])
                self.assertEqual(kwargs["pass_fds"], (lock_fd,))
                self.assertEqual(kwargs["env"]["PKI_CA_DIR"], str(self.ca))
                return b"{}"
            with patch.object(self.executor, "_run", side_effect=run):
                self.executor.issue(self.ca, OPERATION, b"public CSR", "partner", lock_fd=lock_fd)
        self.assertTrue(retained)
        self.assertTrue(all(not path.exists() and not path.parent.exists() for path in retained))

    def test_bounds_and_noncanonical_names_fail_before_process_execution(self):
        with patch.object(self.executor, "_run", side_effect=AssertionError("process must not start")):
            for material, root in ((b"x" * 16385, b"root"), (b"CSR", b"x" * 16385), (b"", b"root")):
                with self.assertRaises(ValueError):
                    self.executor.admit(material, root)
            with journal.locked(self.root) as lock_fd:
                for name, purpose in (("../outside", "partner"), (OPERATION, "root")):
                    with self.assertRaises(ValueError):
                        self.executor.issue(self.ca, name, b"CSR", purpose, lock_fd=lock_fd)

    def test_revocation_passes_only_private_temporary_certificate_and_shared_lock(self):
        retained = []
        with journal.locked(self.root) as lock_fd:
            def run(args, **kwargs):
                self.assertEqual(args[:2], ["/bin/bash", (REPOSITORY / "pki/revoke.sh").resolve()])
                self.assertEqual(len(args), 3)
                path = Path(args[2])
                retained.append(path)
                self.assertEqual(path.read_bytes(), b"public leaf")
                self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
                self.assertEqual(stat.S_IMODE(path.parent.stat().st_mode), 0o700)
                self.assertEqual(kwargs["pass_fds"], (lock_fd,))
                self.assertEqual(kwargs["env"]["PKI_CA_DIR"], str(self.ca))
                return b"local revocation only"
            with patch.object(self.executor, "_run", side_effect=run):
                self.executor.revoke(self.ca, b"public leaf", lock_fd=lock_fd)
        self.assertTrue(retained)
        self.assertTrue(all(not path.exists() for path in retained))

    def test_revocation_rejects_unbounded_public_material_and_wrong_lock_before_execution(self):
        with patch.object(self.executor, "_run", side_effect=AssertionError("must not execute")):
            with journal.locked(self.root) as lock_fd:
                for certificate in (b"", b"x" * 16385, b"\xff"):
                    with self.assertRaises(ValueError):
                        self.executor.revoke(self.ca, certificate, lock_fd=lock_fd)
                with tempfile.TemporaryFile() as other:
                    with self.assertRaises(ValueError):
                        self.executor.revoke(self.ca, b"public leaf", lock_fd=other.fileno())

    def fake_inspector(self, body):
        path = self.root / "inspector"
        path.write_text("#!" + str(Path(sys.executable).resolve()) + "\n" + body, encoding="ascii")
        path.chmod(0o700)
        return NativeExecutor(path, Path(sys.executable), REPOSITORY / "pki", timeout=0.25)

    def test_failure_timeout_and_oversized_output_return_generic_errors(self):
        for body in ("import sys; sys.stderr.write('PRIVATE DETAIL'); sys.exit(1)\n",
                     "import time; time.sleep(10)\n", "print('x' * 70000)\n"):
            with self.subTest(body=body):
                executor = self.fake_inspector(body)
                with self.assertRaises(RuntimeError) as error:
                    executor.admit(b"public CSR", b"public root")
                self.assertNotIn("PRIVATE DETAIL", str(error.exception))
                self.assertNotIn(str(self.root), str(error.exception))


class CaNativeCryptoTests(NativeFixture):
    def test_existing_tsa_is_visible_and_downloadable_but_not_an_issuance_profile(self):
        import inventory
        import portal
        from ca_issuance_support import OPERATOR
        self.issue("partner")
        self.command(["bash", REPOSITORY / "pki/issue-tsa-cert.sh"],
                     env={**os.environ, "PKI_CA_DIR": str(self.ca),
                          "TSA_DIR": str(self.scratch / "existing-tsa")})
        items = inventory.list_certificates(self.root, self.executor, now_unix=1800000000)
        self.assertEqual({row["purpose"] for row in items}, {"partner", "time-stamping"})
        tsa = next(row for row in items if row["purpose"] == "time-stamping")
        downloaded = portal.execute(self.root, OPERATOR, {"action": "certificate",
            "serial": tsa["serial"], "fingerprint": tsa["fingerprint"]}, self.executor,
            self.admission["root_fingerprint"])
        self.assertEqual(downloaded["certificate_pem"].encode(),
                         (self.ca / "newcerts" / (tsa["serial"] + ".pem")).read_bytes())
        with self.assertRaises(RuntimeError):
            self.executor.inspect_certificate(downloaded["certificate_pem"].encode(), self.root_pem)

    def test_real_revocation_response_loss_recovers_without_crl_publication(self):
        import revocation
        from ca_issuance_support import OPERATOR, Interrupted
        _, issued = self.issue("partner")
        request = {"serial": issued["serial"], "fingerprint": issued["fingerprint"],
            "reason": "Native disposable cancellation", "operator_id": OPERATOR,
            "expected_root_fingerprint": self.admission["root_fingerprint"]}
        before = {name: (self.ca / name).read_bytes() for name in ("serial", "crlnumber", "ca.crt.pem")}
        original = self.executor.revoke
        def interrupted(ca, certificate, *, lock_fd):
            original(ca, certificate, lock_fd=lock_fd)
            raise Interrupted("native revocation response lost")
        with self.inherited_openssl(), patch.object(self.executor, "revoke", side_effect=interrupted):
            with self.assertRaises(Interrupted):
                revocation.revoke(self.root, OPERATION, request, self.executor)
        self.assertGreaterEqual(len(self.lock_observations.read_text().splitlines()), 2)
        with patch.object(self.executor, "revoke", side_effect=AssertionError("must not revoke again")):
            receipt = revocation.revoke(self.root, OPERATION, request, self.executor)
            self.assertEqual(revocation.revoke(self.root, OPERATION, request, self.executor), receipt)
        self.assertEqual((receipt["local_status"], receipt["published_status"]), ("revoked", "pending"))
        self.assertEqual({name: (self.ca / name).read_bytes() for name in before}, before)
        self.assertFalse((self.ca / "crl/crl.pem").exists())

    def test_real_holder_csr_issues_both_profiles_without_creating_holder_keys(self):
        before_holder = self.holder_key.read_bytes()
        before_private = {path.name: path.read_bytes() for path in (self.ca / "private").iterdir()}
        with self.inherited_openssl():
            for purpose in ("partner", "document-signing"):
                operation, receipt = self.issue(purpose)
                description = self.executor.describe_certificate(receipt["certificate_pem"].encode(), self.root_pem)
                self.assertEqual(description["purpose"], purpose)
                self.assertEqual(description["fingerprint"], receipt["fingerprint"])
                self.assertEqual(description["spki_sha256"], self.admission["spki_sha256"])
                self.assertIs(type(description["not_before"]), int)
                self.assertIs(type(description["not_after"]), int)
                self.assertEqual(self.resume(operation), receipt)
        self.assertGreaterEqual(len(self.lock_observations.read_text().splitlines()), 2)
        self.assertEqual(self.holder_key.read_bytes(), before_holder)
        self.assertEqual({path.name: path.read_bytes() for path in (self.ca / "private").iterdir()}, before_private)

    def test_real_issued_leaf_is_recovered_after_response_loss_without_second_serial(self):
        from ca_issuance_support import Interrupted
        original = self.executor.issue
        def interrupted(ca, name, request, purpose, *, lock_fd):
            original(ca, name, request, purpose, lock_fd=lock_fd)
            (ca / "certs" / (name + ".crt.pem")).unlink()
            raise Interrupted("native response lost")
        with patch.object(self.executor, "issue", side_effect=interrupted):
            with self.assertRaises(Interrupted):
                self.issue("partner", operation=OPERATION)
        counter = (self.ca / "serial").read_bytes()
        index = (self.ca / "index.txt").read_bytes()
        with patch.object(self.executor, "issue", side_effect=AssertionError("must not sign again")):
            receipt = self.resume(OPERATION)
            self.assertEqual(self.resume(OPERATION), receipt)
        self.assertEqual((self.ca / "serial").read_bytes(), counter)
        self.assertEqual((self.ca / "index.txt").read_bytes(), index)


if __name__ == "__main__":
    unittest.main()
