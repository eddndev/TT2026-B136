"""Durably mask four attested service entries without stopping or reopening them."""
import math
from pathlib import Path
from time import monotonic

from backup_manifest_files import present, sync_directory
from backup_manifest_schema import hexadecimal, keys
import controller_gate_files as gate_files
import controller_gate_target as targets
import controller_publication_files as files
from controller_publication import _identifier
import crl_journal as journal
from restore_commands import run
from restore_quiesce_observe import process_snapshot


UNITS = targets.UNITS
ERROR = "controller entry gate failed; closure must be reconciled before further work"
_exchange = gate_files.exchange


def _remaining(deadline):
    result = deadline - monotonic()
    if result <= 0:
        raise RuntimeError("controller entry gate deadline expired")
    return result


def _command(target, environment, deadline, action):
    arguments = [Path(target["systemctl"]["path"]), "--user", action]
    if action == "show":
        arguments.extend((*UNITS, "--property=" + ",".join(targets.PROPERTIES)))
    elif action != "daemon-reload":
        raise ValueError("controller entry gate command is unsupported")
    result = run(arguments, env=environment, timeout=min(_remaining(deadline), 10),
                 maximum=65536, text=True)
    _remaining(deadline)
    return result.stdout


def _processes(target, rows, deadline):
    for unit, row in rows.items():
        group = target["units"][unit]["control_group"]
        members = process_snapshot(group, timeout=min(_remaining(deadline), 10))
        _remaining(deadline)
        if type(members) is not list or len(members) > 4096:
            raise ValueError("controller gate process observation exceeds its boundary")
        identifiers = set()
        for member in members:
            keys(member, ("pid", "uid", "start_ticks", "control_group"))
            if (any(type(member[key]) is not int for key in ("pid", "uid", "start_ticks"))
                    or not 0 < member["pid"] < 2**31 or member["pid"] in identifiers
                    or member["uid"] != target["root"]["uid"] or member["start_ticks"] <= 0
                    or not isinstance(member["control_group"], str)
                    or not (member["control_group"] == group or member["control_group"].startswith(group + "/"))):
                raise ValueError("controller gate contains a process with another identity")
            identifiers.add(member["pid"])
        main = targets.process_id(row["MainPID"])
        if main and main not in identifiers:
            raise ValueError("controller gate main process is absent from its owned group")


def _observe(target, environment, deadline, orientations, *, complete=False):
    rows = targets.rows(_command(target, environment, deadline, "show"), target,
                        orientations, complete=complete)
    _processes(target, rows, deadline)


def _active(marker, expected):
    if present(marker):
        observed = gate_files.read(marker)
        keys(observed, ("operation_id", "target_sha256"))
        if observed != expected:
            raise ValueError("controller entry gate belongs to another operation")
        return True
    return False


def _live(root, target_path, digest, expected_root, lock, expected_target=None):
    if files.located(root) != expected_root:
        raise ValueError("controller entry gate root changed")
    files.unchanged(root / "deploy.lock", lock, directory=False)
    observed = targets.load(root, target_path, digest)
    if expected_target is not None and observed != expected_target:
        raise ValueError("controller entry gate target context changed")
    return observed


def _originals(unit_directory, target):
    result = {unit: {"original": gate_files.regular(unit_directory / unit)} for unit in UNITS}
    targets.attest_originals(result, target)
    return result


def _prepare(root, operation, target, digest, lock, unit_directory, originals, recheck):
    paths = gate_files.directories(root, operation)
    identities = {name: gate_files.directory(path, sync_directory) for name, path in paths.items()}
    entries = {}
    for unit in UNITS:
        gate_files.sync_file(unit_directory / unit)
        slot = paths["slots"] / unit
        slot.symlink_to("/dev/null")
        entries[unit] = {**originals[unit], "mask": gate_files.mask(slot)}
    sync_directory(paths["slots"])
    sync_directory(operation)
    recheck()
    if _originals(unit_directory, target) != originals:
        raise ValueError("controller entry originals changed during preparation")
    value = {"format": "qadra-controller-entry-gate", "version": 1, "operation_id": operation.name,
             "target_sha256": digest, "target": target, "root": files.located(root), "lock": lock,
             "unit_directory": files.located(unit_directory), "directories": identities,
             "units": entries, "state": "prepared"}
    gate_files.validate_record(value, root, operation, target, digest, lock, unit_directory, UNITS)
    gate_files.save(operation / "journal.json", value, sync_directory)
    return value


