"""Exercise Owner login observations against an actual disposable Redis server."""
import base64
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

import test_deployment_redis_snapshot as native_redis


class OwnerLoginRedisTransport(unittest.TestCase):
    def test_real_info_and_binary_observations_without_non_json_fallback(self):
        native_redis.RedisSnapshotTests.setUpClass()
        fixture = native_redis.RedisSnapshotTests(
            "test_snapshot_preserves_values_expiry_and_cleaned_aof_survives_restart")
        self.addCleanup(fixture.doCleanups)
        fixture.setUp()
        scripts = Path(__file__).resolve().parents[1]
        environment = {
            "REDISCLI_AUTH": fixture.password,
            "TT_OWNER_CERT_WORK": str(fixture.root),
            "TT_OWNER_CERT_TOKEN": "unused-transport-fixture",
            "TT_OWNER_LOGIN_WORK": str(fixture.root / "data"),
            "TT_OWNER_LOGIN_REDIS_PORT": str(fixture.port),
            "TT_OWNER_LOGIN_REDIS_PID": str(fixture.process.pid),
            "TT_OWNER_LOGIN_SERVER_PID": "",
        }
        with patch.dict(os.environ, environment), patch.dict(sys.modules), \
                patch.object(sys, "path", [str(scripts), *sys.path]):
            sys.modules.pop("api_owner_certificate_evidence", None)
            spec = importlib.util.spec_from_file_location(
                "owner_login_transport_probe", scripts / "api_owner_login_support.py")
            helper = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(helper)
            # RESP3 INFO is a verbatim reply, unlike the ordinary JSON responses.
            helper.redis_guard(stopped=True)
            now = helper.now_ms()
            self.assertGreater(now, 0)
            expiry = now + 300_000
            scalar, hashed = "owner-login-transport:scalar", "owner-login-transport:hash"
            self.assertEqual(fixture.cli("SET", scalar, "binary scalar", "PXAT", expiry), "OK")
            self.assertEqual(fixture.cli("HSET", hashed, "z", "last", "a", "first"), 2)
            self.assertEqual(fixture.cli("PEXPIREAT", hashed, expiry), 1)
            binary = "redis.call('HSET', KEYS[1], string.char(0,255), string.char(10,0,254)); return 1"
            self.assertEqual(fixture.cli("EVAL", binary, 1, hashed), 1)
            scalar_row = helper.read(scalar)
            self.assertEqual(scalar_row["type"], "string")
            self.assertEqual(scalar_row["expires_at_unix_ms"], expiry)
            dumped = subprocess.run([
                "redis-cli", "-h", "127.0.0.1", "-p", str(fixture.port),
                "--raw", "DUMP", scalar], capture_output=True, timeout=10, check=True).stdout
            self.assertTrue(dumped.endswith(b"\n"))
            self.assertEqual(base64.b64decode(scalar_row["dump_base64"], validate=True), dumped[:-1])
            hash_row = helper.read(hashed)
            self.assertEqual(hash_row["type"], "hash")
            self.assertEqual(hash_row["expires_at_unix_ms"], expiry)
            self.assertEqual({base64.b64decode(k): base64.b64decode(v)
                              for k, v in hash_row["fields"]},
                             {b"z": b"last", b"a": b"first", b"\x00\xff": b"\n\x00\xfe"})
            self.assertEqual(helper.read("owner-login-transport:absent"),
                             {"type": "none", "expires_at_unix_ms": -2})
            malformed = subprocess.CompletedProcess([], 0, stdout=b"not JSON", stderr=b"")
            with patch.object(helper.subprocess, "run", return_value=malformed):
                with self.assertRaisesRegex(AssertionError, "Invalid disposable Redis response"):
                    helper.redis("TIME")


if __name__ == "__main__":
    unittest.main()
