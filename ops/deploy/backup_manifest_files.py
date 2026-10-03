"""Bounded stable file reads for private backup capture metadata."""
import hashlib
import os
from pathlib import Path
import stat


def present(path):
    return os.path.lexists(path)


def managed(root, backup):
    if (not isinstance(root, Path) or not isinstance(backup, Path)
            or root.is_symlink() or (root / "backups").is_symlink()
            or backup.is_symlink() or backup.parent != root / "backups"
            or backup.resolve(strict=True).parent != root.resolve(strict=True) / "backups"):
        raise ValueError("backup must be a direct managed directory")
    information = backup.stat()
    if not stat.S_ISDIR(information.st_mode) or stat.S_IMODE(information.st_mode) != 0o700:
        raise ValueError("backup directory must remain private")


def identity(information):
    return (information.st_dev, information.st_ino, information.st_mode,
            information.st_size, information.st_mtime_ns, information.st_ctime_ns)


def inspect(path, maximum, *, private=False, contents=False):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        before = os.fstat(stream.fileno())
        if (not stat.S_ISREG(before.st_mode) or not 0 < before.st_size <= maximum
                or (private and stat.S_IMODE(before.st_mode) != 0o600)):
            raise ValueError("backup source file exceeds its type, size or permission boundary")
        digest = hashlib.sha256()
        blocks = []
        amount = 0
        while block := stream.read(min(1024 * 1024, maximum - amount + 1)):
            amount += len(block)
            if amount > maximum:
                raise ValueError("backup source grew beyond its size boundary")
            digest.update(block)
            if contents:
                blocks.append(block)
        after = os.fstat(stream.fileno())
        if (identity(before) != identity(after) or amount != before.st_size
                or identity(after) != identity(path.lstat())):
            raise ValueError("backup source changed while its integrity was checked")
    return {"bytes": amount, "sha256": digest.hexdigest()}, b"".join(blocks)


def sync_directory(path):
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def discard_temporary(path, owned):
    try:
        actual = path.lstat()
        if (actual.st_dev, actual.st_ino) == owned:
            path.unlink()
    except FileNotFoundError:
        pass
