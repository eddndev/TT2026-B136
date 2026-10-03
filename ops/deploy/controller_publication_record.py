"""Exact identities and observed orientations for one controller publication."""
from pathlib import Path

from backup_manifest_schema import hexadecimal, keys
from controller_publication_files import (
    digest, encoded, identity, inventory_shape, located, snapshot, unchanged,
)

FIELDS = ("format", "version", "state", "operation_id", "root", "lock", "parents",
          "operation", "staging", "sources", "manifest_sha256", "source_revision",
          "previous_sha256", "installed_sha256", "previous", "candidate")


def _identity(value, *, path=False):
    keys(value, ("path", "uid", "device", "inode") if path else ("uid", "device", "inode"))
    if any(type(value[key]) is not int or value[key] < 0 for key in ("uid", "device", "inode")):
        raise ValueError("controller publication identity is invalid")
    if path and (not isinstance(value["path"], str) or not Path(value["path"]).is_absolute()):
        raise ValueError("controller publication location is invalid")


def validate(value):
    keys(value, FIELDS)
    if (value["format"] != "qadra-controller-publication-state"
            or type(value["version"]) is not int or value["version"] != 1
            or value["state"] not in ("prepared", "published")):
        raise ValueError("controller publication record format is unsupported")
    for name in ("root", "staging"):
        _identity(value[name], path=True)
    for name in ("lock", "operation", "sources"):
        _identity(value[name])
    keys(value["parents"], ("maintenance", "controllers"))
    for item in value["parents"].values():
        _identity(item)
    for name in ("manifest_sha256", "previous_sha256", "installed_sha256"):
        hexadecimal(value[name], 64)
    hexadecimal(value["source_revision"], 40)
    for name, fingerprint in (("previous", "previous_sha256"), ("candidate", "installed_sha256")):
        keys(value[name], ("identity", "files"))
        _identity(value[name]["identity"])
        inventory_shape(value[name]["files"])
        if digest(encoded(value[name]["files"])) != value[fingerprint]:
            raise ValueError("controller publication inventory digest differs")
    if value["previous"]["identity"] == value["candidate"]["identity"]:
        raise ValueError("controller publication generations share an identity")


def fixed(value, root, operation, lock):
    if located(root) != value["root"] or identity(root / "deploy.lock", directory=False) != lock:
        raise ValueError("controller publication root or lock changed")
    unchanged(root / "maintenance", value["parents"]["maintenance"])
    unchanged(operation.parent, value["parents"]["controllers"])
    unchanged(operation, value["operation"])
    if located(Path(value["staging"]["path"])) != value["staging"]:
        raise ValueError("controller source staging identity changed")
    unchanged(Path(value["staging"]["path"]) / "sources", value["sources"])


def orientation(value, root, operation, lock):
    fixed(value, root, operation, lock)
    installed, _ = snapshot(root / "tools")
    retained, _ = snapshot(operation / "exchange")
    fixed(value, root, operation, lock)
    if installed == value["previous"] and retained == value["candidate"]:
        if value["state"] == "prepared":
            return "prepared"
    elif installed == value["candidate"] and retained == value["previous"]:
        return "installed"
    raise ValueError("controller publication directories differ from their recorded generations")


def receipt(value, operation):
    return {"operation_id": value["operation_id"], "manifest_sha256": value["manifest_sha256"],
            "previous_sha256": value["previous_sha256"], "installed_sha256": value["installed_sha256"],
            "previous_path": str(operation / "exchange")}
