"""Capture a quiescent private target and publish an exact compatibility receipt."""
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import tempfile

import backup_manifest
from backup_manifest_files import discard_temporary, inspect, present, sync_directory
import backup_manifest_schema as capture_schema
import crl_journal
import restore_admission
import restore_admission_release
import restore_admission_schema as schema
import restore_catalog
from restore_commands import run
from restore_observations import observe, private_directory
from runtime import Runtime, environment


@contextmanager
def temporary(root, prefix, value, *, directory=None):
    directory = root / "data/tmp" if directory is None else directory
    private_directory(directory)
    descriptor, name = tempfile.mkstemp(prefix=prefix, dir=directory)
    information = os.fstat(descriptor)
    owned = information.st_dev, information.st_ino
    path = Path(name)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(value)
            stream.flush()
            os.fsync(stream.fileno())
        yield path
    finally:
        discard_temporary(path, owned)


@contextmanager
def audit_sidecar(path):
    sidecar = Path(str(path) + ".lock")
    descriptor = os.open(sidecar, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        information = os.fstat(descriptor)
    finally:
        os.close(descriptor)
    owned = information.st_dev, information.st_ino
    try:
        yield
    finally:
        discard_temporary(sidecar, owned)


def verified_audit(root, tools, target, release):
    env = environment(root, admin=True)
    rows, encoded = restore_catalog.audit(tools, env, target["schema"])
    with temporary(root, "restore-audit-", encoded) as path, audit_sidecar(path):
        result = run([release / "bin/despacho-cli", "--json", "audit", "verify-chain"],
                     env={**env, "AUDIT_LOG_PATH": str(path), "LD_LIBRARY_PATH": str(release / "lib")},
                     timeout=10, maximum=8192).stdout
    if len(result) > 8192:
        raise ValueError("restore audit verification exceeds its boundary")
    receipt = capture_schema.decode(result)
    capture_schema.keys(receipt, ("valid", "entries"))
    if (receipt["valid"] is not True or type(receipt["entries"]) is not int
            or receipt["entries"] != len(rows)):
        raise ValueError("restore audit verification did not confirm the entire source")
    predecessor = None if not rows else {"sequence": rows[-1]["sequence"], "head": rows[-1]["chain"]}
    return rows, predecessor


def destination(root, path):
    private_directory(root)
    if (not isinstance(path, Path) or path.parent != root / "receipts"
            or not path.name.isascii() or not 1 <= len(path.name) <= 128
            or any(character not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-"
                   for character in path.name) or path.name in (".", "..")):
        raise ValueError("restore capture receipt must use its private managed directory")
    private_directory(path.parent)
    if present(path):
        raise ValueError("restore capture never replaces an existing receipt")


def capture(root, *, controller_directory, targets, tools, descriptor_path):
    try:
        destination(root, descriptor_path)
        with crl_journal.locked(root):
            crl_journal.guard(root)
            destination(root, descriptor_path)
            source = backup_manifest.capture_source(root, controller_directory)
            if source["initialized"] is not True:
                raise ValueError("restore capture requires an initialized release")
            restore_admission_release.verify(root, source["release"])
            release = (root / "current").resolve(strict=True)
            facts, catalog = observe(root, targets, tools)
            audit, predecessor = verified_audit(root, tools, targets["postgres"], release)
            def backup_command(arguments, **kwargs):
                capture_data = arguments[0] == tools["pg_dump"] or "--rdb" in arguments
                kwargs["timeout"] = min(kwargs["timeout"], 300 if capture_data else 10)
                return run(arguments, **kwargs)
            backup = Runtime(root).backup(tools=tools, command=backup_command)
            after, after_catalog = observe(root, targets, tools)
            after_audit, _ = restore_catalog.audit(tools, environment(root, admin=True), targets["postgres"]["schema"])
            manifest = backup_manifest.validate(root, backup)
            if (facts != after or catalog != after_catalog or audit != after_audit
                    or manifest["source"] != source
                    or backup_manifest.capture_source(root, controller_directory) != source):
                raise ValueError("restore source changed across the private capture")
            files = dict(manifest["payloads"])
            for name, maximum in (("COMPLETE", 4096), (capture_schema.NAME, capture_schema.MAX_MANIFEST)):
                files[name] = inspect(backup / name, maximum, private=True)[0]
            header = restore_admission.prefix(backup / "redis.rdb", 9, files["redis.rdb"])
            if not header.startswith(b"REDIS") or not header[5:].isdigit():
                raise ValueError("restore capture does not contain a supported RDB header")
            facts["redis"]["rdb_version"] = int(header[5:])
            document = {"format": "qadra-restore-compatibility", "version": 1,
                        "backup": {"id": manifest["backup_id"], "captured_at": manifest["captured_at"], "files": files},
                        "release": source["release"], "controller": source["controller"],
                        **facts, "audit_predecessor": predecessor}
            schema.check(document)
            encoded = (json.dumps(document, sort_keys=True) + "\n").encode("ascii")
            if len(encoded) > schema.MAX_DESCRIPTOR:
                raise ValueError("restore compatibility receipt exceeds its boundary")
            digest = hashlib.sha256(encoded).hexdigest()
            with temporary(root, "restore-descriptor-", encoded, directory=descriptor_path.parent) as staged:
                restore_admission.admit(root, backup, staged, expected_descriptor_sha256=digest,
                                        controller_directory=controller_directory, observed=facts)
                destination(root, descriptor_path)
                os.link(staged, descriptor_path, follow_symlinks=False)
                sync_directory(descriptor_path.parent)
            return {"backup": backup, "descriptor_sha256": digest}
    except (OSError, ValueError, RuntimeError, KeyError, TypeError, UnicodeError):
        raise RuntimeError("private restore capture failed") from None
