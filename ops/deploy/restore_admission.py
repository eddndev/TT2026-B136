"""Read-only exact compatibility evidence; never restore or authorize publication."""
import os
import re
import stat

import backup_manifest
from backup_manifest_files import identity, inspect
import backup_manifest_schema as capture
import crl_journal
import restore_admission_release as installed
import restore_admission_schema as schema


def prefix(path, amount, expected):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        before = os.fstat(stream.fileno())
        if (not stat.S_ISREG(before.st_mode) or stat.S_IMODE(before.st_mode) != 0o600
                or before.st_size != expected["bytes"] or before.st_size < amount):
            raise ValueError("restore container header input differs from the captured file")
        value = stream.read(amount)
        after = os.fstat(stream.fileno())
        if (len(value) != amount or identity(before) != identity(after)
                or identity(after) != identity(path.lstat())):
            raise ValueError("restore container changed while its header was checked")
    return value


def admit(root, backup, descriptor_path, *, expected_descriptor_sha256,
          controller_directory, observed):
    capture.hexadecimal(expected_descriptor_sha256, 64)
    fingerprint, raw = inspect(descriptor_path, schema.MAX_DESCRIPTOR, private=True, contents=True)
    if fingerprint["sha256"] != expected_descriptor_sha256:
        raise ValueError("restore compatibility descriptor differs from its external pin")
    descriptor = capture.decode(raw)
    schema.check(descriptor)
    schema.facts(observed)
    if crl_journal.pending(root):
        raise ValueError("pending CRL maintenance prevents restore compatibility admission")
    manifest = backup_manifest.validate(root, backup)
    if manifest["source"]["initialized"] is not True:
        raise ValueError("restore compatibility requires an initialized source capture")
    expected = descriptor["backup"]
    if (manifest["backup_id"] != expected["id"]
            or manifest["captured_at"] != expected["captured_at"]):
        raise ValueError("restore compatibility capture identity differs")
    for name in schema.FILES:
        if name in capture.PAYLOADS:
            # validate has already rehashed each payload against this exact record.
            actual = manifest["payloads"][name]
        else:
            maximum = 4096 if name == "COMPLETE" else capture.MAX_MANIFEST
            actual, _ = inspect(backup / name, maximum, private=True)
        if actual != expected["files"][name]:
            raise ValueError("restore compatibility complete file inventory differs")
    for field in ("release", "controller"):
        if manifest["source"][field] != descriptor[field]:
            raise ValueError("restore compatibility source identity differs from its descriptor")
    current = backup_manifest.capture_source(root, controller_directory)
    if current != manifest["source"]:
        raise ValueError("restore compatibility installed source identity differs")
    installed.verify(root, descriptor["release"])
    if observed != {name: descriptor[name] for name in ("postgres", "redis")}:
        raise ValueError("restore compatibility observed destination facts differ")
    files = expected["files"]
    if prefix(backup / "database.dump", 5, files["database.dump"]) != b"PGDMP":
        raise ValueError("restore compatibility requires a custom PostgreSQL dump header")
    header = prefix(backup / "redis.rdb", 9, files["redis.rdb"])
    if (not re.fullmatch(rb"REDIS[0-9]{4}", header)
            or int(header[5:]) != descriptor["redis"]["rdb_version"]):
        raise ValueError("restore compatibility RDB header differs from its recorded format")
    return {"backup_id": expected["id"], "descriptor_sha256": fingerprint["sha256"],
            "audit_predecessor": descriptor["audit_predecessor"]}