def _state(operation, value, state):
    if value["state"] == state:
        gate_files.sync_file(operation / "journal.json")
        sync_directory(operation)
        return value
    changed = {**value, "state": state}
    gate_files.save(operation / "journal.json", changed, sync_directory, previous=value)
    return changed


def _close_locked(root, operation_id, *, target_path, expected_target_sha256,
                  deadline, prepare_only=False):
    """Prepare or finish exact masks while the caller continuously owns deploy.lock."""
    expected_root = files.located(root)
    lock = files.identity(root / "deploy.lock", directory=False)
    operation = root / "maintenance/controller-installation" / operation_id
    marker = operation.parent / "active.json"
    active = {"operation_id": operation_id, "target_sha256": expected_target_sha256}
    target_context = _live(root, target_path, expected_target_sha256, expected_root, lock)
    target, environment, unit_directory = target_context
    unit_identity = files.located(unit_directory)
    for path in (root / "maintenance", operation.parent):
        if present(path):
            files.identity(path)
    marked = _active(marker, active)
    value = None
    if present(operation):
        files.identity(operation)
        value = gate_files.read(operation / "journal.json")
        orientations = gate_files.validate_record(value, root, operation, target,
            expected_target_sha256, lock, unit_directory, UNITS)
        targets.attest_originals(value["units"], target)
        if not marked and value["state"] != "prepared":
            raise ValueError("controller gate journal lost its active operation marker")
    else:
        if marked:
            raise ValueError("controller gate active operation lacks its journal")
        originals = _originals(unit_directory, target)
        orientations = dict.fromkeys(UNITS, "original")
    _observe(target, environment, deadline, orientations)

    def recheck():
        _remaining(deadline)
        _live(root, target_path, expected_target_sha256, expected_root, lock, target_context)
        if files.located(unit_directory) != unit_identity:
            raise ValueError("controller gate unit directory changed")
        _remaining(deadline)

    recheck()
    if value is None:
        value = _prepare(root, operation, target, expected_target_sha256, lock,
                         unit_directory, originals, recheck)
    if prepare_only:
        return value
    if not marked:
        gate_files.save(marker, active, sync_directory)
    gate_files.sync_file(marker)
    sync_directory(operation.parent)

    def orientation():
        recheck()
        if not _active(marker, active) or gate_files.read(operation / "journal.json") != value:
            raise ValueError("controller gate durable intention changed")
        observed = gate_files.validate_record(value, root, operation, target,
            expected_target_sha256, lock, unit_directory, UNITS)
        targets.attest_originals(value["units"], target)
        return observed

    orientations = orientation()
    if "original" in orientations.values():
        value = _state(operation, value, "masking")
        for unit in UNITS:
            if orientation()[unit] == "original":
                _exchange(unit_directory / unit, operation / "slots" / unit)
            sync_directory(unit_directory)
            sync_directory(operation / "slots")
            orientation()
    sync_directory(unit_directory)
    sync_directory(operation / "slots")
    if set(orientation().values()) != {"masked"}:
        raise ValueError("controller gate still contains an open entry")
    if value["state"] != "gated":
        value = _state(operation, value, "reload_pending")
    orientation()
    _command(target, environment, deadline, "daemon-reload")
    _observe(target, environment, deadline, orientation(), complete=True)
    orientation()
    value = _state(operation, value, "gated")
    orientation()
    _remaining(deadline)
    return {**active, "state": "gated"}


def close_entries(root, operation_id, *, target_path, expected_target_sha256, timeout=30):
    """Establish persistent masks and fresh manager evidence under one local lock."""
    try:
        if type(timeout) not in (int, float) or not 1 <= timeout <= 60 or not math.isfinite(timeout):
            raise ValueError("controller entry gate timeout is invalid")
        deadline = monotonic() + timeout
        operation_id = _identifier(operation_id)
        hexadecimal(expected_target_sha256, 64)
        expected_root = files.located(root)
        expected_lock = files.identity(root / "deploy.lock", directory=False)
        with journal.locked(root):
            if files.located(root) != expected_root:
                raise ValueError("controller entry gate root changed before locking")
            files.unchanged(root / "deploy.lock", expected_lock, directory=False)
            return _close_locked(root, operation_id, target_path=target_path,
                                 expected_target_sha256=expected_target_sha256, deadline=deadline)
    except (OSError, ValueError, RuntimeError, KeyError, TypeError, UnicodeError, RecursionError):
        raise RuntimeError(ERROR) from None
