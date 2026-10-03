"""Validate the pinned local deployment identity without retaining its secrets."""
import os
from pathlib import Path
import re
import stat

from backup_manifest_files import inspect
from backup_manifest_schema import decode, hexadecimal, keys
from restore_fence import _identifier


UNITS = ("qadra-web.service", "qadra-api.service", "qadra-postgres.service", "qadra-redis.service")
PORTS = ("web_port", "api_port", "postgres_port", "redis_port")
MAXIMUM = 65536


def exact_path(path):
    if (not isinstance(path, Path) or not path.is_absolute() or path.resolve(strict=True) != path
            or any(part.is_symlink() for part in (path, *path.parents))):
        raise ValueError("restore target path is not exact")
    return path


def directory(path, *, private=False):
    exact_path(path)
    info = path.stat()
    if (not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid()
            or (private and stat.S_IMODE(info.st_mode) != 0o700)):
        raise ValueError("restore target directory ownership or permissions differ")
    return info


def root_identity(root):
    info = directory(root, private=True)
    return {"path": str(root), "uid": info.st_uid, "device": info.st_dev, "inode": info.st_ino}


def material(record, path=None, *, private=False, executable=False):
    keys(record, ("path", "size", "sha256"))
    if not isinstance(record["path"], str) or (path is not None and record["path"] != str(path)):
        raise ValueError("restore target file belongs to another location")
    path = exact_path(Path(record["path"]))
    maximum = 512 * 1024 * 1024 if executable else MAXIMUM
    if type(record["size"]) is not int or not 0 < record["size"] <= maximum:
        raise ValueError("restore target file size is invalid")
    hexadecimal(record["sha256"], 64)
    info = path.stat()
    if info.st_mode & 0o022 or (private and info.st_uid != os.getuid()):
        raise ValueError("restore target file ownership or permissions differ")
    if executable and (not os.access(path, os.X_OK) or info.st_uid not in (0, os.getuid())):
        raise ValueError("restore service controller is not a trusted local executable")
    observed, raw = inspect(path, maximum, private=private, contents=not executable)
    if observed != {"bytes": record["size"], "sha256": record["sha256"]}:
        raise ValueError("restore target file fingerprint changed")
    return raw


def local_environment(value):
    keys(value, ("HOME", "XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS"))
    if any(not isinstance(entry, str) or os.environ.get(name) != entry for name, entry in value.items()):
        raise ValueError("restore target user manager context differs")
    directory(Path(value["HOME"]))
    directory(Path(value["XDG_RUNTIME_DIR"]), private=True)
    if value["DBUS_SESSION_BUS_ADDRESS"] != "unix:path=" + value["XDG_RUNTIME_DIR"] + "/bus":
        raise ValueError("restore target requires the current local user bus")
    return {**value, "LC_ALL": "C"}


def load(root, target_path, expected):
    identity = root_identity(root)
    hexadecimal(expected, 64)
    directory(root / "receipts", private=True)
    exact_path(target_path)
    if target_path.parent != root / "receipts" or target_path.stat().st_uid != os.getuid():
        raise ValueError("restore target descriptor is outside its private receipts")
    observed, raw = inspect(target_path, MAXIMUM, private=True, contents=True)
    if observed["sha256"] != expected or not raw.isascii():
        raise ValueError("restore target descriptor differs from its pin")
    target = decode(raw)
    keys(target, ("format", "version", "target_id", "root", "environment", "settings", "systemctl", "units", "ports"))
    if target["format"] != "qadra-restore-target" or type(target["version"]) is not int or target["version"] != 1:
        raise ValueError("restore target descriptor format is unsupported")
    _identifier(target["target_id"])
    keys(target["root"], identity)
    if (any(type(target["root"][key]) is not int for key in ("uid", "device", "inode"))
            or target["root"] != identity):
        raise ValueError("restore target root identity differs")
    env = local_environment(target["environment"])
    directory(root / "config", private=True)
    directory(root / "data", private=True)
    config = decode(material(target["settings"], root / "config/settings.json", private=True))
    if type(config) is not dict:
        raise ValueError("restore deployment settings are invalid")
    ports = {unit: config[field] for unit, field in zip(UNITS, PORTS)}
    if (any(type(port) is not int or not 0 < port <= 65535 for port in ports.values())
            or len(set(ports.values())) != len(UNITS)):
        raise ValueError("restore deployment ports are invalid")
    keys(target["ports"], UNITS)
    if any(type(port) is not int for port in target["ports"].values()) or target["ports"] != ports:
        raise ValueError("restore target ports differ from deployment settings")
    material(target["systemctl"], executable=True)
    keys(target["units"], UNITS)
    unit_directory = Path(env["HOME"]) / ".config/systemd/user"
    directory(unit_directory)
    for unit in UNITS:
        item = target["units"][unit]
        keys(item, ("fragment", "working_directory", "control_group"))
        raw_unit = material(item["fragment"], unit_directory / unit)
        if not raw_unit.isascii() or item["working_directory"] != str(root / "data"):
            raise ValueError("restore target unit directory differs")
        lines = raw_unit.decode("ascii").splitlines()
        if lines.count("WorkingDirectory=" + str(root / "data")) != 1 or lines.count("TimeoutStopSec=90") != 1:
            raise ValueError("restore target unit does not use its managed working directory or stop deadline")
        group = item["control_group"]
        prefix = f"/user.slice/user-{os.getuid()}.slice/user@{os.getuid()}.service/"
        if (not isinstance(group, str) or len(group) > 512 or not group.startswith(prefix)
                or not re.fullmatch(r"(?:[A-Za-z0-9_@.-]+/)*" + re.escape(unit), group[len(prefix):])
                or any(part in (".", "..") for part in group.split("/"))):
            raise ValueError("restore target unit cgroup differs from the local user manager")
    return target, env
