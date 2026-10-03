"""Run one approved controller generation in a fresh isolated Python process."""
import argparse
from contextlib import contextmanager
import hashlib
import importlib
import json
import os
from pathlib import Path
# runpy imports pkgutil lazily; load it before exposing any controller path.
import pkgutil
import re
import runpy
import stat
import sys


ENTRYPOINTS = ("runtime.py", "release.py", "restore_fence.py", "renew_crl.py", "controller_installation.py")
PYTHON_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*\.py")
MAX_FILES = 64
MAX_FILE_BYTES = 256 * 1024
MAX_TOTAL_BYTES = 8 * 1024 * 1024
DIRECTORY_FLAGS = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
SOURCE_FLAGS = os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK


def identity(row):
    return (row.st_dev, row.st_ino, row.st_uid, row.st_gid, row.st_mode,
            row.st_nlink, row.st_size, row.st_mtime_ns, row.st_ctime_ns)


def private_directory(row, uid):
    if (not stat.S_ISDIR(row.st_mode) or row.st_uid != uid
            or stat.S_IMODE(row.st_mode) != 0o700):
        raise ValueError("controller directory ownership or permissions differ")


def private_source(row, uid):
    if (not stat.S_ISREG(row.st_mode) or row.st_uid != uid or row.st_nlink != 1
            or stat.S_IMODE(row.st_mode) != 0o600
            or not 1 <= row.st_size <= MAX_FILE_BYTES):
        raise ValueError("controller source type, ownership or boundary differs")


