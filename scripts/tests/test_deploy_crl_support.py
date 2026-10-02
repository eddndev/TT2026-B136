"""Private filesystem fixtures with controlled SQL and OpenSSL boundaries."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
import renew_crl


def fingerprint(value):
    return hashlib.sha256(value).hexdigest()


class ControlledRuntime:
    def __init__(self, owner):
        self.owner = owner
        self.events = []
        self.running = True
        self.fail_backup = False
        self.fail_start = False

    def stop(self):
        self.events.append("stop")
        self.running = False

    def backup(self):
        self.events.append("backup")
        if self.running:
            raise AssertionError("backup ran while the application was active")
        if self.fail_backup:
            raise RuntimeError("backup failed")
        destination = self.owner.root / "backups" / "original"
        destination.mkdir(mode=0o700, parents=True)
        for name in ("database.dump", "redis.rdb", "private-state.tar.gz", "COMPLETE"):
            path = destination / name
            path.write_bytes(b"isolated test backup")
            path.chmod(0o600)
        return destination

    def start(self, target):
        self.events.append("start")
        self.owner.assertEqual(target, self.owner.target)
        self.owner.assertFalse(self.owner.fence.exists())
        self.owner.assertEqual(self.owner.active_crl.read_bytes(), b"candidate CRL")
        self.owner.assertEqual(self.owner.head["crl_der"], self.owner.candidate["crl_der"])
        self.running = True
        if self.fail_start:
            raise RuntimeError("health failed")

    def check(self, target):
        self.events.append("check")
        if self.fail_start:
            raise RuntimeError("health failed")


class CrlFixture(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name) / "qadra"
        self.target = self.root / "releases" / ("v0.1.0-" + "a" * 40)
        self.target.mkdir(mode=0o700, parents=True)
        self.metadata = {"version": "v0.1.0", "commit": "a" * 40, "schema": "b" * 64}
        (self.target / "release.json").write_text(json.dumps(self.metadata))
        (self.root / "current").symlink_to(self.target)
        (self.root / "config").mkdir(mode=0o700)
        (self.root / "config/schema").write_text(self.metadata["schema"] + "\n")
        self.fence = self.root / "config/crl-maintenance.json"
        self.active_crl = self.root / "data/ca/crl/crl.pem"
        self.counter = self.root / "data/ca/crlnumber"
        self.original = {
            "ca/ca.crt.pem": b"original CA",
            "ca/private/ca.key.pem": b"original private CA key",
            "ca/index.txt": b"R\tcertificate\trevocation\t1000\tunknown\tsubject\n",
            "ca/index.txt.attr": b"unique_subject = no\n",
            "ca/serial": b"1002\n",
            "ca/crlnumber": b"1001\n",
            "ca/crl/crl.pem": b"original CRL",
            "ca/certs/qadra-server.crt.pem": b"original signer certificate",
            "ca/private/qadra-server.key.pem": b"original signer key",
            "tsa/tsa.crt.pem": b"original TSA certificate",
            "tsa/private/tsa.key.pem": b"original TSA key",
            "tsa/tsa-chain.pem": b"original TSA chain",
            "tsa/serial": b"1001\n",
        }
        for name, value in self.original.items():
            path = self.root / "data" / name
            path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            path.write_bytes(value)
            path.chmod(0o600)
        self.now = int(time.time())
        self.head = {
            "deployment_id": "10000000-0000-4000-8000-000000000001",
            "revision": 3,
            "root_der": b"root DER".hex(),
            "root_fingerprint": fingerprint(b"root DER"),
            "crl_der": b"original DER".hex(),
            "crl_digest": fingerprint(b"original DER"),
            "crl_number": 4096,
            "crl_this_update": self.now - 86400,
            "crl_next_update": self.now + 86400,
            "valid_from": self.now - 86400,
            "valid_until": self.now + 86400,
            "revoked_serials": ["1000"],
        }
        self.baseline = copy.deepcopy(self.head)
        self.candidate = {
            **copy.deepcopy(self.head),
            "crl_der": b"candidate DER".hex(),
            "crl_digest": fingerprint(b"candidate DER"),
            "crl_number": 4097,
            "crl_this_update": self.now,
            "crl_next_update": self.now + 7 * 86400,
            "valid_from": self.now,
            "valid_until": self.now + 7 * 86400,
        }
        self.runtime = ControlledRuntime(self)
        self.publications = []
        self.generated = []
        self.publish_error = None
        self.query_error = False
        for name, function in (("read_head", self.read_head), ("inspect", self.inspect),
                               ("generate", self.generate), ("publish", self.publish)):
            patcher = patch.object(renew_crl.material, name, side_effect=function)
            patcher.start()
            self.addCleanup(patcher.stop)

    def read_head(self, root):
        self.assertEqual(root, self.root)
        if self.query_error:
            raise RuntimeError("database unavailable")
        return copy.deepcopy(self.head)

    def inspect(self, root_cert, crl):
        self.assertEqual(Path(root_cert).read_bytes(), b"original CA")
        raw = Path(crl).read_bytes()
        if raw == b"original CRL":
            material = copy.deepcopy(self.baseline)
        elif raw == b"candidate CRL":
            material = copy.deepcopy(self.candidate)
        else:
            raise RuntimeError("CRL signature rejected")
        return {key: value for key, value in material.items()
                if key not in ("deployment_id", "revision")}

    def generate(self, root, target, operation):
        self.assertEqual((root, target), (self.root, self.target))
        self.assertEqual(self.active_crl.read_bytes(), b"original CRL")
        self.generated.append(operation)
        candidate = operation / "candidate.pem"
        candidate.write_bytes(b"candidate CRL")
        candidate.chmod(0o600)
        return {"path": candidate, "next_counter": format(self.candidate["crl_number"] + 1, "X")}

    def publish(self, root, target, candidate, expected):
        self.assertFalse(self.runtime.running)
        self.assertTrue(self.fence.exists())
        self.assertTrue((root / "backups/original/COMPLETE").is_file())
        self.assertEqual(self.active_crl.read_bytes(), Path(candidate).read_bytes())
        self.assertGreater(int(self.counter.read_text(), 16), int(self.candidate["crl_number"]))
        self.publications.append(expected)
        if self.publish_error == "before":
            raise RuntimeError("publication failed before commit")
        self.assertEqual(expected, self.head["revision"])
        self.head = {**copy.deepcopy(self.candidate), "revision": expected + 1}
        if self.publish_error == "after":
            raise RuntimeError("publication response lost after commit")
        return copy.deepcopy(self.head)

    def renew(self, expected=3):
        return renew_crl.renew(self.root, expected, runtime=self.runtime)

    def resume(self):
        operation = json.loads(self.fence.read_text())["operation_id"]
        return renew_crl.resume(self.root, operation, runtime=self.runtime)

    def assert_keys_preserved(self):
        for name, value in self.original.items():
            if name not in ("ca/crlnumber", "ca/crl/crl.pem"):
                self.assertEqual((self.root / "data" / name).read_bytes(), value, name)
        self.assertEqual((self.root / "current").resolve(), self.target)
        self.assertFalse((self.root / "previous").exists())
