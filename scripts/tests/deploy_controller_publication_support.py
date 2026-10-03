"""Private source-only fixtures for atomic controller directory publication."""
from contextlib import contextmanager, ExitStack
import ctypes
import errno
import hashlib
import importlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
from unittest.mock import patch

REPOSITORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPOSITORY / "ops/deploy"))
import crl_journal as journal
import host

OPERATION = "10000000-0000-4000-8000-000000000001"
OTHER_OPERATION = "20000000-0000-4000-8000-000000000002"
REJECTIONS = (ValueError, RuntimeError, OSError)


def module():
    return importlib.import_module("controller_publication")


def encoded(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("ascii")


def digest(value):
    return hashlib.sha256(value).hexdigest()


def identity(path):
    row = path.lstat()
    return row.st_dev, row.st_ino


def put(path, value):
    missing = []
    parent = path.parent
    while not parent.exists():
        missing.append(parent)
        parent = parent.parent
    for parent in reversed(missing):
        parent.mkdir(mode=0o700)
    path.write_bytes(value)
    path.chmod(0o600)


def inventory(directory):
    return {path.name: {"bytes": len(path.read_bytes()), "sha256": digest(path.read_bytes())}
            for path in sorted(directory.iterdir())}


def descriptor_inventory(descriptor):
    result = {}
    for name in sorted(os.listdir(descriptor)):
        file = os.open(name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=descriptor)
        with os.fdopen(file, "rb") as stream:
            value = stream.read()
        result[name] = {"bytes": len(value), "sha256": digest(value)}
    return result


def tree(directory):
    result = {}
    for path in [directory, *sorted(directory.rglob("*"))]:
        row = path.lstat()
        content = (os.readlink(path) if stat.S_ISLNK(row.st_mode) else
                   path.read_bytes() if stat.S_ISREG(row.st_mode) else None)
        result[str(path.relative_to(directory))] = (
            row.st_dev, row.st_ino, row.st_uid, row.st_mode, row.st_mtime_ns, content)
    return result


@contextmanager
def no_operational_effects():
    with ExitStack() as stack:
        for owner, name in ((host, "prepare"), (host, "start_databases"),
                            (subprocess, "Popen"), (os, "system"), (os, "execve")):
            stack.enter_context(patch.object(
                owner, name, side_effect=AssertionError("source publication reached an operational edge")))
        yield


class Fixture:
    def __init__(self, case):
        temporary = tempfile.TemporaryDirectory()
        case.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.base.chmod(0o700)
        self.root = self.base / "deployment"
        self.root.mkdir(mode=0o700)
        self.tools = self.root / "tools"
        self.tools.mkdir(mode=0o700)
        put(self.tools / "alpha.py", b"VALUE = 'old'\n")
        put(self.tools / "retired.py", b"VALUE = 'retained history'\n")
        self.old = inventory(self.tools)
        self.old_tree = tree(self.tools)
        self.previous = digest(encoded(self.old))
        self.stage = self.base / "staged"
        self.stage.mkdir(mode=0o700)
        self.sources = self.stage / "sources"
        self.sources.mkdir(mode=0o700)
        put(self.sources / "alpha.py", b"VALUE = 'new'\n")
        put(self.sources / "omega.py", b"VALUE = 'complete candidate'\n")
        self.new = inventory(self.sources)
        self.manifest = {"format": "qadra-controller-publication", "version": 1,
                         "source_revision": "a" * 40, "files": self.new}
        self.save_manifest()
        put(self.root / "deploy.lock", b"")
        for name in ("config/settings.json", "config/nginx.conf", "config/redis.conf",
                     "units/qadra-api.service", "data/.env", "data/postgres/PG_VERSION",
                     "data/redis/dump.rdb", "data/pki/private/synthetic.key",
                     "backups/retained/COMPLETE", "maintenance/crl/retained.json",
                     "maintenance/restore/active.json", "releases/old/release.json",
                     "releases/current/release.json"):
            put(self.root / name, ("synthetic preservation fixture: " + name + "\n").encode())
        (self.root / "current").symlink_to("releases/current")
        (self.root / "previous").symlink_to("releases/old")
        self.operation = self.root / "maintenance/controllers" / OPERATION
        self.exchange = self.operation / "exchange"
        self.record = self.operation / "journal.json"
        self.protected = self.protected_snapshot()

    def save_manifest(self, raw=None):
        value = encoded(self.manifest) if raw is None else raw
        put(self.stage / "manifest.json", value)
        self.manifest_hash = digest(value)

    def protected_snapshot(self):
        result = {name: tree(self.root / name)
                  for name in ("config", "units", "data", "backups", "releases")}
        for name in ("current", "previous", "deploy.lock", "maintenance/crl/retained.json",
                     "maintenance/restore/active.json"):
            path = self.root / name
            row = path.lstat()
            value = os.readlink(path) if path.is_symlink() else path.read_bytes()
            result[name] = (identity(path), row.st_mode, row.st_uid, row.st_mtime_ns, value)
        return result

    def publish(self, publisher, **overrides):
        values = {"root": self.root, "operation_id": OPERATION,
                  "staged_directory": self.stage,
                  "expected_manifest_sha256": self.manifest_hash,
                  "expected_previous_sha256": self.previous}
        values.update(overrides)
        with no_operational_effects():
            return publisher.publish_controllers(**values)

    def assert_preserved(self, case):
        case.assertEqual(self.protected_snapshot(), self.protected)

    def assert_published(self, case, receipt):
        case.assertEqual(inventory(self.tools), self.new)
        case.assertEqual(tree(self.exchange), self.old_tree)
        case.assertEqual(receipt, {
            "operation_id": OPERATION, "manifest_sha256": self.manifest_hash,
            "previous_sha256": self.previous, "installed_sha256": digest(encoded(self.new)),
            "previous_path": str(self.exchange),
        })
        record = json.loads(self.record.read_bytes())
        case.assertEqual(record["state"], "published")
        case.assertEqual(record["operation_id"], OPERATION)
        case.assertEqual(record["manifest_sha256"], self.manifest_hash)
        row = self.root.stat()
        case.assertEqual(record["root"], {"path": str(self.root), "uid": row.st_uid,
                                         "device": row.st_dev, "inode": row.st_ino})
        self.assert_preserved(case)


def exchange_fixture_directories(left, right):
    """Use one Linux syscall only; this is not a production publication helper."""
    library = ctypes.CDLL(None, use_errno=True)
    try:
        exchange = library.renameat2
    except AttributeError:
        raise OSError(errno.ENOSYS, "renameat2 is unavailable") from None
    exchange.argtypes = (ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p,
                         ctypes.c_uint)
    exchange.restype = ctypes.c_int
    if exchange(-100, os.fsencode(left), -100, os.fsencode(right), 2):
        code = ctypes.get_errno()
        raise OSError(code, os.strerror(code))
