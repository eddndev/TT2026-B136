"""Reuse the existing stopped-service backup fixture without rerunning its tests."""
import hashlib
import importlib
import json
import unittest

import test_deploy_backup


NAME = "backup-manifest.json"
PAYLOADS = ("database.dump", "redis.rdb", "private-state.tar.gz")
COMPLETE = b"database, Redis and private state captured while API stopped\n"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def module():
    return importlib.import_module("backup_manifest")


class ManifestFixture(unittest.TestCase):
    def setUp(self):
        self.fixture = test_deploy_backup.BackupTests(methodName="runTest")
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.root = self.fixture.root
        self.target = self.root / "releases" / ("v0.1.0-" + "a" * 40)
        self.target.mkdir(mode=0o700, parents=True)
        self.release = {"version": "v0.1.0", "commit": "a" * 40,
                        "schema": "b" * 64, "files": {"bin/despacho-cli": "c" * 64}}
        self.release_file = self.target / "release.json"
        self.write_json(self.release_file, self.release)
        (self.root / "current").symlink_to(self.target)
        self.schema_file = self.root / "config/schema"
        self.schema_file.write_text(self.release["schema"] + "\n")
        self.schema_file.chmod(0o600)

    def backup(self):
        return self.fixture.backup()

    def validate(self, backup):
        return module().validate(self.root, backup)

    def document(self, backup):
        path = backup / NAME
        self.assertTrue(path.is_file(), "prospective backup manifest was not published")
        return json.loads(path.read_text())

    def write_json(self, path, value):
        path.write_text(json.dumps(value, sort_keys=True) + "\n")
        path.chmod(0o600)

    def assert_no_completion(self):
        self.assertEqual(list((self.root / "backups").glob("*/COMPLETE")), [])

    def assert_rejected_unchanged(self, backup):
        path = backup / NAME
        original = path.read_bytes()
        with self.assertRaises((ValueError, OSError)):
            self.validate(backup)
        self.assertTrue(path.read_bytes() == original, "validation rewrote its input")

    def staged_payloads(self):
        backup = self.root / "backups/staged"
        backup.mkdir(mode=0o700)
        for name in PAYLOADS:
            path = backup / name
            path.write_bytes(b"private fixture payload")
            path.chmod(0o600)
        controller = self.root / "controller"
        controller.mkdir(mode=0o700)
        for name in ("runtime.py", "backup_manifest.py"):
            path = controller / name
            path.write_text('"""Fixture controller."""\n')
            path.chmod(0o600)
        return backup, controller
