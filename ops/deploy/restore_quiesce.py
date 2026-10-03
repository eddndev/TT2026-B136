"""Close exact managed services durably without restoring or reopening data."""
import math
import os
from pathlib import Path
from time import monotonic

import crl_journal as journal
import restore_fence as fence
import restore_quiesce_target as target_files
from backup_manifest_files import inspect, present
from backup_manifest_schema import decode, hexadecimal, keys
from restore_commands import run
from restore_quiesce_observe import listening_ports, process_snapshot


UNITS = target_files.UNITS
PROPERTIES = ("Id", "LoadState", "ActiveState", "SubState", "Result", "ExecMainCode",
              "ExecMainStatus", "MainPID", "ControlPID", "ControlGroup", "FragmentPath",
              "DropInPaths", "NeedDaemonReload", "WorkingDirectory", "TimeoutStopUSec")
ERROR = "restore service closure failed; admission must remain closed"


def remaining(deadline):
    value = deadline - monotonic()
    if value <= 0:
        raise RuntimeError("restore service closure deadline expired")
    return value


def process_id(value):
    if not isinstance(value, str) or not value.isascii() or not value.isdecimal() or not 0 <= int(value) < 2**31:
        raise ValueError("restore service process identifier is invalid")
    return int(value)


def command(target, env, deadline, action, units):
    timeout = remaining(deadline)
    if action == "stop" and timeout < 90:
        raise RuntimeError("restore service closure lacks the configured stop budget")
    arguments = [Path(target["systemctl"]["path"]), "--user", action, *units]
    if action == "show":
        arguments.append("--property=" + ",".join(PROPERTIES))
    result = run(arguments, env=env, timeout=timeout if action == "stop" else min(timeout, 10),
                 maximum=65536, text=True)
    remaining(deadline)
    return result.stdout


def snapshot(target, env, deadline, units):
    raw = command(target, env, deadline, "show", units)
    if not isinstance(raw, str) or not raw.isascii() or not 0 < len(raw) <= 65536:
        raise ValueError("restore service snapshot exceeds its boundary")
    rows = {}
    for block in raw.strip().split("\n\n"):
        row = {}
        for line in block.splitlines():
            key, separator, value = line.partition("=")
            if not separator or key in row:
                raise ValueError("restore service snapshot is ambiguous")
            row[key] = value
        keys(row, PROPERTIES)
        unit = row["Id"]
        if unit not in units or unit in rows:
            raise ValueError("restore service snapshot belongs to another unit")
        expected = target["units"][unit]
        main, control = process_id(row["MainPID"]), process_id(row["ControlPID"])
        group = row["ControlGroup"]
        absent_group = not group and row["ActiveState"] == "inactive" and main == control == 0
        if (row["LoadState"] != "loaded" or row["FragmentPath"] != expected["fragment"]["path"]
                or row["WorkingDirectory"] != expected["working_directory"]
                or (group != expected["control_group"] and not absent_group)
                or row["DropInPaths"] or row["NeedDaemonReload"] != "no"
                or row["TimeoutStopUSec"] not in ("1min 30s", "90s", "90000000")
                or row["ActiveState"] not in ("active", "inactive") or control != 0):
            raise ValueError("restore loaded service identity or state differs")
        if ((row["ActiveState"] == "active" and (main == 0 or row["SubState"] != "running"))
                or (row["ActiveState"] == "inactive" and (main != 0 or row["SubState"] != "dead"))):
            raise ValueError("restore service process and activity disagree")
        rows[unit] = row
    if set(rows) != set(units):
        raise ValueError("restore service snapshot is incomplete")
    return rows


def owned_processes(target, deadline, unit, row):
    group = target["units"][unit]["control_group"]
    members = process_snapshot(group, timeout=min(remaining(deadline), 10))
    remaining(deadline)
    if type(members) is not list or len(members) > 4096:
        raise ValueError("restore service process snapshot exceeds its boundary")
    identifiers = set()
    for item in members:
        keys(item, ("pid", "uid", "start_ticks", "control_group"))
        if (any(type(item[name]) is not int for name in ("pid", "uid", "start_ticks"))
                or not 0 < item["pid"] < 2**31 or item["pid"] in identifiers
                or item["uid"] != target["root"]["uid"] or item["start_ticks"] <= 0
                or not isinstance(item["control_group"], str)
                or not (item["control_group"] == group or item["control_group"].startswith(group + "/"))):
            raise ValueError("restore service contains a process with another identity")
        identifiers.add(item["pid"])
    main = process_id(row["MainPID"])
    if main and main not in identifiers:
        raise ValueError("restore service main process is not in its owned cgroup")
    return members


def stopped(target, env, deadline, units):
    rows = snapshot(target, env, deadline, units)
    for unit, row in rows.items():
        if (row["ActiveState"] != "inactive" or row["Result"] != "success"
                or row["ExecMainCode"] not in ("0", "1") or row["ExecMainStatus"] != "0"
                or owned_processes(target, deadline, unit, row)):
            raise ValueError("restore service did not finish normally or still owns processes")
    ports = [target["ports"][unit] for unit in units]
    listeners = listening_ports(ports, timeout=min(remaining(deadline), 10))
    remaining(deadline)
    if type(listeners) is not list or listeners:
        raise ValueError("restore service listener remains open or is ambiguous")


def saved(root, operation_id, digest):
    operation = root / "maintenance/restore" / operation_id
    if present(operation):
        target_files.directory(operation, private=True)
    path = operation / "journal.json"
    if present(path):
        target_files.exact_path(path)
        if path.stat().st_uid != os.getuid():
            raise ValueError("restore service journal ownership differs")
        _, raw = inspect(path, 4096, private=True, contents=True)
        record = decode(raw)
        keys(record, ("operation_id", "target_sha256", "state"))
        if (record["operation_id"] != operation_id or record["target_sha256"] != digest
                or record["state"] not in ("closing", "stopped")):
            raise ValueError("restore service journal belongs to another operation or target")
    return operation, path


def quiesce(root, operation_id, *, target_path, expected_target_sha256, timeout=240):
    try:
        if type(timeout) not in (int, float) or not math.isfinite(timeout) or not 90 <= timeout <= 300:
            raise ValueError("restore service closure timeout is invalid")
        deadline = monotonic() + timeout
        fence._identifier(operation_id)
        hexadecimal(expected_target_sha256, 64)
        target_files.root_identity(root)
        with fence.maintenance(root, operation_id) as close_admission:
            target, env = target_files.load(root, target_path, expected_target_sha256)
            operation, path = saved(root, operation_id, expected_target_sha256)
            rows = snapshot(target, env, deadline, UNITS)
            for unit, row in rows.items():
                owned_processes(target, deadline, unit, row)
            target_files.load(root, target_path, expected_target_sha256)
            remaining(deadline)
            close_admission()
            if not operation.exists():
                journal.directory(operation)
            record = {"operation_id": operation_id, "target_sha256": expected_target_sha256, "state": "closing"}
            journal.save(path, record)
            for group in (UNITS[:2], UNITS[2:]):
                target_files.load(root, target_path, expected_target_sha256)
                if any(rows[unit]["ActiveState"] != "inactive" for unit in group):
                    command(target, env, deadline, "stop", group)
                stopped(target, env, deadline, group)
            target_files.load(root, target_path, expected_target_sha256)
            stopped(target, env, deadline, UNITS)
            remaining(deadline)
            record["state"] = "stopped"
            journal.save(path, record)
            remaining(deadline)
            return record
    except (OSError, ValueError, RuntimeError, KeyError, TypeError, UnicodeError, RecursionError):
        raise RuntimeError(ERROR) from None
