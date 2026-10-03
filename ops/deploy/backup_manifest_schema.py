"""Strict versioned shape for capture metadata, without restore admission."""
from datetime import datetime
import json
import re
import uuid

from bundle import validate_version


NAME = "backup-manifest.json"
TEMPORARY = ".backup-manifest.tmp"
PAYLOADS = ("database.dump", "redis.rdb", "private-state.tar.gz")
MAX_MANIFEST = 64 * 1024
MAX_PAYLOAD = 8 * 1024 ** 3
MAX_CONTROLLER_FILE = 256 * 1024
MAX_CONTROLLER_TOTAL = 8 * 1024 * 1024
MAX_CONTROLLER_FILES = 64
MAX_RELEASE = 8 * 1024 * 1024
PYTHON_NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*\.py")


def keys(value, expected):
    if type(value) is not dict or set(value) != set(expected):
        raise ValueError("backup metadata fields are incomplete or unknown")


def hexadecimal(value, length):
    if not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]{" + str(length) + "}", value):
        raise ValueError("backup metadata digest is not canonical")


def version(value):
    if not isinstance(value, str):
        raise ValueError("backup release version is invalid")
    validate_version(value)


def record(value, maximum):
    keys(value, ("bytes", "sha256"))
    if type(value["bytes"]) is not int or not 0 < value["bytes"] <= maximum:
        raise ValueError("backup metadata byte count is outside its boundary")
    hexadecimal(value["sha256"], 64)


def decode(raw):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("backup metadata has duplicate fields")
            result[key] = value
        return result

    def nonfinite(_value):
        raise ValueError("backup metadata contains a nonfinite number")

    try:
        return json.loads(raw.decode("utf-8"), object_pairs_hook=unique, parse_constant=nonfinite)
    except (UnicodeError, json.JSONDecodeError, RecursionError):
        raise ValueError("backup metadata is not bounded valid JSON") from None


def check(document):
    keys(document, ("format", "version", "backup_id", "captured_at", "source", "payloads"))
    if (document["format"] != "qadra-private-backup" or type(document["version"]) is not int
            or document["version"] != 1):
        raise ValueError("backup metadata format is unsupported")
    value = document["backup_id"]
    try:
        parsed = uuid.UUID(value) if isinstance(value, str) else None
    except (ValueError, AttributeError):
        parsed = None
    if parsed is None or parsed.int == 0 or str(parsed) != value:
        raise ValueError("backup identity must be a canonical non-nil UUID")
    captured = document["captured_at"]
    if not isinstance(captured, str) or not re.fullmatch(r"[0-9]{4}(-[0-9]{2}){2}T[0-9]{2}(:[0-9]{2}){2}Z", captured):
        raise ValueError("backup capture time is not canonical UTC")
    try:
        datetime.strptime(captured, "%Y-%m-%dT%H:%M:%SZ")
    except ValueError:
        raise ValueError("backup capture time is not a real calendar instant") from None
    keys(document["payloads"], PAYLOADS)
    for payload in document["payloads"].values():
        record(payload, MAX_PAYLOAD)
    source = document["source"]
    keys(source, ("initialized", "schema", "release", "controller"))
    if type(source["initialized"]) is not bool:
        raise ValueError("backup initialization state must be explicit")
    if source["initialized"]:
        hexadecimal(source["schema"], 64)
        release = source["release"]
        keys(release, ("version", "commit", "schema", "manifest_sha256"))
        version(release["version"])
        hexadecimal(release["commit"], 40)
        hexadecimal(release["schema"], 64)
        hexadecimal(release["manifest_sha256"], 64)
        if release["schema"] != source["schema"]:
            raise ValueError("backup source schema differs from its declared release")
    elif source["schema"] is not None or source["release"] is not None:
        raise ValueError("uninitialized backup cannot declare an active release")
    controller = source["controller"]
    if (type(controller) is not dict or not 2 <= len(controller) <= MAX_CONTROLLER_FILES
            or not {"runtime.py", "backup_manifest.py"}.issubset(controller)):
        raise ValueError("backup controller inventory is incomplete or oversized")
    for name, information in controller.items():
        if not isinstance(name, str) or not PYTHON_NAME.fullmatch(name):
            raise ValueError("backup controller names must be immediate Python basenames")
        record(information, MAX_CONTROLLER_FILE)
    if sum(value["bytes"] for value in controller.values()) > MAX_CONTROLLER_TOTAL:
        raise ValueError("backup controller inventory exceeds its total byte boundary")
