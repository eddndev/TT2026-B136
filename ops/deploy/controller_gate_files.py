"""Exact private file orientations and atomic publication for controller gates."""
import ctypes
import errno
import os
import stat
import uuid

from backup_manifest_files import discard_temporary, present
from backup_manifest_schema import decode, hexadecimal, keys
import controller_publication_files as files


MAXIMUM = 256 * 1024


def identity_shape(value):
    keys(value, ("uid", "device", "inode"))
    if any(type(number) is not int or number < 0 for number in value.values()):
        raise ValueError("controller gate identity is invalid")


def regular(path):
    owned = files.identity(path, directory=False)
    before = path.lstat()
    fingerprint, _ = files.read(path, 65536)
    after = path.lstat()
    if (before.st_mtime_ns, before.st_ctime_ns) != (after.st_mtime_ns, after.st_ctime_ns):
        raise ValueError("controller gate original changed during capture")
    return {"identity": owned, "mode": stat.S_IMODE(after.st_mode),
            "mtime_ns": after.st_mtime_ns, **fingerprint}


def original_shape(value):
    keys(value, ("identity", "mode", "mtime_ns", "bytes", "sha256"))
    identity_shape(value["identity"])
    if (type(value["mode"]) is not int or value["mode"] != 0o600
            or type(value["mtime_ns"]) is not int
            or type(value["bytes"]) is not int or not 0 < value["bytes"] <= 65536):
        raise ValueError("controller gate original metadata is invalid")
    hexadecimal(value["sha256"], 64)


def mask(path):
    files.exact_path(path.parent)
    before = path.lstat()
    if (not stat.S_ISLNK(before.st_mode) or before.st_uid != os.getuid()
            or before.st_nlink != 1 or os.readlink(path) != "/dev/null"):
        raise ValueError("controller gate mask is not an owned null link")
    after = path.lstat()
    if (before.st_dev, before.st_ino, before.st_ctime_ns) != (after.st_dev, after.st_ino, after.st_ctime_ns):
        raise ValueError("controller gate mask changed during capture")
    return {"uid": after.st_uid, "device": after.st_dev, "inode": after.st_ino}


def orientation(fragment, slot, expected):
    keys(expected, ("original", "mask"))
    original_shape(expected["original"])
    identity_shape(expected["mask"])
    if fragment.is_symlink():
        valid = mask(fragment) == expected["mask"] and regular(slot) == expected["original"]
        result = "masked"
    else:
        valid = regular(fragment) == expected["original"] and mask(slot) == expected["mask"]
        result = "original"
    if not valid:
        raise ValueError("controller gate entry or retained original differs")
    return result


def _parent(path):
    expected = files.identity(path)
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    row = os.fstat(descriptor)
    if (row.st_uid, row.st_dev, row.st_ino) != (expected["uid"], expected["device"], expected["inode"]):
        os.close(descriptor)
        raise ValueError("controller gate parent changed before opening")
    return descriptor, expected


def _rename(source, destination, flags, check):
    library = ctypes.CDLL(None, use_errno=True)
    try:
        rename = library.renameat2
    except AttributeError:
        raise OSError(errno.ENOSYS, "atomic controller gate publication is unavailable") from None
    rename.argtypes = (ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint)
    rename.restype = ctypes.c_int
    opened = []
    try:
        for path in (source.parent, destination.parent):
            opened.append(_parent(path))
        for path, (_, expected) in zip((source.parent, destination.parent), opened):
            files.unchanged(path, expected)
        check()
        if rename(opened[0][0], os.fsencode(source.name), opened[1][0], os.fsencode(destination.name), flags):
            raise OSError(ctypes.get_errno(), "atomic controller gate publication failed")
    finally:
        for descriptor, _ in opened:
            os.close(descriptor)


def exchange(fragment, slot):
    original, owned_mask = regular(fragment), mask(slot)
    if original["identity"]["device"] != owned_mask["device"]:
        raise ValueError("controller gate exchange crosses filesystems")

    def check():
        if regular(fragment) != original or mask(slot) != owned_mask:
            raise ValueError("controller gate exchange inputs changed")

    _rename(fragment, slot, 2, check)


