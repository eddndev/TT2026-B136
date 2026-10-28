#!/usr/bin/env python3
"""Reconcile native HTTP commit timings with durable public audit fields."""
import argparse
import ipaddress
import json
import math
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
from uuid import UUID
from urllib.parse import unquote, urlsplit


LIMIT_MICROS = 500000
ACTIONS = {"access.granted", "access.denied", "access.rejected", "access.failed"}
CONTEXT_FIELDS = {"source", "user_id", "ip_address", "request_id"}
ENVIRONMENT_FIELDS = {"os", "cpu", "memory_bytes", "postgres_disk", "build_profile",
                      "temporary_filesystem"}
DATABASE_FIELDS = {"postgres_version", "fsync", "full_page_writes", "synchronous_commit",
                   "audit_events_count", "events_before_first_access"}
ANSI = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
TARGET = re.compile(r"^\S+\s+INFO\s+qadra::audit_commit:\s*(.*)$")
SQL = """BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY;
SELECT jsonb_build_object(
  'postgres_version',current_setting('server_version'),
  'fsync',current_setting('fsync'),
  'full_page_writes',current_setting('full_page_writes'),
  'synchronous_commit',current_setting('synchronous_commit'),
  'audit_events_count',(SELECT count(*) FROM audit_events),
  'events_before_first_access',(SELECT count(*) FROM audit_events
    WHERE sequence < (SELECT min(sequence) FROM audit_events
      WHERE action IN ('access.granted','access.denied','access.rejected','access.failed'))));
SELECT jsonb_build_object('sequence',sequence::text,'action',action,
  'operation',split_part(resource,' ',1)||' '||split_part(resource,' ',2),
  'context',context_json::jsonb)
FROM audit_events
WHERE action IN ('access.granted','access.denied','access.rejected','access.failed')
ORDER BY sequence;
COMMIT;
"""


class EvidenceError(ValueError):
    """A fixed diagnostic that never includes source data or connection details."""


def require(condition, message):
    if not condition:
        raise EvidenceError(message)


def canonical_uuid(value):
    try:
        require(isinstance(value, str) and str(UUID(value)) == value,
                "invalid canonical audit identity")
    except (AttributeError, ValueError):
        raise EvidenceError("invalid canonical audit identity") from None
    return value


def parse_log(text):
    samples = []
    required = {"request_id", "operation", "durable", "elapsed_micros"}
    for number, line in enumerate(text.splitlines(), 1):
        matched = TARGET.fullmatch(ANSI.sub("", line))
        if not matched:
            continue
        try:
            tokens = shlex.split(matched.group(1))
            fields = {}
            for token in tokens:
                key, separator, value = token.partition("=")
                if separator and key in required:
                    require(key not in fields, "duplicate commit field")
                    fields[key] = value
            require(set(fields) == required, "missing commit field")
            request = re.fullmatch(r"Some\(([a-f0-9-]{36})\)", fields["request_id"])
            require(request is not None, "missing commit identity")
            request_id = canonical_uuid(request.group(1))
            require(fields["durable"] in {"true", "false"}, "invalid durable result")
            require(re.fullmatch(r"[0-9]+", fields["elapsed_micros"]) is not None,
                    "invalid commit duration")
            elapsed = int(fields["elapsed_micros"])
            require(elapsed <= 2**64 - 1, "commit duration exceeds representation")
        except ValueError:
            raise EvidenceError("invalid audit commit record at line " + str(number)) from None
        samples.append({"request_id": request_id, "operation": fields["operation"],
                        "durable": fields["durable"] == "true", "elapsed_micros": elapsed,
                        "log_line": number})
    return samples


def read_catalog(path):
    text = Path(path).read_text(encoding="utf-8")
    blocks = text.split("```tsv\n")
    require(len(blocks) > 1, "functional route catalog missing")
    lines = blocks[1].split("```", 1)[0].strip().splitlines()
    require(lines[0] == "method\tpath\tpolicy\tsource", "invalid route catalog header")
    pairs = []
    for line in lines[1:]:
        columns = line.split("\t")
        require(len(columns) == 4 and columns[0] in {"GET", "POST", "PUT", "DELETE"},
                "invalid functional route catalog row")
        pairs.append(columns[0] + " " + columns[1])
    require(len(pairs) == 223 and len(set(pairs)) == 223, "functional catalog changed")
    return set(pairs)


