"""Prospective backup metadata binds capture bytes without admitting a restore."""
import copy
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import unittest
from unittest.mock import patch
import uuid

from deploy_backup_manifest_support import (
    COMPLETE, NAME, PAYLOADS, ManifestFixture, digest, module,
)
import crl_backup
import runtime
import test_deploy_host


class BackupManifestTests(ManifestFixture):
    def test_host_prepare_installs_complete_importable_controller_sources(self):
        source = Path(runtime.__file__).resolve().parent
        expected = {path.name for path in source.glob("*.py")}
        copied = set()
        verified = []
        copy_file = test_deploy_host.host.shutil.copyfile
        execute = subprocess.run

        def copy_and_verify(original, destination):
            result = copy_file(original, destination)
            copied.add(Path(destination).name)
            if copied == expected:
                script = """import importlib, pathlib, sys
tools = pathlib.Path(sys.argv[1]).resolve()
sys.path.insert(0, str(tools))
for name in ('runtime', 'crl_backup', 'backup_manifest', 'backup_manifest_files',
             'backup_manifest_schema', 'bundle'):
    loaded = importlib.import_module(name)
    assert pathlib.Path(loaded.__file__).resolve().parent == tools
"""
                process = execute([sys.executable, "-I", "-B", "-c", script,
                                   str(Path(destination).parent)],
                                  capture_output=True, timeout=10)
                self.assertEqual(process.returncode, 0, "installed controller import failed")
                verified.append(True)
            return result

        # Reuse the existing host fixture's mocked binaries, sockets and systemd.
        probe = test_deploy_host.HostTests("test_prepare_keeps_every_nginx_temporary_directory_private")
        with patch.object(test_deploy_host.host.shutil, "copyfile", side_effect=copy_and_verify):
            probe.debug()
        self.assertEqual(copied, expected)
        self.assertEqual(verified, [True])

    def test_runtime_publishes_private_versioned_hashes_and_declared_source_identity(self):
        backup = self.backup()
        document = self.document(backup)
        self.assertEqual(set(document), {"format", "version", "backup_id", "captured_at",
                                         "source", "payloads"})
        self.assertEqual(document["format"], "qadra-private-backup")
        self.assertIs(type(document["version"]), int)
        self.assertEqual(document["version"], 1)
        identity = uuid.UUID(document["backup_id"])
        self.assertNotEqual(identity.int, 0)
        self.assertEqual(str(identity), document["backup_id"])
        self.assertRegex(document["captured_at"], r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
        self.assertEqual(set(document["payloads"]), set(PAYLOADS))
        for name in PAYLOADS:
            self.assertEqual(document["payloads"][name], {
                "bytes": (backup / name).stat().st_size, "sha256": digest(backup / name)})
        source = document["source"]
        self.assertEqual(set(source), {"initialized", "schema", "release", "controller"})
        self.assertIs(source["initialized"], True)
        self.assertEqual(source["schema"], self.release["schema"])
        self.assertEqual(source["release"], {
            **{key: self.release[key] for key in ("version", "commit", "schema")},
            "manifest_sha256": digest(self.release_file)})
        controller = Path(runtime.__file__).resolve().parent
        expected = {path.name: {"bytes": path.stat().st_size, "sha256": digest(path)}
                    for path in sorted(controller.glob("*.py"))}
        self.assertEqual(source["controller"], expected)
        self.assertEqual(self.validate(backup), document)
        self.assertEqual((backup / "COMPLETE").read_bytes(), COMPLETE)
        for path in backup.iterdir():
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
        encoded = json.dumps(document)
        for private in self.fixture.config.values():
            if isinstance(private, str):
                self.assertFalse(private in encoded, "manifest copied private configuration")

    def test_preinitialization_is_explicit_and_never_invents_a_release_or_schema(self):
        (self.root / "current").unlink()
        self.schema_file.unlink()
        backup = self.backup()
        source = self.document(backup)["source"]
        self.assertEqual((source["initialized"], source["schema"], source["release"]),
                         (False, None, None))
        self.assertEqual(self.validate(backup)["source"], source)

    def test_manifest_and_its_directory_are_flushed_before_complete_is_visible(self):
        flushed_manifest = False
        flushed_directory = False
        observed_completion = False
        fsync = os.fsync
        rename = Path.rename

        def track_flush(descriptor):
            nonlocal flushed_manifest, flushed_directory
            information = os.fstat(descriptor)
            for path in (self.root / "backups").glob("*/*"):
                current = path.stat()
                if (path.name in (NAME, ".backup-manifest.tmp")
                        and (current.st_dev, current.st_ino) == (information.st_dev, information.st_ino)):
                    flushed_manifest = True
                if path.name == NAME and not (path.parent / "COMPLETE").exists():
                    parent = path.parent.stat()
                    if (stat.S_ISDIR(information.st_mode)
                            and (parent.st_dev, parent.st_ino) == (information.st_dev, information.st_ino)):
                        flushed_directory = True
            return fsync(descriptor)

        def complete(source, destination):
            nonlocal observed_completion
            if Path(destination).name == "COMPLETE":
                self.assertTrue(flushed_manifest, "completion preceded manifest fsync")
                self.assertTrue(flushed_directory, "completion preceded manifest directory fsync")
                self.assertTrue((Path(destination).parent / NAME).is_file())
                observed_completion = True
            return rename(source, destination)

        with patch.object(runtime.os, "fsync", side_effect=track_flush), patch.object(Path, "rename", complete):
            self.backup()
        self.assertTrue(observed_completion)

    def test_manifest_failure_never_publishes_a_completion_marker(self):
        with patch.object(module(), "publish", side_effect=OSError("manifest flush unavailable")):
            with self.assertRaises(OSError):
                self.backup()
        self.assert_no_completion()

    def test_inconsistent_or_escaping_source_identity_rejects_without_completion(self):
        self.schema_file.write_text("d" * 64 + "\n")
        with self.assertRaises(ValueError):
            self.backup()
        self.assert_no_completion()
        self.schema_file.write_text(self.release["schema"] + "\n")
        (self.root / "current").unlink()
        (self.root / "current").symlink_to(self.root / "config", target_is_directory=True)
        with self.assertRaises(ValueError):
            self.backup()
        self.assert_no_completion()

    def test_reader_rejects_changed_missing_linked_or_public_payloads(self):
        for fault in ("changed", "missing", "symlink", "mode"):
            with self.subTest(fault=fault):
                backup = self.backup()
                path = backup / "database.dump"
                original = path.read_bytes()
                if fault == "changed":
                    path.write_bytes(bytes([original[0] ^ 1]) + original[1:])
                elif fault == "missing":
                    path.unlink()
                elif fault == "symlink":
                    outside = self.root / ("unrelated-" + backup.name)
                    outside.write_bytes(original)
                    outside.chmod(0o600)
                    path.unlink()
                    path.symlink_to(outside)
                else:
                    path.chmod(0o644)
                self.assert_rejected_unchanged(backup)

    def test_reader_bounds_json_and_rejects_ambiguous_versioned_shapes(self):
        backup = self.backup()
        baseline = self.document(backup)
        candidates = []
        for field, value in (("version", True), ("version", 2), ("backup_id", str(uuid.UUID(int=0))),
                             ("captured_at", "2026-02-30T00:00:00Z"), ("unexpected", "field")):
            changed = copy.deepcopy(baseline)
            changed[field] = value
            candidates.append(json.dumps(changed).encode())
        for size in (0, True, 8 * 1024 ** 3 + 1):
            changed = copy.deepcopy(baseline)
            changed["payloads"]["database.dump"]["bytes"] = size
            candidates.append(json.dumps(changed).encode())
        changed = copy.deepcopy(baseline)
        changed["source"]["controller"]["../outside.py"] = {"bytes": 1, "sha256": "a" * 64}
        candidates.append(json.dumps(changed).encode())
        changed = copy.deepcopy(baseline)
        changed["source"]["initialized"] = False
        candidates.append(json.dumps(changed).encode())
        changed = copy.deepcopy(baseline)
        changed["source"]["schema"] = "d" * 64
        candidates.append(json.dumps(changed).encode())
        changed = copy.deepcopy(baseline)
        changed["payloads"]["COMPLETE"] = {"bytes": len(COMPLETE), "sha256": "a" * 64}
        candidates.append(json.dumps(changed).encode())
        candidates.extend((b"[]", b" " * (64 * 1024 + 1),
                           json.dumps(baseline).replace('"version": 1', '"version": 1, "version": 1').encode()))
        for encoded in candidates:
            with self.subTest(case=len(encoded)):
                (backup / NAME).write_bytes(encoded)
                self.assert_rejected_unchanged(backup)

    def test_publisher_bounds_controller_and_payload_bytes_before_hashing(self):
        backup, controller = self.staged_payloads()
        path = controller / "runtime.py"
        with path.open("r+b") as stream:
            stream.truncate(256 * 1024 + 1)
        with self.assertRaises(ValueError):
            module().publish(self.root, backup, controller_directory=controller)
        self.assertFalse((backup / NAME).exists())
        path.write_bytes(b"fixture")
        with (backup / "database.dump").open("r+b") as stream:
            stream.truncate(8 * 1024 ** 3 + 1)
        with self.assertRaises(ValueError):
            module().publish(self.root, backup, controller_directory=controller)
        self.assertFalse((backup / NAME).exists())
        self.assertFalse((backup / "COMPLETE").exists())

    def test_manifest_links_modes_and_managed_directory_boundaries_are_rejected(self):
        backup = self.backup()
        document = (backup / NAME).read_bytes()
        outside = self.root / "unrelated-manifest"
        outside.write_bytes(document)
        outside.chmod(0o600)
        (backup / NAME).unlink()
        (backup / NAME).symlink_to(outside)
        with self.assertRaises((ValueError, OSError)):
            self.validate(backup)
        self.assertTrue(outside.read_bytes() == document)
        (backup / NAME).unlink()
        (backup / NAME).write_bytes(document)
        (backup / NAME).chmod(0o644)
        self.assert_rejected_unchanged(backup)
        (backup / NAME).chmod(0o600)
        linked = self.root / "backups/linked"
        linked.symlink_to(backup, target_is_directory=True)
        with self.assertRaises((ValueError, OSError)):
            self.validate(linked)

    def test_new_crl_receipt_binds_manifest_bytes_and_rejects_its_removal(self):
        backup = self.backup()
        receipt = crl_backup.capture(self.root, backup)
        self.assertEqual(set(receipt), {"path", "files"})
        self.assertEqual(set(receipt["files"]), {*PAYLOADS, "COMPLETE", NAME})
        crl_backup.validate(self.root, receipt)
        path = backup / NAME
        original = path.read_bytes()
        path.write_bytes(original + b"\n")
        with self.assertRaises(ValueError):
            crl_backup.validate(self.root, receipt)
        path.unlink()
        with self.assertRaises(ValueError):
            crl_backup.validate(self.root, receipt)

    def test_historical_four_file_receipt_is_not_upgraded_when_manifest_appears(self):
        backup = self.backup()
        prospective = (backup / NAME).read_bytes()
        (backup / NAME).unlink()
        historical = crl_backup.capture(self.root, backup)
        original = copy.deepcopy(historical)
        self.assertEqual(set(historical["files"]), {*PAYLOADS, "COMPLETE"})
        crl_backup.validate(self.root, historical)
        with self.assertRaises((ValueError, OSError)):
            self.validate(backup)
        (backup / NAME).write_bytes(prospective)
        (backup / NAME).chmod(0o600)
        with self.assertRaises(ValueError):
            crl_backup.validate(self.root, historical)
        self.assertEqual(historical, original)


if __name__ == "__main__":
    unittest.main()
