"""Interrupted initialization must reconcile exact immutable credential trust."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
import provision


class InitializationTrustTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name) / "qadra"
        self.target = self.root / "releases/v1.0.0"
        self.target.mkdir(parents=True)
        (self.root / "config").mkdir()
        self.marker = self.root / "config/schema"
        (self.target / "release.json").write_text(json.dumps({"schema": "schema-digest"}))
        self.material = {
            "ca/ca.crt.pem": b"root PEM",
            "ca/private/ca.key.pem": b"private root test key",
            "ca/certs/qadra-server.crt.pem": b"signer PEM",
            "ca/private/qadra-server.key.pem": b"private test key",
            "ca/crl/crl.pem": b"CRL PEM",
            "tsa/tsa.crt.pem": b"TSA PEM",
            "tsa/private/tsa.key.pem": b"private TSA test key",
            "tsa/tsa-chain.pem": b"root PEM",
            "tsa/serial": b"1000\n",
        }
        for name, content in self.material.items():
            path = self.root / "data" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
        self.head = None
        self.query_error = False
        self.publications = 0
        self.key_scripts = []
        self.addCleanup(patch.stopall)
        patch.object(provision, "environment", return_value={}).start()
        patch.object(provision, "run", side_effect=self.command).start()

    def matching_trust(self, revision=1):
        return {"revision": revision, "root_der": b"root DER".hex(),
                "crl_der": b"CRL DER".hex(), "valid_now": True}

    def command(self, arguments, **kwargs):
        args = [str(value) for value in arguments]
        if args[0] == "psql":
            if self.query_error:
                raise subprocess.CalledProcessError(1, ["psql"])
            output = "" if self.head is None else json.dumps(self.head) + "\n"
            return subprocess.CompletedProcess(args, 0, stdout=output)
        if args[0] == "openssl":
            if not Path(args[args.index("-in") + 1]).is_file():
                raise subprocess.CalledProcessError(1, ["openssl"])
            output = b"root DER" if args[1] == "x509" else b"CRL DER"
            return subprocess.CompletedProcess(args, 0, stdout=output)
        if "credential-trust" in args:
            self.publications += 1
            if self.head is not None:
                raise RuntimeError("credential trust revision conflict")
            self.head = self.matching_trust()
        if args[0] == "bash":
            self.key_scripts.append(args)
        return subprocess.CompletedProcess(args, 0)

    def assert_private_material_preserved(self):
        self.assertEqual(self.key_scripts, [])
        for name, content in self.material.items():
            self.assertEqual((self.root / "data" / name).read_bytes(), content)

    def test_new_trust_is_published_once_before_the_schema_marker(self):
        provision.initialize(self.root, self.target)
        self.assertEqual(self.publications, 1)
        self.assertEqual(self.head, self.matching_trust())
        self.assertEqual(self.marker.read_text(), "schema-digest\n")
        self.assert_private_material_preserved()

    def test_marker_write_interruption_recovers_without_a_second_publication(self):
        original = Path.write_text

        def interrupted(path, value, *args, **kwargs):
            if path == self.marker:
                raise OSError("simulated marker write interruption")
            return original(path, value, *args, **kwargs)

        with patch.object(Path, "write_text", interrupted):
            with self.assertRaisesRegex(OSError, "marker write"):
                provision.initialize(self.root, self.target)
        self.assertEqual(self.publications, 1)
        self.assertFalse(self.marker.exists())
        provision.initialize(self.root, self.target)
        self.assertEqual(self.publications, 1)
        self.assertEqual(self.marker.read_text(), "schema-digest\n")
        self.assert_private_material_preserved()

    def test_existing_trust_must_match_both_local_der_objects(self):
        for field in ("root_der", "crl_der"):
            with self.subTest(field=field):
                self.head = self.matching_trust()
                self.head[field] = b"different public material".hex()
                with self.assertRaisesRegex(RuntimeError, "trust"):
                    provision.initialize(self.root, self.target)
                self.assertEqual(self.publications, 0)
                self.assertFalse(self.marker.exists())
                self.assert_private_material_preserved()

    def test_newer_trust_history_requires_explicit_maintenance(self):
        self.head = self.matching_trust(revision=2)
        with self.assertRaisesRegex(RuntimeError, "trust"):
            provision.initialize(self.root, self.target)
        self.assertEqual(self.publications, 0)
        self.assertFalse(self.marker.exists())
        self.assert_private_material_preserved()

    def test_expired_persisted_trust_cannot_complete_initialization(self):
        self.head = self.matching_trust()
        self.head["valid_now"] = False
        with self.assertRaisesRegex(RuntimeError, "trust"):
            provision.initialize(self.root, self.target)
        self.assertEqual(self.publications, 0)
        self.assertFalse(self.marker.exists())
        self.assert_private_material_preserved()

    def test_published_trust_with_missing_local_root_never_regenerates_keys(self):
        self.head = self.matching_trust()
        (self.root / "data/ca/ca.crt.pem").unlink()
        with self.assertRaises(subprocess.CalledProcessError):
            provision.initialize(self.root, self.target)
        self.assertEqual(self.publications, 0)
        self.assertEqual(self.key_scripts, [])
        self.assertFalse(self.marker.exists())
        self.assertEqual((self.root / "data/ca/private/qadra-server.key.pem").read_bytes(),
                         self.material["ca/private/qadra-server.key.pem"])

    def test_published_trust_requires_complete_signer_and_tsa_state(self):
        self.head = self.matching_trust()
        required = ("ca/private/ca.key.pem", "ca/certs/qadra-server.crt.pem",
                    "ca/private/qadra-server.key.pem", "tsa/tsa.crt.pem",
                    "tsa/private/tsa.key.pem", "tsa/tsa-chain.pem", "tsa/serial")
        for name in required:
            with self.subTest(missing=name):
                path = self.root / "data" / name
                path.unlink()
                try:
                    with self.assertRaisesRegex(RuntimeError, "PKI"):
                        provision.initialize(self.root, self.target)
                    self.assertEqual(self.publications, 0)
                    self.assertEqual(self.key_scripts, [])
                    self.assertFalse(self.marker.exists())
                    for other, content in self.material.items():
                        if other != name:
                            self.assertEqual((self.root / "data" / other).read_bytes(), content)
                finally:
                    path.write_bytes(self.material[name])
                    self.marker.unlink(missing_ok=True)
                    self.key_scripts.clear()

    def test_failed_trust_query_cannot_publish_or_write_the_marker(self):
        self.query_error = True
        with self.assertRaises(subprocess.CalledProcessError):
            provision.initialize(self.root, self.target)
        self.assertEqual(self.publications, 0)
        self.assertFalse(self.marker.exists())
        self.assert_private_material_preserved()


if __name__ == "__main__":
    unittest.main()
