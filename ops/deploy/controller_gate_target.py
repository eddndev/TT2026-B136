"""Attested local manager context and complete controller entry observations."""
import os
from pathlib import Path
import re

from backup_manifest_files import present
from backup_manifest_schema import decode, hexadecimal, keys
import controller_publication_files as files
from controller_publication import _identifier
import restore_quiesce_target as targets
from restore_quiesce import PROPERTIES as SERVICE_PROPERTIES, process_id


UNITS = targets.UNITS
PROPERTIES = (*SERVICE_PROPERTIES, "UnitFileState")


def load(root, target_path, expected):
    identity = targets.root_identity(root)
    hexadecimal(expected, 64)
    files.identity(root / "receipts")
    files.exact_path(target_path)
    if target_path.parent != root / "receipts":
        raise ValueError("controller gate target is outside its private receipts")
    observed, raw = files.read(target_path, targets.MAXIMUM)
    if observed["sha256"] != expected or not raw.isascii():
        raise ValueError("controller gate target differs from its external pin")
    target = decode(raw)
    keys(target, ("format", "version", "target_id", "root", "environment", "settings", "systemctl", "units", "ports"))
    if (target["format"] != "qadra-restore-target" or type(target["version"]) is not int
            or target["version"] != 1):
        raise ValueError("controller gate target format is unsupported")
    _identifier(target["target_id"])
    keys(target["root"], identity)
    if (any(type(target["root"][name]) is not int for name in ("uid", "device", "inode"))
            or target["root"] != identity):
        raise ValueError("controller gate target root differs")
    environment = targets.local_environment(target["environment"])
    files.identity(root / "config")
    files.identity(root / "data")
    settings = decode(targets.material(target["settings"], root / "config/settings.json", private=True))
    if type(settings) is not dict:
        raise ValueError("controller gate deployment settings are invalid")
    ports = {unit: settings[name] for unit, name in zip(UNITS, targets.PORTS)}
    keys(target["ports"], UNITS)
    if (any(type(port) is not int or not 0 < port <= 65535 for port in ports.values())
            or len(set(ports.values())) != len(UNITS)
            or any(type(port) is not int for port in target["ports"].values())
            or target["ports"] != ports):
        raise ValueError("controller gate ports differ from its deployment")
    targets.material(target["systemctl"], executable=True)
    unit_directory = Path(environment["HOME"]) / ".config/systemd/user"
    directory = files.identity(unit_directory)
    if directory["device"] != identity["device"]:
        raise ValueError("controller gate units and retained originals cross filesystems")
    keys(target["units"], UNITS)
    prefix = f"/user.slice/user-{os.getuid()}.slice/user@{os.getuid()}.service/"
    for unit, value in target["units"].items():
        keys(value, ("fragment", "working_directory", "control_group"))
        fragment = value["fragment"]
        keys(fragment, ("path", "size", "sha256"))
        if (fragment["path"] != str(unit_directory / unit) or type(fragment["size"]) is not int
                or not 0 < fragment["size"] <= targets.MAXIMUM
                or value["working_directory"] != str(root / "data")):
            raise ValueError("controller gate fragment attestation differs")
        hexadecimal(fragment["sha256"], 64)
        group = value["control_group"]
        if (not isinstance(group, str) or len(group) > 512 or not group.startswith(prefix)
                or not re.fullmatch(r"(?:[A-Za-z0-9_@.-]+/)*" + re.escape(unit), group[len(prefix):])
                or any(part in (".", "..") for part in group.split("/"))):
            raise ValueError("controller gate cgroup belongs to another manager")
    if any(present(unit_directory / name) for name in (
            "service.d", "qadra-.service.d", *(unit + ".d" for unit in UNITS))):
        raise ValueError("controller gate cannot adopt unit drop-ins")
    return target, environment, unit_directory


def attest_originals(value, target):
    for unit, entry in value.items():
        expected = target["units"][unit]["fragment"]
        original = entry["original"]
        if original["bytes"] != expected["size"] or original["sha256"] != expected["sha256"]:
            raise ValueError("controller gate original differs from its attested fragment")


def rows(raw, target, orientations, *, complete=False):
    if not isinstance(raw, str) or not raw.isascii() or not 0 < len(raw) <= 65536:
        raise ValueError("controller gate manager observation exceeds its boundary")
    result = {}
    for block in raw.strip().split("\n\n"):
        row = {}
        for line in block.splitlines():
            key, separator, value = line.partition("=")
            if not separator or key in row:
                raise ValueError("controller gate manager observation is ambiguous")
            row[key] = value
        keys(row, PROPERTIES)
        unit = row["Id"]
        if unit not in UNITS or unit in result:
            raise ValueError("controller gate manager observation belongs to another unit")
        expected = target["units"][unit]
        masked = row["LoadState"] == "masked"
        if (row["DropInPaths"] or row["NeedDaemonReload"] not in ("yes", "no")
                or row["LoadState"] not in ("loaded", "masked") or not row["UnitFileState"]
                or (orientations[unit] == "original" and (masked or row["UnitFileState"] == "masked"
                                                         or row["NeedDaemonReload"] != "no"))
                or (complete and (not masked or row["UnitFileState"] != "masked" or row["NeedDaemonReload"] != "no"))):
            raise ValueError("controller gate manager has not established the required entry state")
        if masked:
            if (orientations[unit] != "masked" or row["UnitFileState"] != "masked"
                    or row["FragmentPath"] not in (expected["fragment"]["path"], "/dev/null")):
                raise ValueError("controller gate manager mask belongs to another path")
        elif (row["FragmentPath"] != expected["fragment"]["path"]
                or row["WorkingDirectory"] != expected["working_directory"]
                or row["TimeoutStopUSec"] not in ("1min 30s", "90s", "90000000")):
            raise ValueError("controller gate loaded fragment identity differs")
        main, control = process_id(row["MainPID"]), process_id(row["ControlPID"])
        inactive = row["ActiveState"] == "inactive" and row["SubState"] == "dead" and main == 0
        active = row["ActiveState"] == "active" and row["SubState"] == "running" and main > 0
        group = row["ControlGroup"]
        if (control != 0 or not (inactive or active)
                or (group != expected["control_group"] and not (inactive and not group))):
            raise ValueError("controller gate process and cgroup observations differ")
        result[unit] = row
    if set(result) != set(UNITS):
        raise ValueError("controller gate manager observation is incomplete")
    return result
