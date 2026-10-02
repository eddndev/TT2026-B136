"""Bind CRL maintenance recovery to its exact completed private backup."""
import hashlib
import os
from pathlib import Path
import stat


FILES = ("database.dump", "redis.rdb", "private-state.tar.gz", "COMPLETE")


def fingerprint(path):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        before = os.fstat(stream.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_size == 0:
            raise ValueError("maintenance backup payload must be a nonempty regular file")
        if before.st_mode & 0o777 != 0o600:
            raise ValueError("maintenance backup payload must remain private")
        digest = hashlib.sha256()
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
        after = os.fstat(stream.fileno())
        if (before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (
                after.st_size, after.st_mtime_ns, after.st_ctime_ns):
            raise ValueError("maintenance backup changed while its integrity was checked")
    return digest.hexdigest()


def capture(root, backup):
    if (not isinstance(backup, Path) or backup.is_symlink()
            or (root / "backups").is_symlink()
            or backup.resolve(strict=True).parent != root / "backups"):
        raise ValueError("maintenance backup must be a local managed directory")
    if not backup.is_dir() or backup.stat().st_mode & 0o777 != 0o700:
        raise ValueError("maintenance backup directory must remain private")
    return {"path": str(backup), "files": {name: fingerprint(backup / name) for name in FILES}}


def validate(root, manifest):
    if (not isinstance(manifest, dict) or set(manifest) != {"path", "files"}
            or not isinstance(manifest["path"], str)):
        raise ValueError("maintenance backup manifest is incomplete")
    actual = capture(root, Path(manifest["path"]))
    if actual != manifest:
        raise ValueError("maintenance backup differs from its recorded integrity manifest")
