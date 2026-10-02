"""A complete private backup includes verified Redis security state."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
import runtime


class BackupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name in ("config", "backups", "data/ca", "data/tsa"):
            (self.root / name).mkdir(parents=True, exist_ok=True)
        self.config = {"postgres_port": 15486, "redis_port": 16386,
                       "admin_password": "private-admin", "runtime_password": "private-user",
                       "redis_password": "private-redis", "kek": "private-kek"}
        (self.root / "config/settings.json").write_text(json.dumps(self.config))
        self.calls = []
        self.state = "inactive\ninactive\n"
        self.failure = None
        self.empty_redis = False
        self.redis_pid = "123"
        self.redis_dir = str(self.root / "data")

    def execute(self, arguments, **kwargs):
        arguments = [str(value) for value in arguments]
        self.calls.append((arguments, kwargs))
        name = arguments[0]
        if name == self.failure:
            raise subprocess.CalledProcessError(1, arguments)
        if name == "systemctl":
            output = "123\n" if "--property=MainPID" in arguments else self.state
            return subprocess.CompletedProcess(arguments, 0, stdout=output)
        if name == "pg_dump":
            Path(arguments[arguments.index("--file") + 1]).write_bytes(b"database-fixture")
        if name == "redis-cli":
            if "INFO" in arguments:
                return subprocess.CompletedProcess(arguments, 0, stdout=f"# Server\r\nprocess_id:{self.redis_pid}\r\n")
            if "CONFIG" in arguments:
                return subprocess.CompletedProcess(arguments, 0, stdout=f"dir\n{self.redis_dir}\n")
            Path(arguments[arguments.index("--rdb") + 1]).write_bytes(
                b"" if self.empty_redis else b"REDIS-fixture")
        return subprocess.CompletedProcess(arguments, 0, stdout=b"")

    def backup(self):
        with patch.object(runtime, "run", side_effect=self.execute):
            return runtime.Runtime(self.root).backup()

    def test_complete_backup_contains_private_checked_redis_and_database(self):
        original = os.umask(0o022)
        try:
            reported_backup = self.backup()
        finally:
            os.umask(original)
        backup, = (self.root / "backups").iterdir()
        self.assertEqual(reported_backup, backup)
        self.assertEqual((backup / "redis.rdb").read_bytes(), b"REDIS-fixture")
        self.assertTrue((backup / "COMPLETE").is_file())
        for path in backup.iterdir():
            self.assertEqual(path.stat().st_mode & 0o777, 0o600, path.name)
        self.assertEqual(backup.stat().st_mode & 0o777, 0o700)
        commands = [call[0][0] for call in self.calls]
        self.assertIn("redis-check-rdb", commands)
        redis = next(call for call in self.calls if call[0][0] == "redis-cli")
        self.assertEqual(redis[1]["env"]["REDISCLI_AUTH"], "private-redis")
        self.assertIn("127.0.0.1", redis[0])
        self.assertIn("16386", redis[0])
        self.assertTrue(redis[1]["capture_output"])
        for arguments, _ in self.calls:
            for value in ("private-admin", "private-user", "private-redis", "private-kek"):
                self.assertNotIn(value, " ".join(arguments))

    def test_live_or_ambiguous_services_reject_backup_before_capture(self):
        for state in ("active\ninactive\n", "inactive\nactivating\n", "inactive\n", ""):
            with self.subTest(state=state):
                self.state = state
                with self.assertRaisesRegex(RuntimeError, "stopped"):
                    self.backup()
                self.assertEqual(list((self.root / "backups").iterdir()), [])

    def test_wrong_redis_process_or_directory_rejects_before_capture(self):
        for pid, directory in (("456", str(self.root / "data")),
                               ("0", str(self.root / "data")), ("123", "/other/data")):
            with self.subTest(pid=pid, directory=directory):
                self.redis_pid, self.redis_dir = pid, directory
                with self.assertRaisesRegex(RuntimeError, "Redis identity"):
                    self.backup()
                self.assertEqual(list((self.root / "backups").iterdir()), [])

    def test_failed_final_directory_flush_removes_completion(self):
        def durable_or_fail(descriptor):
            if list((self.root / "backups").glob("*/COMPLETE")):
                raise OSError("directory flush unavailable")
        with patch.object(runtime.os, "fsync", side_effect=durable_or_fail):
            with self.assertRaises(OSError):
                self.backup()
        self.assertEqual(list((self.root / "backups").glob("*/COMPLETE")), [])

    def test_dependency_failures_never_mark_a_partial_backup_complete(self):
        for command in ("pg_dump", "redis-cli", "redis-check-rdb"):
            with self.subTest(command=command):
                self.failure = command
                with self.assertRaises(subprocess.CalledProcessError):
                    self.backup()
                self.assertEqual(list((self.root / "backups").glob("*/COMPLETE")), [])

    def test_empty_redis_snapshot_is_rejected_without_completion(self):
        self.empty_redis = True
        with self.assertRaisesRegex(RuntimeError, "empty"):
            self.backup()
        self.assertEqual(list((self.root / "backups").glob("*/COMPLETE")), [])

    def test_failed_durable_flush_is_not_a_complete_backup(self):
        with patch.object(runtime.os, "fsync", side_effect=OSError("disk unavailable")):
            with self.assertRaises(OSError):
                self.backup()
        self.assertEqual(list((self.root / "backups").glob("*/COMPLETE")), [])

    def test_private_archive_failure_is_not_a_complete_backup(self):
        with patch.object(runtime.tarfile, "open", side_effect=OSError("archive unavailable")):
            with self.assertRaises(OSError):
                self.backup()
        self.assertEqual(list((self.root / "backups").glob("*/COMPLETE")), [])


if __name__ == "__main__":
    unittest.main()
