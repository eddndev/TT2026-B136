"""Publish a complete private controller directory without operating services."""
import ctypes
import errno
import os
from pathlib import Path
import uuid

from backup_manifest_schema import hexadecimal
import crl_journal as journal
import controller_publication_files as files
import controller_publication_record as records


def exchange_directories(left, right):
    """Exchange two owned directories with one Linux operation and no fallback."""
    first, second = files.identity(left), files.identity(right)
    if (first["device"] != second["device"] or left == right
            or left.is_relative_to(right) or right.is_relative_to(left)):
        raise ValueError("controller exchange requires separate directories on one filesystem")
    library = ctypes.CDLL(None, use_errno=True)
    try:
        exchange = library.renameat2
    except AttributeError:
        raise OSError(errno.ENOSYS, "atomic controller directory exchange is unavailable") from None
    exchange.argtypes = (ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p,
                         ctypes.c_uint)
    exchange.restype = ctypes.c_int
    descriptors = []
    try:
        for parent in (left.parent, right.parent):
            expected = files.identity(parent)
            descriptor = os.open(parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
            descriptors.append(descriptor)
            info = os.fstat(descriptor)
            if (info.st_uid, info.st_dev, info.st_ino) != (
                    expected["uid"], expected["device"], expected["inode"]):
                raise ValueError("controller exchange parent changed")
        files.unchanged(left, first)
        files.unchanged(right, second)
        if exchange(descriptors[0], os.fsencode(left.name), descriptors[1], os.fsencode(right.name), 2):
            code = ctypes.get_errno()
            raise OSError(code, "atomic controller directory exchange failed")
    finally:
        for descriptor in descriptors:
            os.close(descriptor)


def _identifier(value):
    try:
        parsed = uuid.UUID(value)
        if not parsed.int or str(parsed) != value:
            raise ValueError("noncanonical operation")
    except (ValueError, TypeError, AttributeError):
        raise ValueError("controller operation must be a canonical non-nil UUID") from None
    return value


def _root_unchanged(root, expected, lock):
    if files.located(root) != expected:
        raise ValueError("controller publication root changed")
    files.unchanged(root / "deploy.lock", lock, directory=False)


def _parents(root):
    for path in (root / "maintenance", root / "maintenance/controllers"):
        if os.path.lexists(path):
            files.identity(path)


def _prepare(root, operation, expected, lock, staging, sources, manifest, contents,
             manifest_sha256, previous_sha256):
    previous, _ = files.snapshot(root / "tools")
    if files.digest(files.encoded(previous["files"])) != previous_sha256:
        raise ValueError("installed controller inventory differs from its approved digest")
    _root_unchanged(root, expected, lock)
    for parent in (root / "maintenance", operation.parent):
        files.ensure_directory(parent)
    parents = {"maintenance": files.identity(root / "maintenance"),
               "controllers": files.identity(operation.parent)}
    operation.mkdir(mode=0o700)
    owned_operation = files.identity(operation)
    files.sync_directory(operation.parent)
    owned_candidate = files.copy_sources(operation / "exchange", contents)
    candidate, _ = files.snapshot(operation / "exchange")
    if (candidate["files"] != manifest["files"] or candidate["identity"] != owned_candidate
            or candidate["identity"]["device"] != previous["identity"]["device"]):
        raise ValueError("staged controller generation differs or crosses filesystems")
    value = {"format": "qadra-controller-publication-state", "version": 1,
             "state": "prepared", "operation_id": operation.name, "root": expected, "lock": lock,
             "parents": parents, "operation": owned_operation, "staging": staging, "sources": sources,
             "manifest_sha256": manifest_sha256, "source_revision": manifest["source_revision"],
             "previous_sha256": previous_sha256,
             "installed_sha256": files.digest(files.encoded(candidate["files"])),
             "previous": previous, "candidate": candidate}
    records.validate(value)
    records.orientation(value, root, operation, lock)
    files.write_record(operation / "journal.json", value)
    return value


def _resume(root, operation, expected, lock, staging, sources, manifest,
            manifest_sha256, previous_sha256):
    files.identity(operation)
    value = files.load_record(operation / "journal.json")
    records.validate(value)
    if (value["operation_id"] != operation.name or value["root"] != expected
            or value["lock"] != lock or value["staging"] != staging
            or value["sources"] != sources or value["manifest_sha256"] != manifest_sha256
            or value["previous_sha256"] != previous_sha256
            or value["source_revision"] != manifest["source_revision"]
            or value["candidate"]["files"] != manifest["files"]):
        raise ValueError("controller publication reentry differs from its original intention")
    records.orientation(value, root, operation, lock)
    return value


def _complete(value, root, operation, lock):
    if records.orientation(value, root, operation, lock) == "prepared":
        exchange_directories(root / "tools", operation / "exchange")
    if records.orientation(value, root, operation, lock) != "installed":
        raise ValueError("controller publication did not install its candidate")
    # An earlier exchange or receipt rename may have outlived its failed fsync.
    files.sync_directory(root)
    files.sync_directory(operation)
    if records.orientation(value, root, operation, lock) != "installed":
        raise ValueError("controller publication changed during directory synchronization")
    if value["state"] == "prepared":
        value = {**value, "state": "published"}
        files.write_record(operation / "journal.json", value)
    if files.load_record(operation / "journal.json") != value:
        raise ValueError("controller publication receipt changed")
    if records.orientation(value, root, operation, lock) != "installed":
        raise ValueError("controller publication receipt lacks its exact generations")
    return records.receipt(value, operation)


def _inputs(root, operation_id, staged_directory, manifest_sha256, previous_sha256):
    operation_id = _identifier(operation_id)
    hexadecimal(manifest_sha256, 64)
    hexadecimal(previous_sha256, 64)
    expected = files.located(root)
    lock = files.identity(root / "deploy.lock", directory=False)
    files.exact_path(staged_directory)
    if (staged_directory.is_relative_to(root / "tools")
            or staged_directory.is_relative_to(root / "maintenance/controllers")
            or Path(__file__).resolve().is_relative_to(root / "tools")):
        raise ValueError("controller publisher and staging must remain outside exchanged generations")
    return expected, lock, root / "maintenance/controllers" / operation_id


def _publish_locked(root, operation_id, staged_directory, *,
                    expected_manifest_sha256, expected_previous_sha256):
    """Publish exact generations while the caller holds the deployment lock."""
    expected, lock, operation = _inputs(root, operation_id, staged_directory,
                                       expected_manifest_sha256, expected_previous_sha256)
    _root_unchanged(root, expected, lock)
    manifest, staging, sources, contents = files.package(staged_directory, expected_manifest_sha256)
    _parents(root)
    if os.path.lexists(operation):
        value = _resume(root, operation, expected, lock, staging, sources, manifest,
                        expected_manifest_sha256, expected_previous_sha256)
    else:
        value = _prepare(root, operation, expected, lock, staging, sources, manifest, contents,
                         expected_manifest_sha256, expected_previous_sha256)
    return _complete(value, root, operation, lock)


def publish_controllers(root, operation_id, staged_directory, *,
                        expected_manifest_sha256, expected_previous_sha256):
    """Publish or reconcile exact source generations under the deployment lock."""
    expected, lock, _ = _inputs(root, operation_id, staged_directory,
                                expected_manifest_sha256, expected_previous_sha256)
    with journal.locked(root):
        _root_unchanged(root, expected, lock)
        return _publish_locked(root, operation_id, staged_directory,
                               expected_manifest_sha256=expected_manifest_sha256,
                               expected_previous_sha256=expected_previous_sha256)
