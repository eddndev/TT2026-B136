"""Validate the isolated HTTP durable-commit evidence parser and reconciliation."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import stat
import tempfile
import unittest
from unittest.mock import patch


SCRIPTS = Path(__file__).resolve().parents[1]
REQUEST = "00000000-0000-4000-8000-000000000001"
USER = "00000000-0000-4000-8000-000000000002"
OTHER = "00000000-0000-4000-8000-000000000003"
OPERATION = "GET /api/v1/auth/me"
CATALOG = {OPERATION, "POST /api/v1/auth/login"}
ENVIRONMENT = {"os": "fixture", "cpu": "fixture", "memory_bytes": 1024,
               "postgres_disk": "fixture", "build_profile": "debug",
               "temporary_filesystem": "disk"}
DATABASE = {"postgres_version": "16.15", "fsync": "on", "full_page_writes": "on",
            "synchronous_commit": "on", "audit_events_count": 12,
            "events_before_first_access": 1}


def log_line(request=REQUEST, operation=OPERATION, durable="true", elapsed="499999"):
    return ("2026-10-11T12:00:00.000000Z  INFO qadra::audit_commit: "
            "access audit append finished durable=" + durable + " request_id=Some(" + request
            + ') operation="' + operation + '" elapsed_micros=' + elapsed)


def event():
    return {"sequence": "12", "action": "access.granted", "operation": OPERATION,
            "context": {"source": "http", "user_id": USER,
                        "ip_address": "127.0.0.1", "request_id": REQUEST}}


class AccessAuditReceipt(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location(
            "api_access_audit_test", SCRIPTS / "api-access-audit-demo.py")
        self.helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.helper)

    def receipt(self, logs=None, rows=None, catalog=None, database=None):
        return self.helper.build_receipt(
            self.helper.parse_log(log_line()) if logs is None else logs,
            [event()] if rows is None else rows,
            CATALOG if catalog is None else catalog, "local-working-copy",
            ENVIRONMENT, DATABASE if database is None else database)

    def test_only_exact_target_lines_supply_samples_and_ansi_is_removed(self):
        unrelated = ["listening on http://127.0.0.1:9999",
                     log_line().replace("qadra::audit_commit:", "qadra::audit_commit_extra:"),
                     "2026-10-11T12:00:00Z INFO other: qadra::audit_commit: password=private"]
        colored = log_line().replace("INFO", "\x1b[32mINFO\x1b[0m")
        rows = self.helper.parse_log("\n".join([*unrelated, colored]))
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0], {"request_id": REQUEST, "operation": OPERATION,
                                  "durable": True, "elapsed_micros": 499999,
                                  "log_line": 4})

    def test_target_with_missing_or_invalid_fields_fails_without_echoing_input(self):
        malformed = [log_line(request="private-secret"), log_line(elapsed="-1"),
                     log_line(durable="yes"), log_line().replace("Some(" + REQUEST + ")", "None"),
                     log_line().replace(" elapsed_micros=499999", ""),
                     log_line() + " elapsed_micros=1"]
        for line in malformed:
            with self.subTest(line=line), self.assertRaises(ValueError) as failure:
                self.helper.parse_log(line)
            self.assertNotIn("private-secret", str(failure.exception))

    def test_reconciles_one_stored_context_and_reports_missing_catalog_operations(self):
        receipt = self.receipt()
        self.assertTrue(receipt["accepted"])
        self.assertEqual(receipt["sample_count"], 1)
        self.assertEqual(receipt["distinct_operations_seen"], [OPERATION])
        self.assertEqual(receipt["operations_missing"], ["POST /api/v1/auth/login"])
        self.assertFalse(receipt["universal_route_coverage"])
        sample = receipt["samples"][0]
        self.assertEqual(sample["context"], event()["context"])
        self.assertEqual(sample["sequence"], "12")
        self.assertEqual(sample["action"], "access.granted")
        self.assertEqual(receipt["database"]["audit_events_count"], 12)

    def test_each_sample_must_be_durable_and_strictly_below_limit(self):
        for durable, elapsed in [("true", "500000"), ("true", "600000"), ("false", "10")]:
            with self.subTest(durable=durable, elapsed=elapsed):
                receipt = self.receipt(logs=self.helper.parse_log(
                    log_line(durable=durable, elapsed=elapsed)))
                self.assertFalse(receipt["accepted"])
                self.assertEqual(receipt["samples"][0]["elapsed_micros"], int(elapsed))
                self.assertEqual(len(receipt["samples"]), 1)

    def test_genesis_audit_sequence_zero_is_a_valid_stored_event(self):
        receipt = self.receipt(rows=[{**event(), "sequence": "0"}],
                               database={**DATABASE, "events_before_first_access": 0})
        self.assertTrue(receipt["accepted"])
        self.assertEqual(receipt["samples"][0]["sequence"], "0")

    def test_empty_or_duplicate_log_or_database_request_identity_is_rejected(self):
        logs = self.helper.parse_log(log_line())
        for samples, rows in [([], []), (logs * 2, [event()]), (logs, [event(), event()])]:
            with self.subTest(samples=len(samples), rows=len(rows)), self.assertRaises(ValueError):
                self.receipt(logs=samples, rows=rows)

    def test_missing_extra_or_mismatched_stored_event_is_rejected(self):
        changed = event()
        changed["context"]["request_id"] = OTHER
        wrong_route = {**event(), "operation": "POST /api/v1/auth/login"}
        for rows in [[], [event(), changed], [changed], [wrong_route]]:
            with self.subTest(rows=rows), self.assertRaises(ValueError):
                self.receipt(rows=rows)

    def test_context_requires_http_valid_address_and_canonical_optional_user_uuid(self):
        for field, value in [("source", "cli"), ("ip_address", None),
                             ("ip_address", "not-an-ip"), ("user_id", "owner@example.test")]:
            row = event()
            row["context"][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                self.receipt(rows=[row])
        row = event()
        row["context"]["user_id"] = None
        row["action"] = "access.denied"
        self.assertTrue(self.receipt(rows=[row])["accepted"])

    def test_all_four_outcomes_are_supported_without_equating_rejection_with_denial(self):
        for action in ["access.granted", "access.denied", "access.rejected", "access.failed"]:
            row = {**event(), "action": action}
            with self.subTest(action=action):
                self.assertEqual(self.receipt(rows=[row])["samples"][0]["action"], action)
        with self.assertRaises(ValueError):
            self.receipt(rows=[{**event(), "action": "case.read"}])

    def test_operation_must_be_an_exact_catalog_pair_without_query_or_resource_suffix(self):
        for operation in ["GET /healthz", OPERATION + "?token=secret", OPERATION + " user=" + USER]:
            with self.subTest(operation=operation), self.assertRaises(ValueError):
                self.receipt(logs=self.helper.parse_log(log_line(operation=operation)))

    def test_source_catalog_has_222_unique_functional_pairs(self):
        catalog = self.helper.read_catalog(SCRIPTS.parent / "docs/access-audit-matrix.md")
        self.assertEqual(len(catalog), 222)
        self.assertIn(OPERATION, catalog)
        self.assertTrue(all(not pair.startswith("HEAD ") for pair in catalog))

    def test_receipt_rejects_disabled_database_durability(self):
        for setting in ["fsync", "full_page_writes", "synchronous_commit"]:
            with self.subTest(setting=setting), self.assertRaises(ValueError):
                self.receipt(database={**DATABASE, setting: "off"})

    def test_receipt_preserves_only_explicit_safe_fields(self):
        row = {**event(), "actor": "private@example.test", "resource": "token=private-token"}
        logs = self.helper.parse_log(log_line() + ' extra="private-token"')
        receipt = self.receipt(logs=logs, rows=[row])
        encoded = json.dumps(receipt)
        self.assertNotIn("private-token", encoded)
        self.assertNotIn("private@example.test", encoded)

    def test_database_url_stays_out_of_command_and_errors(self):
        connection = "postgresql://private-user:private-password@localhost/postgres"
        output = "\n".join([json.dumps(DATABASE), json.dumps(event())])
        with patch.object(self.helper.subprocess, "run") as run:
            run.return_value.returncode = 0
            run.return_value.stdout = output
            self.assertEqual(self.helper.read_snapshot(connection), (DATABASE, [event()]))
            args, kwargs = run.call_args
            self.assertNotIn(connection, repr(args))
            self.assertEqual(kwargs["env"]["PGDATABASE"], "postgres")
            self.assertEqual(kwargs["env"]["PGHOST"], "localhost")
            self.assertEqual(kwargs["env"]["PGPORT"], "5432")
            self.assertEqual(kwargs["env"]["PGUSER"], "private-user")
            self.assertEqual(kwargs["env"]["PGPASSWORD"], "private-password")
            self.assertIn("READ ONLY", kwargs["input"])
            self.assertNotIn("password_hash", kwargs["input"])
            run.return_value.returncode = 1
            run.return_value.stderr = connection
            with self.assertRaises(ValueError) as failure:
                self.helper.read_snapshot(connection)
            self.assertNotIn("private-password", str(failure.exception))

    def test_reconciliation_error_has_safe_reason_and_capture_can_be_replayed(self):
        logs = self.helper.parse_log(log_line())
        capture = self.helper.safe_capture(logs, [event()], CATALOG,
                                           "local-working-copy", ENVIRONMENT, DATABASE)
        self.assertEqual(self.helper.build_receipt(**capture), self.receipt())
        with self.assertRaises(self.helper.EvidenceError) as failure:
            self.helper.build_receipt(**{**capture, "logs": []})
        self.assertEqual(str(failure.exception), "no audit commit samples were observed")
        private = {**event(), "actor": "private@example.test"}
        sanitized = self.helper.safe_capture(logs, [private], CATALOG,
                                             "local-working-copy", ENVIRONMENT, DATABASE)
        self.assertNotIn("private@example.test", json.dumps(sanitized))

    def test_failed_threshold_receipt_is_private_and_never_overwritten(self):
        receipt = self.receipt(logs=self.helper.parse_log(log_line(elapsed="500000")))
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "receipt.json"
            self.helper.write_receipt(path, receipt)
            self.assertEqual(json.loads(path.read_text()), receipt)
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
            with self.assertRaises(FileExistsError):
                self.helper.write_receipt(path, copy.deepcopy(receipt))


if __name__ == "__main__":
    unittest.main()
