"""Pinned installation inputs and exact retained cache and release identities."""
import os
from pathlib import Path
import re

from backup_manifest_files import present
from backup_manifest_schema import decode, hexadecimal, keys
import controller_approval as approval
import controller_gate_files as gate_files
import controller_publication_files as files
import controller_unit_commands as commands
import restore_quiesce_target as targets


UNITS = targets.UNITS
MAXIMUM = 65536


def pinned(path, digest, maximum=MAXIMUM):
    hexadecimal(digest, 64)
    observed, raw = files.read(path, maximum)
    if observed["sha256"] != digest or not raw.isascii():
        raise ValueError("controller installation input differs from its external pin")
    return decode(raw)


def load(root, operation_id, path, digest):
    value = pinned(path, digest)
    keys(value, ("format", "version", "operation_id", "root", "lock", "unit_directory",
                 "target", "previous_sources", "cache", "candidate", "previous_approval_sha256",
                 "launcher", "python", "legacy_absent_prestarts", "release"))
    if (value["format"] != "qadra-controller-installation-intent" or type(value["version"]) is not int
            or value["version"] != 1 or value["operation_id"] != operation_id
            or value["root"] != files.located(root)):
        raise ValueError("controller installation intention belongs to another root or operation")
    approval.approvals.identity(value["root"], located=True)
    gate_files.identity_shape(value["lock"])
    files.unchanged(root / "deploy.lock", value["lock"], directory=False)
    keys(value["unit_directory"], ("path", "uid", "device", "inode"))
    approval.approvals.identity(value["unit_directory"], located=True)
    if value["unit_directory"] != files.located(Path(value["unit_directory"]["path"])):
        raise ValueError("controller installation unit directory changed")
    keys(value["previous_sources"], ("identity", "sha256"))
    gate_files.identity_shape(value["previous_sources"]["identity"])
    hexadecimal(value["previous_sources"]["sha256"], 64)
    keys(value["candidate"], ("path", "manifest_sha256", "installed_sha256"))
    for name in ("manifest_sha256", "installed_sha256"):
        hexadecimal(value["candidate"][name], 64)
    if value["previous_approval_sha256"] is not None:
        hexadecimal(value["previous_approval_sha256"], 64)
    keys(value["launcher"], ("source", "destination"))
    if value["launcher"]["destination"] != str(root / "controller_launcher.py"):
        raise ValueError("controller launcher destination differs")
    targets.material(value["target"], private=True)
    targets.material(value["launcher"]["source"], private=True)
    if Path(value["launcher"]["source"]["path"]).is_relative_to(root / "tools"):
        raise ValueError("controller launcher source overlaps the exchanged generation")
    targets.material(value["python"], executable=True)
    release(root, value)
    return value


def release(root, intent):
    value = intent["release"]
    keys(value, ("link", "metadata"))
    path = root / "current"
    info = path.lstat()
    if (not path.is_symlink() or info.st_uid != os.getuid() or info.st_nlink != 1
            or not isinstance(value["link"], str) or not value["link"].isascii()
            or os.readlink(path) != value["link"]):
        raise ValueError("controller installation release link changed")
    metadata = Path(value["metadata"]["path"])
    if (metadata.name != "release.json" or not metadata.parent.is_relative_to(root / "releases")
            or metadata.parent == root / "releases" or path.resolve(strict=True) != metadata.parent):
        raise ValueError("controller installation release belongs to another tree")
    files.identity(root / "releases")
    files.identity(metadata.parent)
    targets.material(value["metadata"], private=True)
    return metadata.parent


def source_inventory(root, intent):
    path = root / "tools"
    files.unchanged(path, intent["previous_sources"]["identity"])
    observed = {}
    for child in path.iterdir():
        if child.name == "__pycache__" and intent["cache"] is not None:
            continue
        if len(observed) >= 64:
            raise ValueError("controller installation source inventory exceeds its boundary")
        observed[child.name] = files.read(child, 256 * 1024)[0]
    files.inventory_shape(observed)
    if files.digest(files.encoded(observed)) != intent["previous_sources"]["sha256"]:
        raise ValueError("controller installation previous source inventory changed")
    return observed