def read(path):
    _, raw = files.read(path, MAXIMUM)
    if not raw.isascii():
        raise ValueError("controller gate record is not ASCII")
    value = decode(raw)
    if files.encoded(value) != raw:
        raise ValueError("controller gate record is not canonical")
    return value


def sync_file(path):
    owned = files.identity(path, directory=False)
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    try:
        row = os.fstat(descriptor)
        if (row.st_uid, row.st_dev, row.st_ino) != (owned["uid"], owned["device"], owned["inode"]):
            raise ValueError("controller gate file changed before synchronization")
        os.fsync(descriptor)
        files.unchanged(path, owned, directory=False)
    finally:
        os.close(descriptor)


def save(path, value, sync_directory, *, previous=None):
    raw = files.encoded(value)
    if not 0 < len(raw) <= MAXIMUM:
        raise ValueError("controller gate record exceeds its boundary")
    parent = files.identity(path.parent)
    current = files.identity(path, directory=False) if present(path) else None
    if ((previous is None and current is not None)
            or (previous is not None and (current is None or read(path) != previous))):
        raise ValueError("controller gate record changed before publication")
    temporary = path.parent / (".gate." + uuid.uuid4().hex)
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600)
    row = os.fstat(descriptor)
    owned = row.st_dev, row.st_ino
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        staged = files.identity(temporary, directory=False)
        if (staged["device"], staged["inode"]) != owned:
            raise ValueError("controller gate temporary identity changed")

        def check():
            files.unchanged(path.parent, parent)
            files.unchanged(temporary, staged, directory=False)
            if files.read(temporary, MAXIMUM)[1] != raw:
                raise ValueError("controller gate temporary changed")
            if current is not None:
                files.unchanged(path, current, directory=False)
                if read(path) != previous:
                    raise ValueError("controller gate previous record changed")

        _rename(temporary, path, 1 if current is None else 0, check)
    finally:
        discard_temporary(temporary, owned)
    files.unchanged(path.parent, parent)
    sync_directory(path.parent)


def directory(path, sync_directory):
    if not present(path):
        path.mkdir(mode=0o700)
        sync_directory(path.parent)
    return files.identity(path)


def directories(root, operation):
    return {"maintenance": root / "maintenance", "installation": operation.parent,
            "operation": operation, "slots": operation / "slots"}


def validate_record(value, root, operation, target, digest, lock, unit_directory, units):
    keys(value, ("format", "version", "operation_id", "target_sha256", "target", "root", "lock",
                 "unit_directory", "directories", "units", "state"))
    if (value["format"] != "qadra-controller-entry-gate" or type(value["version"]) is not int
            or value["version"] != 1 or value["operation_id"] != operation.name
            or value["target_sha256"] != digest or value["target"] != target
            or value["state"] not in ("prepared", "masking", "reload_pending", "gated")):
        raise ValueError("controller gate journal belongs to another intention")
    for name, expected in (("root", files.located(root)), ("unit_directory", files.located(unit_directory))):
        keys(value[name], ("path", "uid", "device", "inode"))
        identity_shape({key: value[name][key] for key in ("uid", "device", "inode")})
        if value[name] != expected:
            raise ValueError("controller gate directory identity changed")
    identity_shape(value["lock"])
    if value["lock"] != lock:
        raise ValueError("controller gate deployment lock changed")
    paths = directories(root, operation)
    keys(value["directories"], paths)
    for name, path in paths.items():
        identity_shape(value["directories"][name])
        files.unchanged(path, value["directories"][name])
    keys(value["units"], units)
    with os.scandir(operation / "slots") as entries:
        names = []
        for entry in entries:
            names.append(entry.name)
            if len(names) > len(units):
                raise ValueError("controller gate contains unrecognized slots")
    if set(names) != set(units):
        raise ValueError("controller gate slots are incomplete")
    return {unit: orientation(unit_directory / unit, operation / "slots" / unit, value["units"][unit])
            for unit in units}
