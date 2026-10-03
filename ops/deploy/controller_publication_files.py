"""Private bounded source snapshots and durable controller publication files."""
import hashlib
import json
import os
from pathlib import Path
import stat
import uuid

from backup_manifest_files import discard_temporary, inspect, sync_directory
from backup_manifest_schema import (
    MAX_CONTROLLER_FILE, MAX_CONTROLLER_FILES, MAX_CONTROLLER_TOTAL, MAX_MANIFEST,
    PYTHON_NAME, decode, hexadecimal, keys, record,
)


def encoded(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("ascii")


def digest(value):
    return hashlib.sha256(value).hexdigest()


def exact_path(path):
    if (not isinstance(path, Path) or not path.is_absolute()
            or path.resolve(strict=True) != path
            or any(parent.is_symlink() for parent in (path, *path.parents))):
        raise ValueError("controller publication requires an exact local path")
    return path


def identity(path, *, directory=True):
    exact_path(path)
    info = path.lstat()
    valid_type = stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode)
    if (not valid_type or info.st_uid != os.getuid()
            or stat.S_IMODE(info.st_mode) != (0o700 if directory else 0o600)
            or (not directory and info.st_nlink != 1)):
        raise ValueError("controller publication ownership, type or permissions differ")
    return {"uid": info.st_uid, "device": info.st_dev, "inode": info.st_ino}


def located(path):
    return {"path": str(path), **identity(path)}


def unchanged(path, expected, *, directory=True):
    if identity(path, directory=directory) != expected:
        raise ValueError("controller publication path identity changed")


def read(path, maximum):
    before = identity(path, directory=False)
    observed, raw = inspect(path, maximum, private=True, contents=True)
    unchanged(path, before, directory=False)
    return observed, raw


def inventory_shape(files):
    if type(files) is not dict or not 1 <= len(files) <= MAX_CONTROLLER_FILES:
        raise ValueError("controller source inventory is empty or oversized")
    for name, value in files.items():
        if not isinstance(name, str) or not PYTHON_NAME.fullmatch(name):
            raise ValueError("controller source inventory requires Python basenames")
        record(value, MAX_CONTROLLER_FILE)
    if sum(value["bytes"] for value in files.values()) > MAX_CONTROLLER_TOTAL:
        raise ValueError("controller source inventory exceeds its byte boundary")


def snapshot(path, *, contents=False):
    owned = identity(path)
    before = path.stat()
    names = []
    with os.scandir(path) as entries:
        for entry in entries:
            if len(names) >= MAX_CONTROLLER_FILES or not PYTHON_NAME.fullmatch(entry.name):
                raise ValueError("controller directory contains unapproved entries")
            names.append(entry.name)
    files, sources = {}, {}
    total = 0
    for name in sorted(names):
        observed, raw = read(path / name, MAX_CONTROLLER_FILE)
        total += observed["bytes"]
        if total > MAX_CONTROLLER_TOTAL:
            raise ValueError("controller directory exceeds its byte boundary")
        files[name] = observed
        if contents:
            sources[name] = raw
    inventory_shape(files)
    unchanged(path, owned)
    after = path.stat()
    if (before.st_mtime_ns, before.st_ctime_ns) != (after.st_mtime_ns, after.st_ctime_ns):
        raise ValueError("controller directory changed during inspection")
    return {"identity": owned, "files": files}, sources


def package(path, expected):
    owned = located(path)
    with os.scandir(path) as entries:
        names = []
        for entry in entries:
            names.append(entry.name)
            if len(names) > 2:
                raise ValueError("controller staging contains unapproved entries")
    if set(names) != {"manifest.json", "sources"}:
        raise ValueError("controller staging is incomplete")
    observed, raw = read(path / "manifest.json", MAX_MANIFEST)
    if observed["sha256"] != expected or not raw.isascii():
        raise ValueError("controller manifest differs from its approved digest")
    value = decode(raw)
    keys(value, ("format", "version", "source_revision", "files"))
    if (value["format"] != "qadra-controller-publication"
            or type(value["version"]) is not int or value["version"] != 1):
        raise ValueError("controller manifest format is unsupported")
    hexadecimal(value["source_revision"], 40)
    inventory_shape(value["files"])
    sources, contents = snapshot(path / "sources", contents=True)
    if sources["files"] != value["files"] or located(path) != owned:
        raise ValueError("controller source package differs from its manifest")
    return value, owned, sources["identity"], contents


def ensure_directory(path):
    try:
        identity(path)
    except FileNotFoundError:
        path.mkdir(mode=0o700)
        sync_directory(path.parent)
        identity(path)


def write_new(path, raw):
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())


def copy_sources(path, contents):
    parent = identity(path.parent)
    path.mkdir(mode=0o700)
    owned = identity(path)
    for name, raw in contents.items():
        unchanged(path.parent, parent)
        unchanged(path, owned)
        write_new(path / name, raw)
    unchanged(path.parent, parent)
    unchanged(path, owned)
    sync_directory(path)
    sync_directory(path.parent)
    return owned


def write_record(path, value):
    raw = encoded(value)
    if len(raw) > MAX_MANIFEST:
        raise ValueError("controller publication record exceeds its byte boundary")
    parent = identity(path.parent)
    temporary = path.parent / (".journal." + uuid.uuid4().hex)
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    info = os.fstat(descriptor)
    owned = info.st_dev, info.st_ino
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        unchanged(path.parent, parent)
        os.replace(temporary, path)
        unchanged(path.parent, parent)
        sync_directory(path.parent)
    finally:
        discard_temporary(temporary, owned)


def load_record(path):
    _, raw = read(path, MAX_MANIFEST)
    if not raw.isascii():
        raise ValueError("controller publication record must be ASCII")
    return decode(raw)
