"""Command-boundary fixture for narrowly scoped restore-session invalidation."""
import copy
import importlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "ops/deploy"))


class RedisCommands:
    def __init__(self, owner, directory):
        self.owner = owner
        self.directory = directory
        self.pid = os.getpid()
        self.password = "private-redis-fixture-sentinel"
        self.values = {
            "identity:session:" + "1" * 64: ("private-session", 2200000000000),
            "identity:session:" + "2" * 64: ("private-session", 2200000000001),
            "identity:challenge:" + "3" * 64: ("private-challenge", 2200000000002),
            "identity:password-failures:" + "4" * 64: ("3", 2200000000003),
            "identity:totp-used:" + "5" * 64: ("1", 2200000000004),
            "identity:password-reset:v1:request:global": ("private-limit", 2200000000005),
            "identity:password-reset:v1:completion:global": ("private-limit", 2200000000006),
            "unrelated:preserved": ("private-other", 2200000000007),
            "identity:password-reset:v1:request:email:" + "7" * 64: ("private-limit", 2200000000008),
            "identity:password-reset:v1:completion:digest:" + "8" * 64: ("private-limit", 2200000000009),
        }
        self.calls = []
        self.scan_calls = 0
        self.delete_calls = 0
        self.bad_page = None
        self.endless = False
        self.fail_delete = False
        self.appear_after_delete = False
        self.extra = "identity:session:" + "6" * 64
        self.info_override = {}
        self.config_directory = str(directory)

    def command(self, command):
        self.calls.append(tuple(command))
        name = command[0].upper()
        if name == "PING":
            return "PONG"
        if command == ["INFO", "server"]:
            fields = {"process_id": str(self.pid), "redis_version": "7.2.4",
                      "valkey_version": "8.1.10", "server_name": "valkey"}
            fields.update(self.info_override)
            return "# Server\r\n" + "\r\n".join(f"{key}:{value}" for key, value in fields.items()) + "\r\n"
        if command == ["INFO", "persistence"]:
            return ("aof_enabled:1\r\naof_rewrite_in_progress:0\r\n"
                    "aof_rewrite_scheduled:0\r\naof_last_bgrewrite_status:ok\r\n")
        if name == "CONFIG" and command[1] == "GET":
            available = {"dir": self.config_directory, "dbfilename": "redis.rdb",
                         "appendonly": "yes", "appenddirname": "appendonlydir"}
            return [item for key in command[2:] for item in (key, available[key])]
        if name == "SCAN":
            self.scan_calls += 1
            if self.scan_calls > 40:
                raise AssertionError("Redis scan escaped its bounded traversal")
            pattern = command[command.index("MATCH") + 1]
            if self.bad_page is not None and pattern == "identity:challenge:*":
                return copy.deepcopy(self.bad_page)
            if self.endless:
                return ["1", []]
            keys = sorted(key for key in self.values if key.startswith(pattern[:-1]))
            # Duplicate observations are legal while SCAN walks a changing table.
            if command[1] == "0" and len(keys) > 1:
                return ["7", keys[:1]]
            return ["0", keys]
        if name == "DEL":
            self.delete_calls += 1
            if self.fail_delete:
                self.fail_delete = False
                raise subprocess.TimeoutExpired(["redis-cli"], 1, stderr=b"private-transport-sentinel")
            deleted = 0
            for key in command[1:]:
                if key in self.values:
                    del self.values[key]
                    deleted += 1
            if self.appear_after_delete:
                self.appear_after_delete = False
                self.values[self.extra] = ("new-private-session", 2200000000099)
            return deleted
        raise AssertionError("unexpected Redis effect or lookup")

    def execute(self, arguments, **kwargs):
        args = [str(value) for value in arguments]
        self.owner.assertNotIn(self.password, " ".join(args), "secret entered command arguments")
        self.owner.assertTrue(0 < kwargs.get("timeout", 0) <= 10, "unbounded Redis command")
        self.owner.assertTrue(kwargs.get("env", {}).get("REDISCLI_AUTH") == self.password,
                              "Redis authentication was not supplied privately")
        self.owner.assertEqual(args[args.index("-h") + 1], "127.0.0.1")
        self.owner.assertEqual(args[args.index("-p") + 1], "16386")
        if "-n" in args:
            self.owner.assertEqual(args[args.index("-n") + 1], "0")
        names = {"PING", "INFO", "CONFIG", "SCAN", "DEL"}
        index = next((index for index, value in enumerate(args) if value in names), None)
        if index is None:
            raise AssertionError("unexpected Redis command shape")
        value = self.command(args[index:])
        if "--json" in args and "--raw" not in args and args[index] != "INFO":
            output = json.dumps(value) + "\n"
        elif isinstance(value, list):
            output = "\n".join(str(item) for item in value) + "\n"
        else:
            output = str(value) + "\n"
        if not kwargs.get("text", False):
            output = output.encode("utf-8")
        return subprocess.CompletedProcess(args, 0, stdout=output, stderr="")


class RedisInvalidationFixture(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.root.chmod(0o700)
        self.redis = RedisCommands(self, self.root)
        client = self.root / "redis-cli"
        client.write_bytes(b"command-boundary Redis client fixture")
        client.chmod(0o700)
        self.options = {
            "redis_cli": client, "host": "127.0.0.1", "port": 16386,
            "password": self.redis.password, "expected_pid": self.redis.pid,
            "expected_directory": self.root, "max_scan_calls": 12, "max_keys": 20,
            "batch_size": 1, "timeout": 2,
        }

    def invoke(self, **changes):
        module = importlib.import_module("restore_redis")
        with patch.object(module, "run", side_effect=self.redis.execute):
            return module.invalidate_sessions(
                **{**self.options, **changes})

    def rejection(self, **changes):
        try:
            self.invoke(**changes)
        except (ValueError, RuntimeError, OSError) as error:
            self.assertNotIn("private-", str(error), "private diagnostic escaped")
            return
        self.fail("invalid restore-session operation was accepted")