def connection_environment(connection):
    parsed = urlsplit(connection)
    require(parsed.scheme in {"postgres", "postgresql"} and parsed.hostname
            and parsed.path.startswith("/") and len(parsed.path) > 1
            and not parsed.query and not parsed.fragment, "invalid database URI")
    environment = {key: value for key, value in os.environ.items()
                   if key not in {"PGHOST", "PGHOSTADDR", "PGPORT", "PGDATABASE", "PGUSER",
                                  "PGPASSWORD", "PGSERVICE", "PGSERVICEFILE", "PGOPTIONS"}}
    environment.update(PGHOST=parsed.hostname, PGPORT=str(parsed.port or 5432),
                       PGDATABASE=unquote(parsed.path[1:]))
    if parsed.username is not None:
        environment["PGUSER"] = unquote(parsed.username)
    if parsed.password is not None:
        environment["PGPASSWORD"] = unquote(parsed.password)
    return environment


def read_snapshot(connection):
    try:
        environment = connection_environment(connection)
        result = subprocess.run(["psql", "-XqAt", "-v", "ON_ERROR_STOP=1"],
                                input=SQL, env=environment, capture_output=True,
                                text=True, timeout=30, check=False)
        require(result.returncode == 0, "database snapshot failed")
        values = [json.loads(line) for line in result.stdout.splitlines() if line]
        require(values and all(isinstance(value, dict) for value in values),
                "invalid database snapshot")
        return values[0], values[1:]
    except (OSError, subprocess.SubprocessError, ValueError):
        raise EvidenceError("database snapshot failed") from None


def public_context(context):
    require(isinstance(context, dict) and set(context) == CONTEXT_FIELDS,
            "invalid stored context fields")
    require(context["source"] == "http", "stored access source is not HTTP")
    request_id = canonical_uuid(context["request_id"])
    user_id = context["user_id"]
    if user_id is not None:
        canonical_uuid(user_id)
    try:
        require(isinstance(context["ip_address"], str), "missing observed address")
        address = str(ipaddress.ip_address(context["ip_address"]))
    except ValueError:
        raise EvidenceError("invalid stored HTTP address") from None
    return {"source": "http", "user_id": user_id, "ip_address": address,
            "request_id": request_id}


def safe_metadata(revision, environment, database):
    require(isinstance(revision, str) and 0 < len(revision) <= 200,
            "measured revision missing")
    require(isinstance(environment, dict) and ENVIRONMENT_FIELDS <= set(environment),
            "measurement environment missing")
    for field in ENVIRONMENT_FIELDS - {"memory_bytes"}:
        require(isinstance(environment[field], str) and environment[field],
                "invalid measurement environment")
    require(type(environment["memory_bytes"]) is int and environment["memory_bytes"] > 0,
            "invalid measured memory")
    require(isinstance(database, dict) and DATABASE_FIELDS <= set(database),
            "database evidence missing")
    for field in ["fsync", "full_page_writes", "synchronous_commit"]:
        require(database[field] == "on", "PostgreSQL durability is not enabled")
    require(isinstance(database["postgres_version"], str) and database["postgres_version"],
            "PostgreSQL version missing")
    for field in ["audit_events_count", "events_before_first_access"]:
        require(type(database[field]) is int and database[field] >= 0,
                "invalid database history count")
    require(database["events_before_first_access"] <= database["audit_events_count"],
            "invalid historical prefix count")
    return ({key: environment[key] for key in sorted(ENVIRONMENT_FIELDS)},
            {key: database[key] for key in sorted(DATABASE_FIELDS)})


def durations(samples):
    ordered = sorted(sample["elapsed_micros"] for sample in samples if sample["durable"])
    if not ordered:
        return {"durable_count": 0, "min": None, "max": None,
                "p50": None, "p95": None, "p99": None}
    return {"durable_count": len(ordered), "min": ordered[0], "max": ordered[-1],
            **{"p" + str(percent): ordered[math.ceil(percent * len(ordered) / 100) - 1]
               for percent in [50, 95, 99]}}


def safe_capture(logs, rows, catalog, revision, environment, database):
    environment, database = safe_metadata(revision, environment, database)
    safe_logs, safe_rows = [], []
    for sample in logs:
        require(sample["operation"] in catalog, "observed operation is outside catalog")
        canonical_uuid(sample["request_id"])
        require(type(sample["durable"]) is bool and type(sample["elapsed_micros"]) is int
                and type(sample["log_line"]) is int, "invalid commit sample")
        safe_logs.append({key: sample[key] for key in
                          ["request_id", "operation", "durable", "elapsed_micros", "log_line"]})
    for row in rows:
        require(row["operation"] in catalog, "stored operation is outside catalog")
        require(row["action"] in ACTIONS, "invalid stored access action")
        require(isinstance(row["sequence"], str) and row["sequence"].isdecimal(),
                "invalid stored audit sequence")
        safe_rows.append({**{key: row[key] for key in ["sequence", "action", "operation"]},
                          "context": public_context(row["context"])})
    return {"logs": safe_logs, "rows": safe_rows, "catalog": sorted(catalog),
            "revision": revision, "environment": environment, "database": database}


