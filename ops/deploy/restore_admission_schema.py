"""Strict compatibility facts for an explicitly pinned offline restore candidate."""
import re

import backup_manifest_schema as capture
from bundle import MAX_BYTES


MAX_DESCRIPTOR = 64 * 1024
FILES = (*capture.PAYLOADS, "COMPLETE", capture.NAME)
ROLE_FLAGS = ("login", "superuser", "createdb", "createrole", "inherit",
              "replication", "bypassrls")


def integer(value, minimum, maximum):
    if type(value) is not int or not minimum <= value <= maximum:
        raise ValueError("restore compatibility integer is outside its boundary")


def identifier(value):
    if (not isinstance(value, str) or len(value) > 63
            or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]{0,62}", value)):
        raise ValueError("restore compatibility identifier is invalid")


def text(value):
    if (not isinstance(value, str) or not 1 <= len(value) <= 128
            or not value.isascii() or any(not 32 <= ord(character) < 127 for character in value)):
        raise ValueError("restore compatibility text is outside its boundary")


def version(value, components):
    if (not isinstance(value, str) or len(value) > components * 7 - 1
            or not re.fullmatch(r"(0|[1-9][0-9]{0,5})(\.(0|[1-9][0-9]{0,5})){"
                                + str(components - 1) + r"}", value)):
        raise ValueError("restore compatibility tool version is not canonical")
    return tuple(int(part) for part in value.split("."))


def tools(value, names):
    capture.keys(value, names)
    for record in value.values():
        capture.record(record, MAX_BYTES)


def role(value):
    capture.keys(value, (*ROLE_FLAGS, "connection_limit", "member_of"))
    if any(type(value[key]) is not bool for key in ROLE_FLAGS):
        raise ValueError("restore compatibility role flags must be explicit booleans")
    integer(value["connection_limit"], -1, 1000000)
    memberships = value["member_of"]
    if type(memberships) is not list or len(memberships) > 16:
        raise ValueError("restore compatibility memberships exceed their boundary")
    for name in memberships:
        identifier(name)
    if memberships != sorted(set(memberships)):
        raise ValueError("restore compatibility memberships must be sorted and unique")


def postgres(value):
    capture.keys(value, ("server_version_num", "pg_dump_version", "pg_restore_version",
                         "dump_format", "database", "schema", "owner", "runtime_role",
                         "encoding", "collate", "ctype", "locale_provider", "roles", "tools"))
    integer(value["server_version_num"], 100000, 999999)
    for key in ("pg_dump_version", "pg_restore_version"):
        if version(value[key], 2)[0] != value["server_version_num"] // 10000:
            raise ValueError("PostgreSQL tool and server major versions differ")
    if (value["dump_format"] != "custom" or value["encoding"] != "UTF8"
            or value["locale_provider"] != "libc"):
        raise ValueError("restore compatibility PostgreSQL format or locale is unsupported")
    for key in ("database", "schema", "owner", "runtime_role"):
        identifier(value[key])
    for key in ("collate", "ctype"):
        text(value[key])
    owner, runtime = value["owner"], value["runtime_role"]
    if owner == runtime:
        raise ValueError("restore database owner and runtime role must differ")
    capture.keys(value["roles"], (owner, runtime))
    for profile in value["roles"].values():
        role(profile)
    profile = value["roles"][runtime]
    if (profile["login"] is not True or profile["member_of"]
            or any(profile[key] for key in ROLE_FLAGS if key != "login")):
        raise ValueError("restore runtime role exceeds the restricted provisioned profile")
    tools(value["tools"], ("server", "pg_dump", "pg_restore"))


def redis(value):
    capture.keys(value, ("engine", "server_version", "cli_version", "rdb_version",
                         "database", "appendonly", "tools"))
    if value["engine"] not in ("redis", "valkey"):
        raise ValueError("restore compatibility Redis engine is unsupported")
    version(value["server_version"], 3)
    version(value["cli_version"], 3)
    integer(value["rdb_version"], 1, 9999)
    integer(value["database"], 0, 0)
    if type(value["appendonly"]) is not bool:
        raise ValueError("restore compatibility persistence mode must be explicit")
    tools(value["tools"], ("server", "redis_cli", "redis_check_rdb"))


def facts(value):
    capture.keys(value, ("postgres", "redis"))
    postgres(value["postgres"])
    redis(value["redis"])


def check(document):
    capture.keys(document, ("format", "version", "backup", "release", "controller",
                            "postgres", "redis", "audit_predecessor"))
    if (document["format"] != "qadra-restore-compatibility"
            or type(document["version"]) is not int or document["version"] != 1):
        raise ValueError("restore compatibility descriptor format is unsupported")
    backup = document["backup"]
    capture.keys(backup, ("id", "captured_at", "files"))
    capture.keys(backup["files"], FILES)
    for name, record in backup["files"].items():
        maximum = {"COMPLETE": 4096, capture.NAME: capture.MAX_MANIFEST}.get(
            name, capture.MAX_PAYLOAD)
        capture.record(record, maximum)
    capture.keys(document["release"], ("version", "commit", "schema", "manifest_sha256"))
    # The capture schema already defines these exact identities and inventories.
    capture.check({
        "format": "qadra-private-backup", "version": 1,
        "backup_id": backup["id"], "captured_at": backup["captured_at"],
        "source": {"initialized": True, "schema": document["release"]["schema"],
                   "release": document["release"], "controller": document["controller"]},
        "payloads": {name: backup["files"][name] for name in capture.PAYLOADS},
    })
    facts({name: document[name] for name in ("postgres", "redis")})
    predecessor = document["audit_predecessor"]
    if predecessor is not None:
        capture.keys(predecessor, ("sequence", "head"))
        integer(predecessor["sequence"], 0, 2**63 - 1)
        capture.hexadecimal(predecessor["head"], 64)
