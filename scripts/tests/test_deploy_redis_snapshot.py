"""Exercise backup bytes and expiry with disposable native Redis processes."""
import json
import os
from pathlib import Path
import secrets
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))
import runtime


class RedisSnapshotTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for directory in ("data", "config", "backups"):
            (self.root / directory).mkdir(mode=0o700)
        self.password = secrets.token_hex(24)
        self.process = None
        self.log = None
        self.addCleanup(self.stop)
        self.start(False)
        (self.root / "config/settings.json").write_text(json.dumps({
            "postgres_port": 1, "redis_port": self.port,
            "admin_password": "synthetic", "runtime_password": "synthetic",
            "redis_password": self.password, "kek": "synthetic"}))

    def cli(self, *args):
        output = "--raw" if args[0] == "INFO" else "--json"
        result = subprocess.run(["redis-cli", "-h", "127.0.0.1", "-p", str(self.port),
                                 output, *map(str, args)],
                                env={**os.environ, "REDISCLI_AUTH": self.password},
                                capture_output=True, text=True, timeout=10, check=True)
        return result.stdout if output == "--raw" else json.loads(result.stdout)

    def start(self, appendonly):
        with socket.socket() as socket_:
            socket_.bind(("127.0.0.1", 0))
            self.port = socket_.getsockname()[1]
        config = self.root / "config/server.conf"
        config.write_text(f"bind 127.0.0.1\nport {self.port}\nrequirepass {self.password}\n"
                          f"dir {self.root}/data\ndbfilename redis.rdb\nsave \"\"\n"
                          f"appendonly {'yes' if appendonly else 'no'}\n"
                          "maxmemory 64mb\ndaemonize no\n")
        config.chmod(0o600)
        self.log = (self.root / "server.log").open("ab")
        self.process = subprocess.Popen(["redis-server", str(config)],
                                        stdout=self.log, stderr=self.log)
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            self.assertIsNone(self.process.poll(), "disposable Redis exited before readiness")
            try:
                if self.cli("PING") == "PONG":
                    self.assertEqual(self.server_info()["process_id"], str(self.process.pid))
                    return
            except (subprocess.SubprocessError, json.JSONDecodeError):
                pass
            time.sleep(0.05)
        self.fail("disposable Redis did not become ready")

    def server_info(self, section="server"):
        return dict(line.split(":", 1) for line in self.cli("INFO", section).splitlines()
                    if ":" in line)

    def stop(self):
        if self.process is not None:
            if self.process.poll() is None:
                self.process.terminate()
                try:
                    self.process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait(timeout=5)
                    self.fail("disposable Redis required forced termination")
            self.process = None
        if self.log is not None:
            self.log.close()
            self.log = None

    def capture(self, arguments, **kwargs):
        arguments = [str(value) for value in arguments]
        if arguments[0] == "systemctl":
            state = str(self.process.pid) if "--property=MainPID" in arguments else "inactive\ninactive"
            return subprocess.CompletedProcess(arguments, 0, stdout=state)
        if arguments[0] == "pg_dump":
            # PostgreSQL capture has separate acceptance; this probe isolates Redis.
            Path(arguments[arguments.index("--file") + 1]).write_bytes(b"synthetic SQL snapshot")
            return subprocess.CompletedProcess(arguments, 0)
        return subprocess.run(arguments, check=True, **kwargs)

    def test_snapshot_preserves_values_expiry_and_cleaned_aof_survives_restart(self):
        expiry = int(time.time() * 1000) + 300_000
        values = {"identity:session:" + "1" * 64: "synthetic-session",
                  "identity:challenge:" + "2" * 64: "synthetic-challenge",
                  "identity:password-failures:" + "3" * 64: "3",
                  "identity:totp-used:" + "4" * 64: "1",
                  "unrelated:preserved": "synthetic-unrelated"}
        for key, value in values.items():
            self.assertEqual(self.cli("SET", key, value, "PXAT", expiry), "OK")
        with patch.object(runtime, "run", side_effect=self.capture):
            runtime.Runtime(self.root).backup()
        backup, = (self.root / "backups").iterdir()
        self.assertTrue((backup / "COMPLETE").is_file())
        snapshot = backup / "redis.rdb"
        self.assertGreater(snapshot.stat().st_size, 0)
        self.stop()
        shutil.copyfile(snapshot, self.root / "data/redis.rdb")
        self.start(False)
        for key, value in values.items():
            self.assertEqual(self.cli("GET", key), value)
            self.assertEqual(self.cli("PEXPIRETIME", key), expiry)
        removed = []
        for pattern in ("identity:session:*", "identity:challenge:*"):
            cursor = "0"
            while True:
                cursor, keys = self.cli("SCAN", cursor, "MATCH", pattern, "COUNT", 100)
                if keys:
                    self.assertEqual(self.cli("DEL", *keys), len(keys))
                    removed.extend(keys)
                if cursor == "0":
                    break
        self.assertEqual(len(removed), 2)
        self.assertEqual(self.cli("CONFIG", "SET", "appendonly", "yes"), "OK")
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            info = self.server_info("persistence")
            if (info["aof_enabled"] == "1" and info["aof_rewrite_in_progress"] == "0"
                    and info["aof_rewrite_scheduled"] == "0"
                    and info["aof_last_bgrewrite_status"] == "ok"):
                break
            time.sleep(0.05)
        else:
            self.fail("cleaned AOF did not become ready")
        self.stop()
        # An old RDB must not replace the cleaned AOF on the next service start.
        shutil.copyfile(snapshot, self.root / "data/redis.rdb")
        self.start(True)
        for key, value in values.items():
            if key in removed:
                self.assertIsNone(self.cli("GET", key))
            else:
                self.assertEqual(self.cli("GET", key), value)
                self.assertEqual(self.cli("PEXPIRETIME", key), expiry)
        self.assertEqual(self.cli("DBSIZE"), 3)


if __name__ == "__main__":
    unittest.main()
