"""Durable admission closure for normal services during pending restoration."""
import json
from contextlib import contextmanager
from pathlib import Path
import stat
import sys
import uuid

import crl_journal as journal


MAXIMUM_RECORD = 4096
BLOCKED = "restore maintenance is pending; normal admission remains closed"


def fence_path(root):
    return root / "maintenance/restore/active.json"


def _directories_exist(root):
    for path in (root, root / "maintenance", root / "maintenance/restore"):
        try:
            info = path.lstat()
        except FileNotFoundError:
            return False
        if not stat.S_ISDIR(info.st_mode):
            raise ValueError("restore maintenance path must be a directory without symlinks")
    return True


def guard(root):
    try:
        if not _directories_exist(root):
            return
        try:
            fence_path(root).lstat()
        except FileNotFoundError:
            return
    except (OSError, ValueError):
        raise RuntimeError(BLOCKED) from None
    raise RuntimeError(BLOCKED)


def _identifier(value):
    try:
        parsed = uuid.UUID(value)
        if parsed.int == 0 or str(parsed) != value:
            raise ValueError("invalid identity")
    except (TypeError, ValueError, AttributeError):
        raise ValueError("restore operation identity must be a canonical non-nil UUID") from None
    return value


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("restore marker contains duplicate fields")
        result[key] = value
    return result


def _record(path, info):
    if (not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) != 0o600
            or not 0 < info.st_size <= MAXIMUM_RECORD):
        raise ValueError("restore marker must be a bounded private regular file")
    raw = journal.read(path, MAXIMUM_RECORD)
    if not raw.isascii():
        raise ValueError("restore marker must be ASCII JSON")
    record = json.loads(raw, object_pairs_hook=_unique_object)
    if not isinstance(record, dict) or set(record) != {"operation_id"}:
        raise ValueError("restore marker fields differ")
    _identifier(record["operation_id"])
    return record


def _enter_locked(root, operation_id):
    operation_id = _identifier(operation_id)
    _directories_exist(root)
    if journal.pending(root):
        raise RuntimeError("CRL maintenance must finish before restore admission closes")
    path = fence_path(root)
    try:
        info = path.lstat()
    except FileNotFoundError:
        info = None
    if info is not None:
        record = _record(path, info)
        if record["operation_id"] != operation_id:
            raise ValueError("another restore operation holds admission closed")
        for directory in (root / "maintenance", path.parent):
            if stat.S_IMODE(directory.lstat().st_mode) != 0o700:
                raise ValueError("restore maintenance directories must remain private")
        # A prior rename may have succeeded while its directory fsync failed.
        journal.sync_directory(path.parent)
        return record
    for directory in (root / "maintenance", path.parent):
        journal.directory(directory)
    record = {"operation_id": operation_id}
    journal.save(path, record)
    return record


@contextmanager
def maintenance(root, operation_id):
    """Retain the existing deployment lock while a caller closes owned services."""
    _identifier(operation_id)
    _directories_exist(root)
    with journal.locked(root):
        yield lambda: _enter_locked(root, operation_id)


def enter(root, operation_id):
    with maintenance(root, operation_id) as close_admission:
        return close_admission()


if __name__ == "__main__":
    try:
        if len(sys.argv) != 2:
            raise ValueError("only the deployment root is accepted")
        guard(Path(sys.argv[1]))
    except Exception:
        print(BLOCKED, file=sys.stderr)
        sys.exit(1)