@contextmanager
def root_directory(root, uid):
    if (not isinstance(root, Path) or root.anchor != "/"
            or any(part in (".", "..") for part in root.parts[1:])):
        raise ValueError("controller root must be an exact absolute local path")
    descriptor = os.open("/", DIRECTORY_FLAGS)
    try:
        for component in root.parts[1:]:
            child = os.open(component, DIRECTORY_FLAGS, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = child
        private_directory(os.fstat(descriptor), uid)
        yield descriptor
    finally:
        os.close(descriptor)


@contextmanager
def generation_directory(root_descriptor, uid):
    descriptor = os.open("tools", DIRECTORY_FLAGS, dir_fd=root_descriptor)
    try:
        private_directory(os.fstat(descriptor), uid)
        if os.get_inheritable(descriptor):
            raise ValueError("controller generation descriptor must be close-on-exec")
        yield descriptor
    finally:
        os.close(descriptor)


def source_names(descriptor):
    names = []
    scan_descriptor = os.open(".", DIRECTORY_FLAGS, dir_fd=descriptor)
    try:
        with os.scandir(scan_descriptor) as entries:
            for entry in entries:
                if len(names) >= MAX_FILES or not PYTHON_NAME.fullmatch(entry.name):
                    raise ValueError("controller generation contains unapproved entries")
                names.append(entry.name)
    finally:
        os.close(scan_descriptor)
    if not names:
        raise ValueError("controller generation is empty")
    return sorted(names)


def source_record(directory, name, uid):
    before = os.stat(name, dir_fd=directory, follow_symlinks=False)
    private_source(before, uid)
    descriptor = os.open(name, SOURCE_FLAGS, dir_fd=directory)
    try:
        opened = os.fstat(descriptor)
        private_source(opened, uid)
        if identity(opened) != identity(before):
            raise ValueError("controller source changed while opening")
        value = bytearray()
        while len(value) <= MAX_FILE_BYTES:
            chunk = os.read(descriptor, min(65536, MAX_FILE_BYTES + 1 - len(value)))
            if not chunk:
                break
            value.extend(chunk)
        after = os.stat(name, dir_fd=directory, follow_symlinks=False)
        if (len(value) != before.st_size or identity(os.fstat(descriptor)) != identity(before)
                or identity(after) != identity(before)):
            raise ValueError("controller source changed during inspection")
        return {"bytes": len(value), "sha256": hashlib.sha256(value).hexdigest()}
    finally:
        os.close(descriptor)


def admitted_inventory(descriptor, uid, expected):
    before = os.fstat(descriptor)
    names = source_names(descriptor)
    files, total = {}, 0
    for name in names:
        record = source_record(descriptor, name, uid)
        total += record["bytes"]
        if total > MAX_TOTAL_BYTES:
            raise ValueError("controller generation exceeds its total byte boundary")
        files[name] = record
    if identity(os.fstat(descriptor)) != identity(before) or source_names(descriptor) != names:
        raise ValueError("controller generation changed during inspection")
    encoded = (json.dumps(files, sort_keys=True, separators=(",", ":")) + "\n").encode("ascii")
    if hashlib.sha256(encoded).hexdigest() != expected:
        raise ValueError("controller generation differs from the approved inventory")
    return files


def interpreter_paths(root, names):
    if not sys.flags.isolated or not sys.flags.no_site:
        raise ValueError("controller launcher requires an isolated interpreter without site imports")
    if any(name[:-3] in sys.modules for name in names):
        raise ValueError("controller modules were imported before generation admission")
    excluded = {root / "tools", Path.cwd()}
    result = []
    for item in sys.path:
        if not isinstance(item, str) or not item or not Path(item).is_absolute():
            continue
        if Path(item).resolve() not in excluded:
            result.append(item)
    return result


def execute(descriptor, entrypoint, arguments, library_paths):
    directory = f"/proc/self/fd/{descriptor}"
    pinned = os.stat(directory)
    opened = os.fstat(descriptor)
    if (pinned.st_dev, pinned.st_ino) != (opened.st_dev, opened.st_ino):
        raise ValueError("controller descriptor pathname does not identify its generation")
    filename = directory + "/" + entrypoint
    previous_arguments, previous_paths = sys.argv, sys.path
    previous_bytecode = sys.dont_write_bytecode
    try:
        sys.dont_write_bytecode = True
        sys.path = [directory, *library_paths]
        sys.argv = [filename, *arguments]
        importlib.invalidate_caches()
        runpy.run_path(filename, run_name="__main__")
    finally:
        sys.argv, sys.path = previous_arguments, previous_paths
        sys.dont_write_bytecode = previous_bytecode
        sys.path_importer_cache.pop(directory, None)


def launch(root, entrypoint, arguments, *, expected_inventory_sha256):
    """Pin and validate all sources before executing an approved entrypoint."""
    if (not isinstance(expected_inventory_sha256, str)
            or not re.fullmatch(r"[0-9a-f]{64}", expected_inventory_sha256)):
        raise ValueError("controller inventory digest must be canonical SHA-256")
    if entrypoint not in ENTRYPOINTS:
        raise ValueError("controller entrypoint is not approved")
    if type(arguments) is not list or any(not isinstance(value, str) for value in arguments):
        raise ValueError("controller arguments must be an explicit string list")
    uid, gid = os.getuid(), os.getgid()
    if uid != os.geteuid() or gid != os.getegid():
        raise ValueError("controller launcher cannot cross a process identity boundary")
    with root_directory(root, uid) as root_descriptor:
        with generation_directory(root_descriptor, uid) as descriptor:
            files = admitted_inventory(descriptor, uid, expected_inventory_sha256)
            if entrypoint not in files:
                raise ValueError("controller entrypoint is missing from its approved generation")
            paths = interpreter_paths(root, files)
            if (os.getuid(), os.geteuid(), os.getgid(), os.getegid()) != (uid, uid, gid, gid):
                raise ValueError("controller process identity changed during admission")
            execute(descriptor, entrypoint, arguments, paths)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--inventory-sha256", required=True)
    parser.add_argument("--entrypoint", choices=ENTRYPOINTS, required=True)
    parser.add_argument("arguments", nargs=argparse.REMAINDER)
    options = parser.parse_args()
    if not options.arguments or options.arguments[0] != "--":
        parser.error("controller arguments must follow --")
    launch(options.root, options.entrypoint, options.arguments[1:],
           expected_inventory_sha256=options.inventory_sha256)


if __name__ == "__main__":
    try:
        main()
    except Exception:
        print("Controller launch failed; inspect its approved generation", file=sys.stderr)
        sys.exit(1)
