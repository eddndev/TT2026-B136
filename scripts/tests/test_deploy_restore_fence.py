"""Durable restore admission blocks normal entry points without doing a restore."""
import importlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
import crl_journal as journal
import host
import provision
import release
import renew_crl
import runtime
from deploy_restore_fence_support import (
    OPERATION, OTHER_OPERATION, denied_before, put_marker, setup,
)


class RestoreFenceTests(unittest.TestCase):
    def setUp(self):
        setup(self)

    def test_publication_is_private_durable_serialized_and_exact_reentry_is_read_only(self):
        fence = importlib.import_module("restore_fence")
        self.assertEqual(fence.fence_path(self.root), self.marker)
        fence.guard(self.root)
        for identifier in ("not-a-uuid", "00000000-0000-0000-0000-000000000000",
                           OPERATION.replace("-", "")):
            with self.subTest(identifier=identifier), self.assertRaises(ValueError):
                fence.enter(self.root, identifier)
        self.assertFalse(self.marker.parent.exists())
        with journal.locked(self.root), self.assertRaises(BlockingIOError):
            fence.enter(self.root, OPERATION)
        self.assertFalse(self.marker.exists())
        events = []
        real_fsync, real_replace, real_write = os.fsync, os.replace, journal.write

        def flush(descriptor):
            info = os.fstat(descriptor)
            events.append(("flush", info.st_dev, info.st_ino))
            return real_fsync(descriptor)

        def replace(source, destination):
            info = Path(source).stat()
            events.append(("replace", info.st_dev, info.st_ino))
            return real_replace(source, destination)

        def locked_write(path, value):
            with self.assertRaises(BlockingIOError), journal.locked(self.root):
                pass
            return real_write(path, value)

        with patch.object(os, "fsync", side_effect=flush):
            with patch.object(os, "replace", side_effect=replace):
                with patch.object(journal, "write", side_effect=locked_write):
                    result = fence.enter(self.root, OPERATION)
        self.assertEqual(result, {"operation_id": OPERATION})
        self.assertEqual(json.loads(self.marker.read_bytes()), result)
        self.assertEqual(stat.S_IMODE(self.marker.stat().st_mode), 0o600)
        for directory in (self.root / "maintenance", self.marker.parent):
            self.assertEqual(stat.S_IMODE(directory.stat().st_mode), 0o700)
        info, parent = self.marker.stat(), self.marker.parent.stat()
        replaced = events.index(("replace", info.st_dev, info.st_ino))
        self.assertIn(("flush", info.st_dev, info.st_ino), events[:replaced])
        self.assertIn(("flush", parent.st_dev, parent.st_ino), events[replaced + 1:])
        before = self.marker.read_bytes(), self.marker.stat().st_mtime_ns
        with patch.object(journal, "write", side_effect=AssertionError("reentry rewrites marker")):
            self.assertEqual(fence.enter(self.root, OPERATION), result)
            with self.assertRaises((ValueError, RuntimeError)):
                fence.enter(self.root, OTHER_OPERATION)
        self.assertEqual((self.marker.read_bytes(), self.marker.stat().st_mtime_ns), before)
        self.assertFalse((self.root / "config/crl-maintenance.json").exists())

    def test_corruption_redirects_and_uncertain_directory_flush_keep_admission_closed(self):
        fence = importlib.import_module("restore_fence")
        self.marker.parent.mkdir(parents=True, mode=0o700)
        (self.root / "maintenance").chmod(0o700)
        for value in (b"", b"invalid JSON", b"{}", b"x" * 4097,
                      ('{"operation_id":"%s","extra":true}' % OPERATION).encode(),
                      ('{"operation_id":"%s","operation_id":"%s"}'
                       % (OPERATION, OPERATION)).encode()):
            with self.subTest(value_length=len(value)):
                self.marker.write_bytes(value)
                self.marker.chmod(0o600)
                with self.assertRaisesRegex(RuntimeError, "restore|maintenance"):
                    fence.guard(self.root)
                with self.assertRaises((ValueError, RuntimeError)):
                    fence.enter(self.root, OPERATION)
                self.assertEqual(self.marker.read_bytes(), value)
        self.marker.unlink()
        self.marker.symlink_to(self.root / "absent")
        with self.assertRaisesRegex(RuntimeError, "restore|maintenance"):
            fence.guard(self.root)
        with self.assertRaises((ValueError, RuntimeError, OSError)):
            fence.enter(self.root, OPERATION)
        self.marker.unlink()
        self.marker.mkdir()
        with self.assertRaisesRegex(RuntimeError, "restore|maintenance"):
            fence.guard(self.root)
        self.marker.rmdir()
        self.marker.parent.rmdir()
        self.marker.parent.symlink_to(self.root / "config", target_is_directory=True)
        with self.assertRaisesRegex(RuntimeError, "restore|maintenance"):
            fence.guard(self.root)
        with self.assertRaises((ValueError, RuntimeError)):
            fence.enter(self.root, OPERATION)
        self.assertFalse((self.root / "config/active.json").exists())
        self.marker.parent.unlink()
        real_sync = journal.sync_directory

        def uncertain_sync(directory):
            if Path(directory) == self.marker.parent and self.marker.exists():
                raise OSError("fixture directory flush interrupted after rename")
            real_sync(directory)

        with patch.object(journal, "sync_directory", side_effect=uncertain_sync):
            with self.assertRaises(OSError):
                fence.enter(self.root, OPERATION)
        self.assertTrue(self.marker.is_file())
        before = self.marker.read_bytes(), self.marker.stat().st_mtime_ns
        with self.assertRaisesRegex(RuntimeError, "restore|maintenance"):
            fence.guard(self.root)
        retried_directories = []
        real_flush = os.fsync

        def retried_flush(descriptor):
            info = os.fstat(descriptor)
            if stat.S_ISDIR(info.st_mode):
                retried_directories.append((info.st_dev, info.st_ino))
            return real_flush(descriptor)

        with patch.object(journal, "write", side_effect=AssertionError("retry rewrites marker")):
            with patch.object(os, "fsync", side_effect=retried_flush):
                self.assertEqual(fence.enter(self.root, OPERATION), {"operation_id": OPERATION})
        parent = self.marker.parent.stat()
        self.assertIn((parent.st_dev, parent.st_ino), retried_directories)
        self.assertEqual((self.marker.read_bytes(), self.marker.stat().st_mtime_ns), before)

    def test_runtime_and_release_reject_before_readiness_secrets_or_side_effects(self):
        target = runtime.Runtime(self.root)
        put_marker(self)
        calls = [
            (lambda: runtime.serve(self.root), [(runtime, "settings"), (runtime, "environment"),
                                             (runtime.os, "chdir"), (runtime.os, "execve")]),
            (lambda: target.start(self.next), [(runtime, "run"), (target, "wait_api")]),
            (target.wait_api, [(target, "dependencies"), (target, "api_ready")]),
            (lambda: target.initialize(self.next), [(provision, "initialize")]),
            (target.backup, [(runtime, "run"), (runtime, "environment")]),
            (lambda: release.admit(self.root, self.root / "incoming/archive.tar.gz",
                                  "a" * 64, "v2.0.0", "a" * 40),
             [(release, "digest"), (release, "extract_bundle")]),
        ]
        for index, (action, edges) in enumerate(calls):
            with self.subTest(entry=index):
                denied_before(self, action, edges)
        for operation in (release.activate, release.rollback):
            controller = Mock()
            controller.stop.side_effect = AssertionError("release changed before guard")
            args = (self.root, self.next, controller) if operation is release.activate else (
                self.root, controller)
            with self.subTest(entry=operation.__name__):
                with self.assertRaisesRegex(RuntimeError, "restore|maintenance"):
                    operation(*args)
                self.assertEqual(controller.mock_calls, [])
        self.assertEqual((self.root / "current").resolve(), self.current)
        self.assertEqual((self.root / "previous").resolve(), self.previous)

    def test_public_preparation_entries_reject_before_loading_or_modifying_state(self):
        put_marker(self)
        calls = [
            (lambda: runtime.Runtime(self.root), [(runtime, "settings")]),
            (lambda: host.prepare(self.root), [(host.os, "umask"), (host.shutil, "which"),
                                              (host.subprocess, "run")]),
            (lambda: host.start_databases(self.root), [(provision, "create_databases")]),
            (lambda: provision.create_databases(self.root), [(provision, "settings"),
                                                            (provision, "run")]),
            (lambda: provision.initialize(self.root, self.next), [(provision, "environment"),
                                                                 (provision, "run")]),
        ]
        before = (self.root / "config/schema").read_bytes()
        for index, (action, edges) in enumerate(calls):
            with self.subTest(entry=index):
                denied_before(self, action, edges)
        self.assertEqual((self.root / "config/schema").read_bytes(), before)

    def test_crl_and_restore_exclude_each_other_including_public_resume_helpers(self):
        fence = importlib.import_module("restore_fence")
        journal.fenced(self.root, OTHER_OPERATION)
        with self.assertRaisesRegex(RuntimeError, "CRL|maintenance"):
            fence.enter(self.root, OPERATION)
        self.assertFalse(self.marker.parent.exists())
        journal.unfence(self.root)
        put_marker(self)
        controller = Mock()
        controller.stop.side_effect = AssertionError("CRL stop reached before guard")
        operation = self.root / "maintenance/crl" / OTHER_OPERATION
        calls = [
            lambda: renew_crl.renew(self.root, 1, controller),
            lambda: renew_crl.resume(self.root, OTHER_OPERATION, controller),
            lambda: renew_crl.prepared(self.root, self.current, {}, 1),
            lambda: renew_crl.continue_operation(self.root, operation, {"backup": None},
                                                self.current, controller),
        ]
        for index, action in enumerate(calls):
            with self.subTest(entry=index):
                denied_before(self, action, [(renew_crl, "current_release"),
                                            (renew_crl.material, "read_head"),
                                            (journal, "write")])
                self.assertEqual(controller.mock_calls, [])
                self.assertFalse(journal.pending(self.root))

    def test_backend_boot_uses_guard_only_and_crl_fence_still_allows_backend_start(self):
        units = host.service_units(self.root, "/private/pg/bin", "/private/python")
        expected = (f"ExecStartPre=/private/python {self.root}/tools/restore_fence.py "
                    f"{self.root}")
        for name in ("qadra-postgres", "qadra-redis"):
            with self.subTest(unit=name):
                self.assertIn(expected, units[name].splitlines())
        script = Path(__file__).resolve().parents[2] / "ops/deploy/restore_fence.py"
        command = [sys.executable, "-B", str(script), str(self.root)]
        # A fresh interpreter models a new boot without cached Python state.
        allowed = subprocess.run(command, capture_output=True, timeout=5)
        self.assertEqual((allowed.returncode, allowed.stdout, allowed.stderr), (0, b"", b""))
        journal.fenced(self.root, OTHER_OPERATION)
        allowed = subprocess.run(command, capture_output=True, timeout=5)
        self.assertEqual((allowed.returncode, allowed.stdout, allowed.stderr), (0, b"", b""))
        journal.unfence(self.root)
        put_marker(self)
        (self.root / "config/settings.json").unlink()
        blocked = subprocess.run(command, capture_output=True, timeout=5)
        self.assertEqual(blocked.returncode, 1)
        self.assertEqual(blocked.stdout, b"")
        self.assertLess(len(blocked.stderr), 256)
        self.assertIn(b"restore", blocked.stderr.lower())
        self.assertTrue(self.marker.is_file())
        unsupported = subprocess.run(command + ["--clear"], capture_output=True, timeout=5)
        self.assertNotEqual(unsupported.returncode, 0)
        self.assertTrue(self.marker.is_file())


if __name__ == "__main__":
    unittest.main()