def cache_at(path, expected):
    if expected is None:
        if present(path):
            raise ValueError("controller installation cache absence changed")
        return
    keys(expected, ("identity", "files"))
    gate_files.identity_shape(expected["identity"])
    files.unchanged(path, expected["identity"])
    entries = expected["files"]
    names = set()
    for child in path.iterdir():
        if len(names) >= 64:
            raise ValueError("controller installation cache inventory exceeds its boundary")
        names.add(child.name)
    if type(entries) is not dict or not 1 <= len(entries) <= 64 or names != set(entries):
        raise ValueError("controller installation cache inventory changed")
    total = 0
    for name, record in entries.items():
        if not isinstance(name, str) or not re.fullmatch(r"[A-Za-z0-9_.-]+\.pyc", name):
            raise ValueError("controller installation cache name is invalid")
        keys(record, ("identity", "bytes", "sha256"))
        gate_files.identity_shape(record["identity"])
        files.unchanged(path / name, record["identity"], directory=False)
        observed, _ = files.read(path / name, 256 * 1024)
        if observed != {key: record[key] for key in ("bytes", "sha256")}:
            raise ValueError("controller installation cache bytes changed")
        total += observed["bytes"]
    if total > 8 * 1024 * 1024:
        raise ValueError("controller installation cache exceeds its boundary")


def retain_cache(root, operation, intent):
    source, destination = root / "tools/__pycache__", operation / "retained-cache"
    expected = intent["cache"]
    if expected is None:
        cache_at(source, None)
        cache_at(destination, None)
        return
    if present(destination):
        cache_at(destination, expected)
        cache_at(source, None)
    else:
        cache_at(source, expected)

        def check():
            cache_at(source, expected)
            cache_at(destination, None)

        gate_files._rename(source, destination, 1, check)
    files.sync_directory(source.parent)
    files.sync_directory(operation)
    cache_at(destination, expected)
    cache_at(source, None)


def originals(root, intent, target, unit_directory, gate_record=None):
    result = {}
    operation = root / "maintenance/controller-installation" / intent["operation_id"]
    for unit in UNITS:
        path = unit_directory / unit
        if gate_record is not None:
            expected = gate_record["units"][unit]["original"]
            slot = operation / "slots" / unit
            path = slot if present(slot) and not slot.is_symlink() else path
            if gate_files.regular(path) != expected:
                raise ValueError("controller installation retained original changed")
        observed, raw = files.read(path, MAXIMUM)
        if observed != {"bytes": target["units"][unit]["fragment"]["size"],
                        "sha256": target["units"][unit]["fragment"]["sha256"]}:
            raise ValueError("controller installation original unit changed")
        lines = raw.splitlines()
        if any(lines.count(line) != 1 for line in (
                b"TimeoutStartSec=240", b"TimeoutStopSec=90", b"KillSignal=SIGINT",
                ("WorkingDirectory=" + str(root / "data")).encode())):
            raise ValueError("controller installation unit policy differs")
        result[unit] = raw
    commands._original_units(root, result, Path(intent["python"]["path"]), intent["legacy_absent_prestarts"])
    return result


def package(root, intent):
    stage = Path(intent["candidate"]["path"])
    if stage.is_relative_to(root / "tools") or stage.is_relative_to(root / "maintenance"):
        raise ValueError("controller installation staging overlaps mutable paths")
    manifest, _, _, _ = files.package(stage, intent["candidate"]["manifest_sha256"])
    if files.digest(files.encoded(manifest["files"])) != intent["candidate"]["installed_sha256"]:
        raise ValueError("controller installation candidate inventory differs")


def launcher(root, intent, *, install=False):
    raw = targets.material(intent["launcher"]["source"], private=True)
    path = root / "controller_launcher.py"
    if present(path):
        if files.read(path, MAXIMUM)[1] != raw:
            raise ValueError("controller installation refuses an unrelated launcher")
    elif install:
        files.write_new(path, raw)
        files.sync_directory(root)
    if install:
        gate_files.sync_file(path)
        files.sync_directory(root)
        return {"identity": files.identity(path, directory=False), **files.read(path, MAXIMUM)[0]}


def predecessor(root, intent):
    current = approval._existing(root / "maintenance/controllers/approved.json", intent["root"])
    approval._predecessor(current, intent["previous_approval_sha256"])
