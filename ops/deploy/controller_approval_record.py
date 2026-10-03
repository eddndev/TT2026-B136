"""Canonical secret-free approval values shared by storage and pure rendering."""
import hashlib
import json
from pathlib import Path
import uuid

from backup_manifest_schema import decode, hexadecimal, keys


MAX_APPROVAL = 16 * 1024
FIELDS = ("format", "version", "operation_id", "root", "generation", "installed_sha256",
          "manifest_sha256", "source_revision", "publication_sha256", "previous_approval_sha256")


def encoded(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("ascii")


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def identifier(value):
    try:
        parsed = uuid.UUID(value)
        if not parsed.int or str(parsed) != value:
            raise ValueError("noncanonical identity")
    except (ValueError, AttributeError, TypeError):
        raise ValueError("controller approval requires a canonical non-nil UUID") from None
    return value


def identity(value, *, located=False):
    keys(value, ("path", "uid", "device", "inode") if located else ("uid", "device", "inode"))
    if any(type(value[key]) is not int or value[key] < 0 for key in ("uid", "device", "inode")):
        raise ValueError("controller approval identity is invalid")
    if located:
        name = value["path"]
        if (not isinstance(name, str) or not name or Path(name).anchor != "/"
                or str(Path(name)) != name or ".." in Path(name).parts):
            raise ValueError("controller approval location is not canonical")


def validate(value):
    keys(value, FIELDS)
    if (value["format"] != "qadra-controller-approval"
            or type(value["version"]) is not int or value["version"] != 1):
        raise ValueError("controller approval format is unsupported")
    identifier(value["operation_id"])
    identity(value["root"], located=True)
    identity(value["generation"])
    if value["root"]["uid"] != value["generation"]["uid"]:
        raise ValueError("controller approval ownership differs")
    for name in ("installed_sha256", "manifest_sha256", "publication_sha256"):
        hexadecimal(value[name], 64)
    hexadecimal(value["source_revision"], 40)
    if value["previous_approval_sha256"] is not None:
        hexadecimal(value["previous_approval_sha256"], 64)
    raw = encoded(value)
    if len(raw) > MAX_APPROVAL:
        raise ValueError("controller approval exceeds its byte boundary")
    return raw


def parse(raw):
    if not raw or len(raw) > MAX_APPROVAL or not raw.isascii():
        raise ValueError("controller approval is not bounded ASCII")
    value = decode(raw)
    if validate(value) != raw:
        raise ValueError("controller approval bytes are not canonical")
    return value


def from_publication(value, publication_sha256, previous_approval_sha256):
    result = {"format": "qadra-controller-approval", "version": 1,
              "operation_id": value["operation_id"], "root": value["root"],
              "generation": value["candidate"]["identity"],
              "installed_sha256": value["installed_sha256"],
              "manifest_sha256": value["manifest_sha256"],
              "source_revision": value["source_revision"],
              "publication_sha256": publication_sha256,
              "previous_approval_sha256": previous_approval_sha256}
    validate(result)
    return result


def receipt(value):
    return {"operation_id": value["operation_id"], "approval_sha256": digest(validate(value)),
            "installed_sha256": value["installed_sha256"]}