def build_receipt(logs, rows, catalog, revision, environment, database):
    catalog = set(catalog)
    environment, database = safe_metadata(revision, environment, database)
    require(logs, "no audit commit samples were observed")
    stored, sequences = {}, set()
    for row in rows:
        require(isinstance(row, dict) and row.get("action") in ACTIONS,
                "invalid stored access action")
        require(row.get("operation") in catalog, "stored operation is outside catalog")
        sequence = row.get("sequence")
        require(isinstance(sequence, str) and re.fullmatch(r"(?:0|[1-9][0-9]*)", sequence)
                and int(sequence) <= 2**63 - 1, "invalid stored audit sequence")
        context = public_context(row.get("context"))
        request_id = context["request_id"]
        require(request_id not in stored and sequence not in sequences,
                "duplicate stored access identity")
        stored[request_id] = {"sequence": sequence, "action": row["action"],
                              "operation": row["operation"], "context": context}
        sequences.add(sequence)
    samples, seen, violations = [], set(), []
    for log in logs:
        request_id = canonical_uuid(log["request_id"])
        require(request_id not in seen, "duplicate commit log identity")
        require(request_id in stored, "commit has no stored access event")
        require(log["operation"] in catalog, "observed operation is outside catalog")
        row = stored[request_id]
        require(log["operation"] == row["operation"], "stored and observed operations differ")
        require(type(log["durable"]) is bool and type(log["elapsed_micros"]) is int
                and log["elapsed_micros"] >= 0, "invalid commit sample")
        sample = {key: log[key] for key in
                  ["request_id", "operation", "durable", "elapsed_micros", "log_line"]}
        sample.update({key: row[key] for key in ["sequence", "action", "context"]})
        samples.append(sample)
        seen.add(request_id)
        if not sample["durable"]:
            violations.append({"request_id": request_id, "reason": "commit_not_durable"})
        if sample["elapsed_micros"] >= LIMIT_MICROS:
            violations.append({"request_id": request_id, "reason": "latency_limit"})
    require(seen == set(stored), "stored access event has no commit log")
    require(database["audit_events_count"] >= len(samples), "inconsistent audit history count")
    operations = {sample["operation"] for sample in samples}
    return {"schema_version": 1, "population": "native_http_workflows_through_pre_restore_capture",
            "revision": revision, "environment": environment, "database": database,
            "declared_http_limits": {"max_requests_per_server": 8, "blocking_handler_operations_per_server": 2,
                                     "server_generations": 5, "max_simultaneous_servers": 2},
            "history_model": "growing_workflow_history", "limit_micros_exclusive": LIMIT_MICROS,
            "timer": "before_tokio_spawn_blocking_through_access_commit_return",
            "excludes": ["handler_execution", "http_admission", "handler_work_budget_wait"],
            "sample_count": len(samples), "catalog_operation_count": len(catalog),
            "distinct_operations_seen": sorted(operations),
            "operations_missing": sorted(catalog - operations), "universal_route_coverage": False,
            "duration_micros": durations(samples), "samples": samples,
            "violations": violations, "accepted": not violations}


def write_receipt(path, receipt):
    path = Path(path)
    require(path.is_absolute(), "receipt path must be absolute")
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w", encoding="ascii") as stream:
        json.dump(receipt, stream, ensure_ascii=True, sort_keys=True, indent=2)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-log", required=True, type=Path)
    parser.add_argument("--receipt", required=True, type=Path)
    args = parser.parse_args()
    try:
        samples = parse_log(args.server_log.read_text(encoding="utf-8"))
        database, rows = read_snapshot(os.environ["TT_ACCESS_AUDIT_DATABASE_URL"])
        catalog = read_catalog(Path(__file__).resolve().parents[1] / "docs/access-audit-matrix.md")
        capture = safe_capture(samples, rows, catalog, os.environ["AUDIT_ACCESS_REVISION"],
                               json.loads(os.environ["AUDIT_ACCESS_ENVIRONMENT"]), database)
        write_receipt(args.receipt.with_name(args.receipt.name + ".inputs.json"), capture)
        receipt = build_receipt(**capture)
        write_receipt(args.receipt, receipt)
    except EvidenceError as error:
        print("HTTP audit receipt failed: " + str(error), file=sys.stderr)
        return 1
    except (OSError, ValueError, KeyError, TypeError):
        print("HTTP audit receipt failed: invalid inputs or inconsistent evidence", file=sys.stderr)
        return 1
    print("HTTP audit receipt: " + str(receipt["sample_count"]) + " commits, "
          + str(len(receipt["distinct_operations_seen"])) + "/" + str(len(catalog))
          + " operations observed, accepted=" + str(receipt["accepted"]).lower())
    return 0 if receipt["accepted"] else 1


if __name__ == "__main__":
    sys.exit(main())
