"""Restore opaque certificate-login captures and retain real Redis controls."""
from pathlib import Path
import shutil
import time
import unittest
from unittest.mock import patch

import test_deployment_redis_snapshot as snapshots
from deploy_restore_certificate_login_support import (
    CAPTURE_PREFIX, CONTROL_KEYS, budget, is_authentication, opaque_capture,
)
import restore_redis
import runtime


class CertificateLoginSnapshotTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        snapshots.RedisSnapshotTests.setUpClass()

    def setUp(self):
        # Composition reuses process ownership without registering its test again.
        self.native = snapshots.RedisSnapshotTests(
            "test_snapshot_preserves_values_expiry_and_cleaned_aof_survives_restart")
        self.addCleanup(self.native.doCleanups)
        self.native.setUp()
        client = shutil.which("redis-cli")
        self.assertIsNotNone(client, "native Redis client is required")
        self.redis_cli = Path(client).resolve(strict=True)

    def snapshot(self):
        result, cursor = {}, "0"
        for _ in range(100):
            cursor, keys = self.native.cli("SCAN", cursor, "COUNT", 100)
            for key in keys:
                kind = self.native.cli("TYPE", key)
                if kind == "hash":
                    fields = self.native.cli("-2", "HGETALL", key)
                    self.assertIsInstance(fields, list)
                    self.assertTrue(all(isinstance(field, str) for field in fields),
                                    "native hash fields must be strings")
                    self.assertEqual(len(fields) % 2, 0)
                    pairs = list(zip(fields[::2], fields[1::2]))
                    self.assertEqual(len(pairs), len(dict(pairs)))
                    value = sorted(pairs)
                elif kind == "string":
                    value = self.native.cli("GET", key)
                else:
                    self.fail("unexpected owned snapshot value type")
                result[key] = (kind, value, self.native.cli("PEXPIRETIME", key))
            if str(cursor) == "0":
                return result
        self.fail("owned snapshot scan exceeded its bound")

    def invalidate(self):
        return restore_redis.invalidate_sessions(
            redis_cli=self.redis_cli, host="127.0.0.1", port=self.native.port,
            password=self.native.password, expected_pid=self.native.process.pid,
            expected_directory=self.native.root / "data", max_scan_calls=100,
            max_keys=1000, batch_size=2, timeout=5)

    def test_real_rdb_and_cleaned_aof_cannot_revive_captures_or_reset_proof_budgets(self):
        db = self.native
        seconds, micros = db.cli("TIME")
        expiry = int(seconds) * 1000 + int(micros) // 1000 + 300000
        strings = {
            "identity:session:" + "1" * 64: "synthetic-session",
            "identity:challenge:" + "2" * 64: "synthetic-MFA",
            CAPTURE_PREFIX + "3" * 64: opaque_capture(),
            "identity:password-failures:" + "5" * 64: "3",
            "identity:totp-used:" + "6" * 64: "1",
            "unrelated:preserved": "synthetic-unrelated",
            "identity:certificate-login-other:" + "7" * 64: "lookalike-preserved",
        }
        for key, value in strings.items():
            self.assertEqual(db.cli("SET", key, value, "PXAT", expiry), "OK")
        for key in (*CONTROL_KEYS, "identity:password-reset:v1:completion:global"):
            fields = budget(expiry)
            args = [item for pair in fields.items() for item in pair]
            self.assertEqual(db.cli("HSET", key, *args), len(fields))
            self.assertEqual(db.cli("PEXPIREAT", key, expiry), 1)
        corrupt = CAPTURE_PREFIX + "4" * 64
        self.assertEqual(db.cli("HSET", corrupt, "corrupt", "capture"), 1)
        self.assertEqual(db.cli("PEXPIREAT", corrupt, expiry), 1)
        original = self.snapshot()
        with patch.object(runtime, "run", side_effect=db.capture):
            backup = runtime.Runtime(db.root).backup()
        self.assertTrue((backup / "COMPLETE").is_file())
        self.assertGreater((backup / "redis.rdb").stat().st_size, 0)
        db.stop()
        shutil.copyfile(backup / "redis.rdb", db.root / "data/redis.rdb")
        db.start(False)
        self.assertTrue(self.snapshot() == original, "RDB changed values, types or exact expiry")
        self.assertEqual(self.invalidate(), {"sessions_removed": 1, "challenges_removed": 1,
                                             "certificate_logins_removed": 2})
        retained = {key: value for key, value in original.items() if not is_authentication(key)}
        self.assertTrue(self.snapshot() == retained, "invalidation changed controls or unrelated state")
        self.assertEqual(self.invalidate(), {"sessions_removed": 0, "challenges_removed": 0,
                                             "certificate_logins_removed": 0})
        self.assertEqual(db.cli("CONFIG", "SET", "appendonly", "yes"), "OK")
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            facts = db.server_info("persistence")
            if (facts["aof_enabled"] == "1" and facts["aof_rewrite_in_progress"] == "0"
                    and facts["aof_rewrite_scheduled"] == "0" and facts["aof_last_bgrewrite_status"] == "ok"):
                break
            time.sleep(0.05)
        else:
            self.fail("sanitized AOF did not become ready")
        db.stop()
        shutil.copyfile(backup / "redis.rdb", db.root / "data/redis.rdb")
        db.start(True)
        self.assertTrue(self.snapshot() == retained, "old RDB revived capture or changed preserved budgets")
        self.assertTrue(all(db.cli("EXISTS", key) == 1 for key in CONTROL_KEYS))


if __name__ == "__main__":
    unittest.main()
