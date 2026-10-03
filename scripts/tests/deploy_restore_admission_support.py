"""Reuse capture fixtures with explicit fake tool facts for read-only admission."""
from contextlib import ExitStack
import copy
import hashlib
import importlib
import os
from pathlib import Path
import stat
import subprocess
import unittest
from unittest.mock import patch

from deploy_backup_manifest_support import ManifestFixture, NAME, PAYLOADS
import backup_manifest
import bundle
import crl_journal
import provision
import restore_fence
import runtime


def fingerprint(value):
    return {"bytes": len(value), "sha256": hashlib.sha256(value).hexdigest()}


def role(administrative):
    return {"login": True, "superuser": administrative, "createdb": administrative,
            "createrole": administrative, "inherit": administrative,
            "replication": administrative, "bypassrls": administrative,
            "connection_limit": -1, "member_of": []}


def observed_facts():
    return {
        "postgres": {
            "server_version_num": 160015, "pg_dump_version": "16.15",
            "pg_restore_version": "16.15", "dump_format": "custom", "database": "qadra",
            "schema": "public", "owner": "qadra_admin", "runtime_role": "qadra_runtime",
            "encoding": "UTF8", "collate": "C", "ctype": "C", "locale_provider": "libc",
            "roles": {"qadra_admin": role(True), "qadra_runtime": role(False)},
            "tools": {name: fingerprint(("fake PostgreSQL " + name).encode())
                      for name in ("server", "pg_dump", "pg_restore")},
        },
        "redis": {
            "engine": "redis", "server_version": "7.4.11", "cli_version": "7.4.11",
            "rdb_version": 12, "database": 0, "appendonly": True,
            "tools": {name: fingerprint(("fake Redis " + name).encode())
                      for name in ("server", "redis_cli", "redis_check_rdb")},
        },
    }


class AdmissionFixture(unittest.TestCase):
    def setUp(self):
        fixture = ManifestFixture()
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        self.fixture = fixture
        self.root, self.target = fixture.root, fixture.target
        files = {
            "bin/despacho-cli": b"fake application binary", "web/index.html": b"<html>fixture",
            "lib/libqpdf.so.30.4.1": b"fake library", "pki/tsa.cnf": b"fake TSA configuration",
            "bin/ffmpeg": b"fake media decoder", "bin/ffprobe": b"fake media probe",
            "migrations/0001_fixture.sql": b"SELECT 1;\n",
        }
        for name, value in files.items():
            path = self.target / name
            path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            path.write_bytes(value)
            path.chmod(0o700 if name.startswith("bin/") else 0o600)
        fixture.release["files"] = {name: fingerprint(value)["sha256"] for name, value in files.items()}
        fixture.release["schema"] = bundle.schema_digest(self.target / "migrations")
        fixture.write_json(fixture.release_file, fixture.release)
        fixture.schema_file.write_text(fixture.release["schema"] + "\n")
        execute = fixture.fixture.execute

        def capture(arguments, **kwargs):
            result = execute(arguments, **kwargs)
            arguments = [str(value) for value in arguments]
            if arguments[0] == "pg_dump":
                Path(arguments[arguments.index("--file") + 1]).write_bytes(b"PGDMP-fake-content")
            elif arguments[0] == "redis-cli" and "--rdb" in arguments:
                Path(arguments[arguments.index("--rdb") + 1]).write_bytes(b"REDIS0012-fake-content")
            return result

        fixture.fixture.execute = capture
        self.backup = fixture.backup()
        self.controller = Path(runtime.__file__).resolve().parent
        self.observed = observed_facts()
        manifest = backup_manifest.validate(self.root, self.backup)
        self.descriptor = {
            "format": "qadra-restore-compatibility", "version": 1,
            "backup": {"id": manifest["backup_id"], "captured_at": manifest["captured_at"],
                       "files": self.files()},
            "release": manifest["source"]["release"],
            "controller": manifest["source"]["controller"],
            **copy.deepcopy(self.observed),
            "audit_predecessor": {"sequence": 7, "head": "d" * 64},
        }
        receipts = self.root / "receipts"
        receipts.mkdir(mode=0o700)
        source_controller = self.controller
        self.controller = receipts / "controller"
        self.controller.mkdir(mode=0o700)
        for source in source_controller.glob("*.py"):
            target = self.controller / source.name
            target.write_bytes(source.read_bytes())
            target.chmod(0o600)
        self.descriptor_path = receipts / "restore-compatibility.json"
        self.write_descriptor()
        self.marker = self.root / "maintenance/restore/active.json"
        self.marker.parent.mkdir(parents=True, mode=0o700)
        self.marker.write_text('{"operation_id":"10000000-0000-4000-8000-000000000001"}\n')
        self.marker.chmod(0o600)

    def files(self):
        return {name: fingerprint((self.backup / name).read_bytes())
                for name in (*PAYLOADS, "COMPLETE", NAME)}

    def write_descriptor(self):
        self.fixture.write_json(self.descriptor_path, self.descriptor)
        self.pin = fingerprint(self.descriptor_path.read_bytes())["sha256"]

    def refresh_binding(self):
        manifest = self.fixture.document(self.backup)
        manifest["payloads"] = {name: fingerprint((self.backup / name).read_bytes())
                                for name in PAYLOADS}
        self.fixture.write_json(self.backup / NAME, manifest)
        self.descriptor["backup"]["files"] = self.files()
        self.write_descriptor()

    def snapshot(self):
        result = {}
        for path in sorted(self.root.rglob("*")):
            info = path.lstat()
            value = os.readlink(path) if path.is_symlink() else (
                fingerprint(path.read_bytes()) if stat.S_ISREG(info.st_mode) else None)
            result[str(path.relative_to(self.root))] = (
                info.st_mode, info.st_ino, info.st_mtime_ns, value)
        return result

    def invoke(self, *, expected=None):
        edges = [(subprocess, "run"), (subprocess, "Popen"), (runtime, "Runtime"),
                 (runtime, "run"), (bundle, "extract_bundle"), (provision, "initialize"),
                 (restore_fence, "enter"), (crl_journal, "write")]
        with ExitStack() as stack:
            for owner, name in edges:
                stack.enter_context(patch.object(
                    owner, name, side_effect=AssertionError("compatibility performed an effect")))
            observed = copy.deepcopy(self.observed)
            try:
                return importlib.import_module("restore_admission").admit(
                    self.root, self.backup, self.descriptor_path,
                    expected_descriptor_sha256=self.pin if expected is None else expected,
                    controller_directory=self.controller, observed=observed)
            finally:
                self.assertTrue(observed == self.observed, "compatibility changed supplied observations")

    def rejected(self, *, expected=None):
        before = self.snapshot()
        with self.assertRaises((ValueError, OSError)):
            self.invoke(expected=expected)
        self.assertTrue(self.snapshot() == before, "read-only rejection changed deployment files")
