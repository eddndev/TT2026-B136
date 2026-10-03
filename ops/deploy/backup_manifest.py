"""Publish capture hashes and declared local identity; do not admit a restore."""
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import uuid

import backup_manifest_schema as schema
from backup_manifest_files import discard_temporary, inspect, managed, present, sync_directory

NAME = schema.NAME


def controller_inventory(directory):
    if not isinstance(directory, Path) or directory.is_symlink() or not directory.is_dir():
        raise ValueError("backup controller must be a regular source directory")
    paths = []
    for path in directory.iterdir():
        if path.name.endswith(".py"):
            if not schema.PYTHON_NAME.fullmatch(path.name):
                raise ValueError("backup controller contains an invalid source name")
            paths.append(path)
            if len(paths) > schema.MAX_CONTROLLER_FILES:
                raise ValueError("backup controller source count exceeds its boundary")
    result = {}
    total = 0
    for path in sorted(paths):
        information, _ = inspect(path, schema.MAX_CONTROLLER_FILE)
        total += information["bytes"]
        if total > schema.MAX_CONTROLLER_TOTAL:
            raise ValueError("backup controller bytes exceed their boundary")
        result[path.name] = information
    return result


def capture_source(root, controller_directory):
    source = {"initialized": False, "schema": None, "release": None,
              "controller": controller_inventory(controller_directory)}
    marker = root / "config/schema"
    current = root / "current"
    if (root / "config").is_symlink() or (root / "releases").is_symlink():
        raise ValueError("backup source directories cannot be redirected")
    if not present(marker) and not present(current):
        return source
    if not present(marker) or not current.is_symlink():
        raise ValueError("backup initialization and active release are inconsistent")
    target = current.resolve(strict=True)
    if not target.is_dir() or target.parent != root.resolve(strict=True) / "releases":
        raise ValueError("backup active release escapes its managed directory")
    _, raw = inspect(marker, 65, contents=True)
    try:
        marker_schema = raw.decode("ascii").removesuffix("\n")
    except UnicodeError:
        raise ValueError("backup schema marker is invalid") from None
    schema.hexadecimal(marker_schema, 64)
    information, raw = inspect(target / "release.json", schema.MAX_RELEASE, contents=True)
    release = schema.decode(raw)
    if type(release) is not dict or not {"version", "commit", "schema"}.issubset(release):
        raise ValueError("backup declared release identity is incomplete")
    source.update({"initialized": True, "schema": marker_schema,
                   "release": {**{key: release[key] for key in ("version", "commit", "schema")},
                               "manifest_sha256": information["sha256"]}})
    return source


def publish(root, backup, *, controller_directory):
    managed(root, backup)
    temporary = backup / schema.TEMPORARY
    destination = backup / NAME
    if any(present(path) for path in (temporary, destination, backup / "COMPLETE")):
        raise ValueError("backup metadata publication requires an unpublished capture")
    payloads = {name: inspect(backup / name, schema.MAX_PAYLOAD, private=True)[0]
                for name in schema.PAYLOADS}
    source = capture_source(root, controller_directory)
    document = {"format": "qadra-private-backup", "version": 1,
                "backup_id": str(uuid.uuid4()),
                "captured_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                "source": source, "payloads": payloads}
    schema.check(document)
    encoded = (json.dumps(document, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")
    if len(encoded) > schema.MAX_MANIFEST:
        raise ValueError("backup metadata exceeds its byte boundary")
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    information = os.fstat(descriptor)
    owned = information.st_dev, information.st_ino
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(encoded)
            stream.flush()
            os.fsync(stream.fileno())
        if present(destination) or present(backup / "COMPLETE"):
            raise ValueError("backup metadata was published by another operation")
        temporary.rename(destination)
        sync_directory(backup)
    finally:
        discard_temporary(temporary, owned)
    return document


def validate(root, backup):
    managed(root, backup)
    inspect(backup / "COMPLETE", 4096, private=True)
    _, raw = inspect(backup / NAME, schema.MAX_MANIFEST, private=True, contents=True)
    document = schema.decode(raw)
    schema.check(document)
    for name in schema.PAYLOADS:
        actual, _ = inspect(backup / name, schema.MAX_PAYLOAD, private=True)
        if actual != document["payloads"][name]:
            raise ValueError("backup payload differs from its recorded capture identity")
    return document
