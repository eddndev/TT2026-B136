"""Approve one published controller generation without operating the deployment."""
import ctypes
import errno
import os
import uuid

from backup_manifest_files import discard_temporary
from backup_manifest_schema import MAX_MANIFEST, decode, hexadecimal
import controller_approval_record as approvals
import controller_publication_files as files
import controller_publication_record as publications
import crl_journal as journal


def _publication(root, operation, expected_root, lock, publication_sha256, installed_sha256):
    if files.located(root) != expected_root:
        raise ValueError("controller approval root changed")
    files.unchanged(root / "deploy.lock", lock, directory=False)
    files.identity(operation)
    observed, raw = files.read(operation / "journal.json", MAX_MANIFEST)
    if observed["sha256"] != publication_sha256 or not raw.isascii():
        raise ValueError("controller publication differs from its approved journal digest")
    value = decode(raw)
    publications.validate(value)
    if (value["state"] != "published" or value["operation_id"] != operation.name
            or value["root"] != expected_root or value["lock"] != lock
            or value["installed_sha256"] != installed_sha256):
        raise ValueError("controller publication does not identify the approved installation")
    if publications.orientation(value, root, operation, lock) != "installed":
        raise ValueError("controller approval requires the published candidate")
    return value


def _existing(path, expected_root):
    if not os.path.lexists(path):
        return None
    owned = files.identity(path, directory=False)
    _, raw = files.read(path, approvals.MAX_APPROVAL)
    value = approvals.parse(raw)
    files.unchanged(path, owned, directory=False)
    if value["root"] != expected_root:
        raise ValueError("controller approval belongs to another root")
    return {"identity": owned, "raw": raw, "value": value}


def _predecessor(current, previous_sha256):
    if current is None:
        if previous_sha256 is not None:
            raise ValueError("expected previous controller approval is missing")
    elif previous_sha256 is None or approvals.digest(current["raw"]) != previous_sha256:
        raise ValueError("current controller approval differs from the explicit predecessor")


def _sync_file(path, owned):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    try:
        info = os.fstat(descriptor)
        if (info.st_uid, info.st_dev, info.st_ino) != (owned["uid"], owned["device"], owned["inode"]):
            raise ValueError("controller approval file changed before synchronization")
        os.fsync(descriptor)
        files.unchanged(path, owned, directory=False)
    finally:
        os.close(descriptor)


def _rename_exclusive(source, destination):
    if source.parent != destination.parent:
        raise ValueError("controller approval publication must keep its private parent")
    parent = files.identity(destination.parent)
    temporary = files.identity(source, directory=False)
    library = ctypes.CDLL(None, use_errno=True)
    try:
        rename = library.renameat2
    except AttributeError:
        raise OSError(errno.ENOSYS, "exclusive controller approval publication is unavailable") from None
    rename.argtypes = (ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p,
                       ctypes.c_uint)
    rename.restype = ctypes.c_int
    descriptor = os.open(destination.parent, os.O_RDONLY | os.O_DIRECTORY
                         | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        row = os.fstat(descriptor)
        if (row.st_uid, row.st_dev, row.st_ino) != (parent["uid"], parent["device"], parent["inode"]):
            raise ValueError("controller approval parent changed before exclusive publication")
        files.unchanged(destination.parent, parent)
        files.unchanged(source, temporary, directory=False)
        if rename(descriptor, os.fsencode(source.name), descriptor, os.fsencode(destination.name), 1):
            code = ctypes.get_errno()
            raise OSError(code, "exclusive controller approval publication failed")
    finally:
        os.close(descriptor)


def _publish_record(path, raw, reobserve, *, exclusive):
    parent = files.identity(path.parent)
    temporary = path.parent / (".approval." + uuid.uuid4().hex)
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL
                         | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600)
    info = os.fstat(descriptor)
    owned = info.st_dev, info.st_ino
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        reobserve()
        files.unchanged(path.parent, parent)
        temporary_identity = files.identity(temporary, directory=False)
        if (temporary_identity["device"], temporary_identity["inode"]) != owned:
            raise ValueError("controller approval temporary changed before publication")
        if files.read(temporary, approvals.MAX_APPROVAL)[1] != raw:
            raise ValueError("controller approval temporary bytes changed")
        if exclusive:
            _rename_exclusive(temporary, path)
        else:
            os.replace(temporary, path)
    finally:
        discard_temporary(temporary, owned)
    files.unchanged(path.parent, parent)
    files.sync_directory(path.parent)


def approve_controllers(root, operation_id, *, expected_publication_sha256,
                        expected_installed_sha256, expected_previous_approval_sha256):
    """Promote exact publication evidence with an explicit previous selection."""
    operation_id = approvals.identifier(operation_id)
    hexadecimal(expected_publication_sha256, 64)
    hexadecimal(expected_installed_sha256, 64)
    if expected_previous_approval_sha256 is not None:
        hexadecimal(expected_previous_approval_sha256, 64)
    expected_root = files.located(root)
    lock = files.identity(root / "deploy.lock", directory=False)
    operation = root / "maintenance/controllers" / operation_id
    history_path = operation / "approval.json"
    current_path = operation.parent / "approved.json"
    with journal.locked(root):
        def published():
            return _publication(root, operation, expected_root, lock,
                                expected_publication_sha256, expected_installed_sha256)

        publication = published()
        value = approvals.from_publication(publication, expected_publication_sha256,
                                           expected_previous_approval_sha256)
        raw = approvals.validate(value)
        history = _existing(history_path, expected_root)
        current = _existing(current_path, expected_root)
        if history is not None and history["raw"] != raw:
            raise ValueError("controller operation already records another approval")
        already_current = current is not None and current["raw"] == raw
        if already_current:
            if history is None:
                raise ValueError("current controller approval lacks its operation evidence")
        else:
            _predecessor(current, expected_previous_approval_sha256)
            if current is not None and current["value"]["operation_id"] == operation_id:
                raise ValueError("controller operation cannot replace its own different approval")

        def reobserve():
            if published() != publication or _existing(current_path, expected_root) != current:
                raise ValueError("controller publication or selected approval changed")

        if history is None:
            _publish_record(history_path, raw, reobserve, exclusive=True)
            history = _existing(history_path, expected_root)
        if history is None or history["raw"] != raw:
            raise ValueError("controller operation evidence differs after publication")
        _sync_file(history_path, history["identity"])
        files.sync_directory(operation)

        def ready():
            reobserve()
            if _existing(history_path, expected_root) != history:
                raise ValueError("controller operation evidence changed before selection")

        ready()
        if already_current:
            _sync_file(current_path, current["identity"])
            files.sync_directory(operation.parent)
        else:
            _publish_record(current_path, raw, ready, exclusive=False)
        if published() != publication:
            raise ValueError("controller publication changed during approval promotion")
        observed_history = _existing(history_path, expected_root)
        observed_current = _existing(current_path, expected_root)
        if (observed_history != history or observed_current is None
                or observed_current["raw"] != raw):
            raise ValueError("controller approval changed during durable promotion")
        return approvals.receipt(value)
