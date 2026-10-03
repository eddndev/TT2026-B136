"""Reuse deployment inventory with explicit PostgreSQL/Redis command doubles."""
from contextlib import ExitStack
import copy
import importlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
from unittest.mock import patch

from deploy_restore_admission_support import AdmissionFixture, fingerprint, role
from deploy_restore_redis_support import RedisCommands
import crl_journal
import runtime


class CaptureFixture(AdmissionFixture):
    def setUp(self):
        super().setUp()
        shutil.rmtree(self.backup)
        self.descriptor_path.unlink()
        self.marker.unlink()
        self.descriptor_path = self.root / "receipts/captured-compatibility.json"
        for directory in self.root.rglob("*"):
            if directory.is_dir() and not directory.is_symlink():
                directory.chmod(0o700)
        self.pg_directory = self.root / "data/postgres"
        self.pg_directory.mkdir(mode=0o700)
        (self.pg_directory / "postmaster.pid").write_text(
            f"{os.getpid()}\n{self.pg_directory}\n1735689600\n15486\n/private/socket\n127.0.0.1\n0 0\nready\n")
        (self.pg_directory / "postmaster.pid").chmod(0o600)
        (self.root / "data/tmp").mkdir(mode=0o700)
        config = self.root / "config/settings.json"
        self.settings = json.loads(config.read_bytes())
        self.settings.update(postgres_port=15486, redis_port=16386,
                             admin_password="private-admin-fixture-sentinel",
                             redis_password="private-redis-fixture-sentinel")
        self.fixture.write_json(config, self.settings)
        self.redis = RedisCommands(self, self.root / "data")
        executable = Path(sys.executable).resolve(strict=True)
        self.tools = {"postgres": executable, "redis_server": executable}
        binary_directory = self.root / "receipts/tools"
        binary_directory.mkdir(mode=0o700)
        for name in ("psql", "pg_dump", "pg_restore", "redis_cli", "redis_check_rdb"):
            path = binary_directory / name.replace("_", "-")
            path.write_bytes(("command boundary fixture " + name).encode("ascii"))
            path.chmod(0o700)
            self.tools[name] = path
        self.targets = {
            "postgres": {"pid": os.getpid(), "data_directory": self.pg_directory,
                         "database": "qadra", "schema": "public", "owner": "qadra_admin",
                         "runtime_role": "qadra_runtime"},
            "redis": {"pid": os.getpid(), "directory": self.root / "data", "database": 0},
        }
        profiles = {"qadra_admin": role(True), "qadra_runtime": role(False)}
        self.catalog = {
            "database": "qadra", "schema": "public", "database_owner": "qadra_admin",
            "schema_owner": "qadra_admin", "server_version_num": 160015,
            "data_directory": str(self.pg_directory), "encoding": "UTF8", "collate": "C",
            "ctype": "C", "locale_provider": "c", "roles": profiles,
            "role_settings": [], "role_expirations": [], "database_settings": [],
            "memberships": [], "other_clients": [],
        }
        self.audit = [{"sequence": 0, "timestamp": "2025-01-01T00:00:00Z", "actor": "fixture-owner",
                       "action": "fixture.capture", "resource": "fixture",
                       "chain": "a" * 64}]
        self.audit_valid = True
        self.audit_count_delta = 0
        self.after_dump = None
        self.catalog_prefix = ""
        self.backups_started = 0
        self.verified_exports = []
        self.commands = []

    def completed(self, arguments, output, kwargs):
        if not kwargs.get("text", False) and isinstance(output, str):
            output = output.encode("utf-8")
        return subprocess.CompletedProcess(arguments, 0, stdout=output, stderr="")

    def execute(self, arguments, **kwargs):
        args = [str(value) for value in arguments]
        self.commands.append(tuple(args))
        for secret in (self.settings["admin_password"], self.redis.password):
            self.assertNotIn(secret, " ".join(args), "secret entered command arguments")
        self.assertTrue(0 < kwargs.get("timeout", 0) <= 300, "external command is unbounded")
        name = Path(args[0]).name
        if name == "systemctl":
            self.assertEqual(args[1:3], ["--user", "show"], "capture attempted a service mutation")
            output = str(os.getpid()) if "--property=MainPID" in args else "inactive\ninactive\n"
            return self.completed(args, output, kwargs)
        if "--version" in args:
            versions = {"pg-dump": "pg_dump (PostgreSQL) 16.15\n",
                        "pg-restore": "pg_restore (PostgreSQL) 16.15\n",
                        "redis-cli": "valkey-cli 8.1.10\n"}
            if name not in versions:
                raise AssertionError("unsupported executable version probe")
            return self.completed(args, versions[name], kwargs)
        if name == "psql":
            self.assertEqual(Path(args[0]), self.tools["psql"], "unbound catalog client")
            return self.sql(args, kwargs)
        if name in ("pg_dump", "pg-dump"):
            self.assert_locked()
            self.assertEqual(Path(args[0]), self.tools["pg_dump"], "unbound SQL dump executable")
            self.assertTrue(kwargs.get("env", {}).get("PGPASSWORD") == self.settings["admin_password"],
                            "SQL capture authentication was not private")
            Path(args[args.index("--file") + 1]).write_bytes(b"PGDMP-native-boundary-fixture")
            self.backups_started += 1
            if self.after_dump is not None:
                self.after_dump()
            return self.completed(args, "", kwargs)
        if name == "redis-cli":
            self.assertEqual(Path(args[0]), self.tools["redis_cli"], "unbound Redis client")
            if "--rdb" in args:
                Path(args[args.index("--rdb") + 1]).write_bytes(b"REDIS0012-native-boundary-fixture")
                return self.completed(args, "", kwargs)
            return self.redis.execute(args, **kwargs)
        if name == "redis-check-rdb":
            self.assertEqual(Path(args[0]), self.tools["redis_check_rdb"], "unbound RDB checker")
            return self.completed(args, "RDB is valid\n", kwargs)
        if Path(args[0]) == self.target / "bin/despacho-cli":
            self.assertEqual(args[1:], ["--json", "audit", "verify-chain"])
            path = Path(kwargs.get("env", {}).get("AUDIT_LOG_PATH", ""))
            self.assertTrue(path.parent == self.root / "data/tmp", "audit export escaped private scratch")
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
            rows = [json.loads(line) for line in path.read_text().splitlines()]
            self.assertTrue(rows == self.audit, "verifier did not receive the complete ordered audit")
            self.verified_exports.append(path)
            return self.completed(args, json.dumps({"valid": self.audit_valid,
                                  "entries": len(rows) + self.audit_count_delta}) + "\n", kwargs)
        raise AssertionError("unexpected capture command")

    def sql(self, args, kwargs):
        self.assert_locked()
        query = args[args.index("-c") + 1] if "-c" in args else kwargs.get("input", "")
        if isinstance(query, bytes):
            query = query.decode("ascii")
        self.assertFalse(re.search(r"\b(INSERT|UPDATE|DELETE|CREATE|DROP|ALTER|GRANT)\b",
                                   query, re.I), "collector attempted SQL mutation")
        self.assertIn("READ ONLY", query.upper(), "collector omitted read-only transaction scope")
        self.assertIn("pg_catalog", query, "collector omitted safe catalog resolution")
        self.assertTrue(kwargs.get("env", {}).get("PGPASSWORD") == self.settings["admin_password"],
                        "catalog authentication was not private")
        if "pg_database" in query:
            output = self.catalog_prefix + json.dumps(self.catalog) + "\n"
        elif "audit_events" in query:
            output = "".join(json.dumps(row) + "\n" for row in self.audit)
        else:
            raise AssertionError("unexpected SQL observation")
        return self.completed(args, output, kwargs)

    def assert_locked(self):
        try:
            with crl_journal.locked(self.root):
                self.fail("capture did not exclude deployment maintenance")
        except BlockingIOError:
            pass

    def invoke_capture(self):
        module = importlib.import_module("restore_capture")
        with ExitStack() as stack:
            for name in ("restore_capture", "restore_catalog", "restore_observations"):
                stack.enter_context(patch.object(importlib.import_module(name), "run", side_effect=self.execute))
            stack.enter_context(patch.object(subprocess, "run",
                                            side_effect=AssertionError("unexpected unbounded command")))
            stack.enter_context(patch.object(subprocess, "Popen",
                                            side_effect=AssertionError("unexpected process launch")))
            return module.capture(
                self.root, controller_directory=self.controller, targets=copy.deepcopy(self.targets),
                tools=dict(self.tools), descriptor_path=self.descriptor_path)

    def rejected_capture(self):
        try:
            self.invoke_capture()
        except (ValueError, RuntimeError, OSError) as error:
            self.assertNotIn("private-", str(error), "private capture diagnostic escaped")
            self.assertTrue(all(not path.exists() for path in self.verified_exports),
                            "private temporary audit export survived failure")
            return
        self.fail("invalid native capture was accepted")

    def expected_facts(self):
        value = copy.deepcopy(self.observed)
        value["redis"].update(engine="valkey", server_version="8.1.10", cli_version="8.1.10")
        for group, mapping in (
                ("postgres", {"server": "postgres", "pg_dump": "pg_dump", "pg_restore": "pg_restore"}),
                ("redis", {"server": "redis_server", "redis_cli": "redis_cli", "redis_check_rdb": "redis_check_rdb"})):
            value[group]["tools"] = {name: fingerprint(self.tools[tool].read_bytes())
                                     for name, tool in mapping.items()}
        return value
