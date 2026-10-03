"""Close exact managed services durably without restoring or reopening data."""
import math
import os
from pathlib import Path
import re
from time import monotonic

import crl_journal as journal
import restore_fence as fence
import restore_quiesce_target as target_files
from backup_manifest_files import inspect, present
from backup_manifest_schema import decode, hexadecimal, keys
from restore_commands import run
from restore_quiesce_bus import retain_units
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
    return rows


def receipt(target, unit, value):
    keys(value, ("manager", "process", "started_usec", "exit"))
    process = value["process"]
    keys(process, ("pid", "uid", "start_ticks", "control_group"))
    group = target["units"][unit]["control_group"]
    if (not isinstance(value["manager"], str) or len(value["manager"]) > 256
            or not re.fullmatch(r":[0-9]+\.[0-9]+", value["manager"])
            or any(type(process[key]) is not int for key in ("pid", "uid", "start_ticks"))
            or not 0 < process["pid"] < 2**31 or process["uid"] != target["root"]["uid"]
            or not 0 < process["start_ticks"] < 2**64
            or not isinstance(process["control_group"], str) or len(process["control_group"]) > 1024
            or not (process["control_group"] == group or process["control_group"].startswith(group + "/"))
            or type(value["started_usec"]) is not int or not 0 < value["started_usec"] < 2**64):
        raise ValueError("restore captured service identity is invalid")
    if value["exit"] is not None:
        exit_value = value["exit"]
        keys(exit_value, ("code", "status", "exited_usec"))
        if (any(type(entry) is not int for entry in exit_value.values())
                or exit_value["code"] != 1 or exit_value["status"] != 0
                or not value["started_usec"] <= exit_value["exited_usec"] < 2**64):
            raise ValueError("restore captured service exit is invalid")


def saved(root, operation_id, digest, target):
    operation = root / "maintenance/restore" / operation_id
    if present(operation):
        target_files.directory(operation, private=True)
    path = operation / "journal.json"
    record = {"operation_id": operation_id, "target_sha256": digest, "state": "closing", "units": {}}
    if present(path):
        target_files.exact_path(path)
        if path.stat().st_uid != os.getuid():
            raise ValueError("restore service journal ownership differs")
        _, raw = inspect(path, 16384, private=True, contents=True)
        record = decode(raw)
        if type(record) is dict and "units" not in record:
            keys(record, ("operation_id", "target_sha256", "state"))
            record["units"] = {}
        keys(record, ("operation_id", "target_sha256", "state", "units"))
        if (record["operation_id"] != operation_id or record["target_sha256"] != digest
                or record["state"] not in ("closing", "stopped")):
            raise ValueError("restore service journal belongs to another operation or target")
        units = record["units"]
        if type(units) is not dict or (units and set(units) != set(UNITS)):
            raise ValueError("restore service journal unit inventory differs")
        for unit, value in units.items():
            receipt(target, unit, value)
            if record["state"] == "stopped" and value["exit"] is None:
                raise ValueError("restore stopped journal lacks captured exit evidence")
    return operation, path, record


def status(references, deadline, unit, row):
    value = references.status(unit, timeout=min(remaining(deadline), 10))
    remaining(deadline)
    keys(value, ("pid", "code", "status", "started_usec", "exited_usec"))
    if (any(type(item) is not int for item in value.values())
            or not 0 <= value["pid"] < 2**31 or not 0 <= value["code"] < 2**31
            or not 0 <= value["status"] < 2**31
            or any(not 0 <= value[key] < 2**64 for key in ("started_usec", "exited_usec"))
            or row["ExecMainCode"] != str(value["code"]) or row["ExecMainStatus"] != str(value["status"])):
        raise ValueError("restore manager exit observation differs or is invalid")
    return value


def exited(previous, observed, manager):
    if previous is None:
        raise ValueError("restore inactive service lacks a captured process identity")
    if previous["exit"] is not None and all(observed[key] == 0 for key in observed):
        return previous
    if (previous["manager"] != manager or observed["pid"] != previous["process"]["pid"]
            or observed["started_usec"] != previous["started_usec"]
            or observed["code"] != 1 or observed["status"] != 0
            or observed["exited_usec"] < previous["started_usec"]):
        raise ValueError("restore service did not retain its exact normal exit")
    result = {key: observed[key] for key in ("code", "status", "exited_usec")}
    if previous["exit"] is not None and previous["exit"] != result:
        raise ValueError("restore service exit differs from its captured evidence")
    return {**previous, "exit": result}


def capture(target, deadline, references, unit, row, previous):
    members = owned_processes(target, deadline, unit, row)
    observed = status(references, deadline, unit, row)
    if row["ActiveState"] == "inactive":
        if members or row["Result"] != "success":
            raise ValueError("restore inactive service still owns processes or failed")
        return exited(previous, observed, references.manager)
    main = process_id(row["MainPID"])
    if (observed["pid"] != main or observed["started_usec"] == 0
            or any(observed[key] != 0 for key in ("code", "status", "exited_usec"))):
        raise ValueError("restore active service lacks its exact start identity")
    process = next(item for item in members if item["pid"] == main)
    value = {"manager": references.manager, "process": process,
             "started_usec": observed["started_usec"], "exit": None}
    receipt(target, unit, value)
    return value


def observe_exits(target, env, deadline, references, units, record):
    references.check(timeout=remaining(deadline))
    rows = stopped(target, env, deadline, units)
    for unit, row in rows.items():
        observed = status(references, deadline, unit, row)
        record["units"][unit] = exited(record["units"][unit], observed, references.manager)


def close_units(root, target_path, digest, target, env, deadline, references, path, record):
    for group in (UNITS[:2], UNITS[2:]):
        target_files.load(root, target_path, digest)
        references.check(timeout=remaining(deadline))
        rows = snapshot(target, env, deadline, group)
        active = []
        for unit, row in rows.items():
            previous = record["units"][unit]
            current = capture(target, deadline, references, unit, row, previous)
            if row["ActiveState"] == "active":
                if current != previous:
                    raise ValueError("restore service restarted after its process capture")
                active.append(unit)
            record["units"][unit] = current
        journal.save(path, record)
        if active:
            command(target, env, deadline, "stop", active)
        observe_exits(target, env, deadline, references, group, record)
        journal.save(path, record)


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
            operation, path, record = saved(root, operation_id, expected_target_sha256, target)
            with retain_units(env, UNITS, timeout=remaining(deadline)) as references:
                references.check(timeout=remaining(deadline))
                rows = snapshot(target, env, deadline, UNITS)
                for unit, row in rows.items():
                    record["units"][unit] = capture(target, deadline, references, unit, row, record["units"].get(unit))
                target_files.load(root, target_path, expected_target_sha256)
                remaining(deadline)
                close_admission()
                if not operation.exists():
                    journal.directory(operation)
                record["state"] = "closing"
                journal.save(path, record)
                close_units(root, target_path, expected_target_sha256, target, env, deadline, references, path, record)
                target_files.load(root, target_path, expected_target_sha256)
                observe_exits(target, env, deadline, references, UNITS, record)
                remaining(deadline)
                record["state"] = "stopped"
                journal.save(path, record)
                references.check(timeout=remaining(deadline))
                return {key: record[key] for key in ("operation_id", "target_sha256", "state")}
    except (OSError, ValueError, RuntimeError, KeyError, TypeError, UnicodeError, RecursionError):
        raise RuntimeError(ERROR) from None
