"""Private durable files and an explicit stop fence for CRL maintenance."""
from contextlib import contextmanager
import fcntl
import json
import os
from pathlib import Path
import stat
import uuid

from runtime import sync_directory


def fence_path(root):
    return root / "config/crl-maintenance.json"


def pending(root):
    path = fence_path(root)
    return path.exists() or path.is_symlink()


def guard(root):
    if pending(root):
        raise RuntimeError("CRL maintenance is pending; ingress must remain stopped")


@contextmanager
def locked(root):
    descriptor = os.open(root / "deploy.lock", os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        yield


def directory(path):
    if path.is_symlink():
        raise ValueError("maintenance directory must not be a symlink")
    path.mkdir(mode=0o700, exist_ok=True)
    path.chmod(0o700)
    sync_directory(path.parent)


def read(path, maximum=8 * 1024 * 1024):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(descriptor, "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError("maintenance input must be a regular file")
        value = stream.read(maximum + 1)
    if not value or len(value) > maximum:
        raise ValueError("maintenance input is empty or too large")
    return value


def write(path, value):
    temporary = path.parent / ("." + path.name + "." + uuid.uuid4().hex)
    descriptor = os.open(temporary, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(value)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        sync_directory(path.parent)
    finally:
        temporary.unlink(missing_ok=True)


def save(path, value):
    write(path, (json.dumps(value, sort_keys=True) + "\n").encode("ascii"))


def load(path):
    value = json.loads(read(path))
    if not isinstance(value, dict):
        raise ValueError("maintenance record must be an object")
    return value


def operation(root, identifier):
    try:
        if str(uuid.UUID(identifier)) != identifier:
            raise ValueError("non-canonical operation identity")
    except (TypeError, AttributeError, ValueError) as error:
        raise ValueError("invalid maintenance operation identity") from error
    path = root / "maintenance/crl" / identifier
    if any(parent.is_symlink() for parent in (path, path.parent, path.parent.parent)):
        raise ValueError("maintenance operation must not traverse a symlink")
    return path


def fenced(root, identifier):
    save(fence_path(root), {"operation_id": identifier})


def unfence(root):
    fence_path(root).unlink()
    sync_directory(root / "config")
