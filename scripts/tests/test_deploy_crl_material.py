"""CRL maintenance must stage changes without rewriting live CA material."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "ops/deploy"))
import crl_material


class MaterialTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.temp.cleanup)
        cls.base = Path(cls.temp.name) / "seed"
        cls.ca = cls.base / "data/ca"
        cls.base.mkdir()
        cls.env = {**os.environ, "PKI_CA_DIR": str(cls.ca), "LC_ALL": "C"}
        for script, args in (("init-ca.sh", []), ("issue-cert.sh", ["Revoked Test"]),
                             ("revoke.sh", [str(cls.ca / "certs/revoked-test.crt.pem")]),
                             ("gen-crl.sh", [])):
            subprocess.run(["bash", str(REPO / "pki" / script), *args], env=cls.env,
                           check=True, capture_output=True, timeout=30)

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "qadra"
        shutil.copytree(self.base, self.root)
        self.ca = self.root / "data/ca"
        self.target = self.root / "releases/test"
        shutil.copytree(REPO / "pki", self.target / "pki")
        self.operation = self.root / "maintenance/crl/op"
        self.operation.mkdir(parents=True, mode=0o700)

    def inspect(self, crl=None):
        return crl_material.inspect(self.ca / "ca.crt.pem", crl or self.ca / "crl/crl.pem")

    def snapshot(self):
        return {str(p.relative_to(self.ca)): hashlib.sha256(p.read_bytes()).hexdigest()
                for p in self.ca.rglob("*") if p.is_file()}

    def test_generation_preserves_live_files_and_revoked_entries(self):
        before, original = self.snapshot(), self.inspect()
        result = crl_material.generate(self.root, self.target, self.operation)
        candidate = self.inspect(result["path"])
        self.assertEqual(self.snapshot(), before)
        self.assertEqual(candidate["root_der"], original["root_der"])
        self.assertEqual(candidate["revoked_serials"], original["revoked_serials"])
        self.assertEqual(len(candidate["revoked_serials"]), 1)
        self.assertEqual(candidate["crl_number"], original["crl_number"] + 1)
        self.assertEqual(int(result["next_counter"], 16), candidate["crl_number"] + 1)
        self.assertLessEqual(original["crl_this_update"], candidate["crl_this_update"])
        self.assertGreater(candidate["valid_until"], candidate["valid_from"])
        self.assertEqual(Path(result["path"]).stat().st_mode & 0o777, 0o600)
        self.assertFalse(any(p.name.endswith("key.pem") for p in self.operation.rglob("*")))

    def test_generated_private_files_resist_permissive_umask(self):
        previous = os.umask(0)
        try:
            crl_material.generate(self.root, self.target, self.operation)
        finally:
            os.umask(previous)
        for path in self.operation.rglob("*"):
            self.assertFalse(path.is_symlink())
            self.assertEqual(path.stat().st_mode & 0o777, 0o700 if path.is_dir() else 0o600,
                             str(path.relative_to(self.operation)))

    def test_invalid_signature_is_rejected(self):
        other = self.operation / "other.pem"
        subprocess.run(["openssl", "req", "-new", "-x509", "-newkey", "rsa:2048",
                        "-nodes", "-subj", "/CN=Other", "-days", "1", "-keyout",
                        str(self.operation / "other.key"), "-out", str(other)],
                       capture_output=True, check=True, timeout=20)
        with self.assertRaises((ValueError, RuntimeError, subprocess.SubprocessError)):
            crl_material.inspect(other, self.ca / "crl/crl.pem")

    def test_missing_index_never_mutates_live_material(self):
        (self.ca / "index.txt").unlink()
        before = self.snapshot()
        with self.assertRaises((ValueError, RuntimeError, OSError)):
            crl_material.generate(self.root, self.target, self.operation)
        self.assertEqual(self.snapshot(), before)

    def test_inspection_rejects_oversized_public_material(self):
        oversized = self.operation / "large.pem"
        oversized.write_bytes(b"x" * (1024 * 1024 + 1))
        with self.assertRaisesRegex(ValueError, "material"):
            crl_material.inspect(self.ca / "ca.crt.pem", oversized)

    def head(self):
        value = self.inspect()
        value.pop("revoked_serials")
        return {**value, "deployment_id": "b4ce9d5b-7398-49ce-81aa-b3c20c6b6a58", "revision": 3}

    def test_query_preserves_large_crl_number_and_checks_digest(self):
        head = self.head()
        head["crl_number"] = "18446744073709551614"
        result = subprocess.CompletedProcess([], 0, stdout=json.dumps(head))
        with patch.object(crl_material, "run", return_value=result), \
                patch.object(crl_material, "environment", return_value={}):
            actual = crl_material.read_head(self.root)
        self.assertEqual(actual["crl_number"], 18446744073709551614)
        self.assertEqual(actual["deployment_id"], head["deployment_id"])
        self.assertEqual(actual["revision"], 3)

    def test_query_refuses_absent_malformed_or_corrupt_trust(self):
        valid = self.head()
        bad_digest = {**valid, "crl_digest": "0" * 64}
        for value in (None, [], {}, bad_digest, {**valid, "revision": True},
                      {**valid, "deployment_id": "invalid"},
                      {**valid, "valid_until": valid["valid_from"] - 1},
                      {**valid, "crl_number": "18446744073709551616"}):
            with self.subTest(value=type(value).__name__), \
                    patch.object(crl_material, "environment", return_value={}), \
                    patch.object(crl_material, "run", return_value=subprocess.CompletedProcess(
                        [], 0, stdout="" if value is None else json.dumps(value))):
                with self.assertRaises((ValueError, RuntimeError)):
                    crl_material.read_head(self.root)

    def test_publish_uses_exact_release_and_expected_revision(self):
        with patch.object(crl_material, "environment", return_value={"safe": "test"}) as env, \
                patch.object(crl_material, "run") as command:
            crl_material.publish(self.root, self.target, self.operation / "candidate.pem", 3)
        env.assert_called_once_with(self.root, admin=True)
        args = [str(a) for a in command.call_args.args[0]]
        self.assertEqual(args[0], str(self.target / "bin/despacho-cli"))
        self.assertEqual(args[args.index("--expected-revision") + 1], "3")
        self.assertEqual(args[args.index("--crl") + 1], str(self.operation / "candidate.pem"))
        self.assertEqual(command.call_args.kwargs["env"]["LD_LIBRARY_PATH"], str(self.target / "lib"))
        self.assertTrue(command.call_args.kwargs["capture_output"])
        self.assertLessEqual(command.call_args.kwargs["timeout"], 60)


if __name__ == "__main__":
    unittest.main()
