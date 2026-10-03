"""Verify an installed immutable release without extracting or activating it."""
import hashlib
import os
from pathlib import Path, PurePosixPath
import stat

from backup_manifest_files import identity, inspect
import backup_manifest_schema as capture
from bundle import MAX_BYTES
from release import linked


MAX_MEMBERS = 20000
REQUIRED = {"bin/despacho-cli", "web/index.html", "lib/libqpdf.so.30.4.1",
            "pki/tsa.cnf", "bin/ffmpeg", "bin/ffprobe"}


def member_name(name):
    if not isinstance(name, str) or not name.isascii() or len(name) > 240:
        raise ValueError("installed release member name exceeds its boundary")
    path = PurePosixPath(name)
    if (path.is_absolute() or ".." in path.parts or not path.parts
            or "\\" in name or ":" in name or path.as_posix() != name
            or any(ord(character) < 32 for character in name)):
        raise ValueError("installed release member name is unsafe")
    return path


def inventory(target, names):
    directories = {PurePosixPath(".")}
    for name in names:
        directories.update(member_name(name).parents)
    pending = [target]
    files = {}
    total = 0
    while pending:
        directory = pending.pop()
        with os.scandir(directory) as entries:
            for entry in entries:
                path = Path(entry.path)
                relative = PurePosixPath(path.relative_to(target).as_posix())
                information = entry.stat(follow_symlinks=False)
                if stat.S_ISDIR(information.st_mode):
                    if relative not in directories:
                        raise ValueError("installed release has an unrecorded directory")
                    pending.append(path)
                elif stat.S_ISREG(information.st_mode):
                    name = relative.as_posix()
                    if name not in names or len(files) >= MAX_MEMBERS:
                        raise ValueError("installed release file inventory differs")
                    total += information.st_size
                    if total > MAX_BYTES:
                        raise ValueError("installed release exceeds its total byte boundary")
                    files[name] = information
                else:
                    raise ValueError("installed release contains a link or nonregular member")
    if set(files) != names:
        raise ValueError("installed release file inventory is incomplete")
    return files


def empty_digest(path, before):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        current = os.fstat(descriptor)
        if (not stat.S_ISREG(current.st_mode) or current.st_size != 0
                or identity(current) != identity(before)
                or identity(current) != identity(path.lstat())):
            raise ValueError("empty installed release member changed while checked")
    finally:
        os.close(descriptor)
    return hashlib.sha256(b"").hexdigest()


def schema_digest(target, files):
    # Preserve bundle.schema_digest's filename/NUL/bytes sequence using bounded reads.
    hasher = hashlib.sha256()
    names = sorted(name for name in files if PurePosixPath(name).parent == PurePosixPath("migrations")
                   and name.endswith(".sql"))
    for name in names:
        path = target / name
        hasher.update(path.name.encode("ascii") + b"\0")
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(descriptor, "rb") as stream:
            before = os.fstat(stream.fileno())
            if not stat.S_ISREG(before.st_mode) or identity(before) != identity(files[name]):
                raise ValueError("installed migration changed while checked")
            amount = 0
            while block := stream.read(min(1024 * 1024, before.st_size - amount + 1)):
                amount += len(block)
                if amount > before.st_size:
                    raise ValueError("installed migration grew while checked")
                hasher.update(block)
            after = os.fstat(stream.fileno())
            if (amount != before.st_size or identity(before) != identity(after)
                    or identity(after) != identity(path.lstat())):
                raise ValueError("installed migration changed while checked")
    return hasher.hexdigest()


def verify(root, expected):
    target = linked(root, "current")
    if target is None or not stat.S_ISDIR(target.lstat().st_mode):
        raise ValueError("restore compatibility requires a managed installed release")
    information, raw = inspect(target / "release.json", capture.MAX_RELEASE, contents=True)
    document = capture.decode(raw)
    capture.keys(document, ("version", "commit", "schema", "files"))
    actual = {key: document[key] for key in ("version", "commit", "schema")}
    actual["manifest_sha256"] = information["sha256"]
    if actual != expected:
        raise ValueError("installed release identity differs from the capture")
    declared = document["files"]
    if (type(declared) is not dict or not 1 <= len(declared) < MAX_MEMBERS
            or not REQUIRED.issubset(declared) or "release.json" in declared):
        raise ValueError("installed release declared inventory is incomplete or oversized")
    for name, digest in declared.items():
        member_name(name)
        capture.hexadecimal(digest, 64)
    files = inventory(target, {*declared, "release.json"})
    for name, expected_digest in declared.items():
        path = target / name
        if files[name].st_size == 0:
            actual_digest = empty_digest(path, files[name])
        else:
            record, _ = inspect(path, MAX_BYTES)
            if identity(path.lstat()) != identity(files[name]):
                raise ValueError("installed release member changed while checked")
            actual_digest = record["sha256"]
        if actual_digest != expected_digest:
            raise ValueError("installed release member checksum differs")
    if schema_digest(target, files) != document["schema"]:
        raise ValueError("installed migration schema differs from the declared release")
