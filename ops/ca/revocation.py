"""One exact, durable local revocation; CRL publication is a separate operation."""
from datetime import datetime
import json
import os
from pathlib import Path
import re
import stat
import time

import crl_journal as journal
import inventory
import issuance
import issuance_material as material


FORMAT = "qadra-ca-revocation-v1"
REQUEST_FIELDS = {"serial", "fingerprint", "reason", "operator_id", "expected_root_fingerprint"}
PARTIAL = "revocation state is partial or differs from its saved baseline"


def request_value(value):
    if not isinstance(value, dict) or set(value) != REQUEST_FIELDS:
        raise ValueError("CA revocation request fields differ")
    result = dict(value)
    result["serial"] = material.serial(result["serial"])
    if not all(material.hex_digest(result[name])
               for name in ("fingerprint", "expected_root_fingerprint")):
        raise ValueError("CA revocation requires exact SHA-256 identities")
    issuance.identifier(result["operator_id"])
    reason = result["reason"]
    if (not isinstance(reason, str) or not reason.strip() or len(reason.encode("utf-8")) > 500
            or any(ord(character) < 32 or ord(character) == 127 for character in reason)):
        raise ValueError("CA revocation reason is empty, unbounded or contains controls")
    return result


def _locations(root, operation_id):
    ca = root / "data/ca"
    for path in (root / "data", ca):
        material.directory(path)
    operation = ca / "revocations" / operation_id
    for path in (operation.parent, operation):
        if not material.absent(path):
            material.directory(path)
    return ca, operation


def _saved_row(before, requested_serial):
    value = before["row"]
    if not isinstance(value, str) or not 0 < len(value) <= 65536:
        raise ValueError("CA revocation baseline row exceeds its bound")
    raw = bytes.fromhex(value)
    fields = raw.rstrip(b"\n").split(b"\t")
    if (not raw.endswith(b"\n") or raw.count(b"\n") != 1 or len(fields) != 6
            or fields[0] not in (b"V", b"R", b"E")
            or material.serial(fields[3].decode("ascii")) != requested_serial):
        raise ValueError("CA revocation baseline row is invalid")
    return raw, fields


def _load(operation):
    path = operation / "journal.json"
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) != 0o600:
        raise ValueError("CA revocation journal must be a private regular file")
    record = json.loads(journal.read(path, 128 * 1024), object_pairs_hook=issuance.unique_object)
    if (not isinstance(record, dict)
            or set(record) != {"format", "operation_id", "request", "before", "recorded_at", "receipt"}
            or record["format"] != FORMAT
            or issuance.identifier(record["operation_id"]) != operation.name
            or type(record["recorded_at"]) is not int or record["recorded_at"] <= 0):
        raise ValueError("CA revocation journal identity or format differs")
    if request_value(record["request"]) != record["request"]:
        raise ValueError("CA revocation journal identity is not canonical")
    before = record["before"]
    if (not isinstance(before, dict) or set(before) != {"files", "serial", "row"}
            or not isinstance(before["files"], dict) or set(before["files"]) != set(material.LIMITS)
            or material.serial(before["serial"]) != before["serial"]
            or any(not material.hex_digest(value) and not (name in material.OPTIONAL and value is None)
                   for name, value in before["files"].items())
            or any(before["files"][name] is not None for name in material.LIMITS if name.endswith(".new"))):
        raise ValueError("CA revocation journal baseline is invalid")
    _saved_row(before, record["request"]["serial"])
    return record


def _receipt(record):
    return {"operation_id": record["operation_id"], "recorded_at": record["recorded_at"],
            **{field: record["request"][field] for field in ("serial", "fingerprint", "reason", "operator_id")},
            "local_status": "revoked", "published_status": "pending"}


def _completed(record):
    if record["receipt"] is None:
        return None
    expected = _receipt(record)
    if record["receipt"] != expected:
        raise ValueError("CA revocation receipt differs from its journal")
    return expected


def guard_pending(root, current_operation=None):
    """Under deploy.lock, exclude mutations that could obscure pending recovery."""
    operations = Path(root) / "data/ca/revocations"
    if material.absent(operations):
        return
    material.directory(operations)
    for operation in operations.iterdir():
        issuance.identifier(operation.name)
        material.directory(operation)
        if operation.name != current_operation and _completed(_load(operation)) is None:
            raise RuntimeError("another CA revocation requires recovery before changing its baseline")


def _exact_certificate(root, request, executor):
    # The caller already owns deploy.lock; the public download API takes another lock.
    found = inventory._certificates(root, executor)
    if request["serial"] not in found:
        raise LookupError("certificate serial is absent from the local inventory")
    facts, certificate, _ = found[request["serial"]]
    if (facts["fingerprint"] != request["fingerprint"]
            or facts["root_fingerprint"] != request["expected_root_fingerprint"]):
        raise ValueError("revocation certificate or adopted root identity differs")
    return certificate


def _prepare(ca, request):
    before = material.snapshot(ca)
    if any(before["files"][name] is not None for name in material.LIMITS if name.endswith(".new")):
        raise RuntimeError(PARTIAL)
    row = next(raw for raw, number, _ in material.index(ca) if number == request["serial"])
    before["row"] = row.hex()
    _saved_row(before, request["serial"])
    return before


def _revoked_date(value):
    if not re.fullmatch(rb"(?:[0-9]{12}|[0-9]{14})Z", value):
        return False
    try:
        datetime.strptime(value.decode("ascii"), "%y%m%d%H%M%SZ" if len(value) == 13 else "%Y%m%d%H%M%SZ")
    except ValueError:
        return False
    return True


def _recover(ca, record):
    before, request = record["before"], record["request"]
    original, prior_fields = _saved_row(before, request["serial"])
    current = material.snapshot(ca)
    rows = material.index(ca)
    selected = [fields for _, number, fields in rows if number == request["serial"]]
    if len(selected) != 1:
        raise RuntimeError(PARTIAL)
    fields = selected[0]
    reconstructed = b"".join(original if number == request["serial"] else raw for raw, number, _ in rows)
    if (fields[0] != b"R" or not _revoked_date(fields[2])
            or any(fields[index] != prior_fields[index] for index in (1, 3, 4, 5))
            or material.digest(reconstructed) != before["files"]["index.txt"]
            or any(current["files"][name] is not None for name in material.LIMITS if name.endswith(".new"))):
        raise RuntimeError(PARTIAL)
    if prior_fields[0] == b"R":
        if current != {key: before[key] for key in ("files", "serial")}:
            raise RuntimeError(PARTIAL)
    else:
        changed = {"index.txt", "index.txt.old", "index.txt.attr.old"}
        if (any(current["files"][name] != before["files"][name] for name in material.LIMITS if name not in changed)
                or current["files"]["index.txt.old"] != before["files"]["index.txt"]
                or current["files"]["index.txt.attr.old"] not in
                   (before["files"]["index.txt.attr.old"], before["files"]["index.txt.attr"])):
            raise RuntimeError(PARTIAL)
    # Flush OpenSSL effects before publishing the durable receipt.
    for name in material.LIMITS:
        if current["files"][name] is None:
            continue
        descriptor = os.open(ca / name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        try:
            if not stat.S_ISREG(os.fstat(descriptor).st_mode):
                raise ValueError("CA material changed its file type")
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
    journal.sync_directory(ca)


def revoke(root, operation_id, request, executor):
    """Record first, revoke exact public identity once, and recover only certain state."""
    operation_id, request = issuance.identifier(operation_id), request_value(request)
    root = Path(root).resolve(strict=True)
    with journal.locked(root) as lock_fd:
        journal.guard_mutation(root, ca_operation=("revocations", operation_id))
        ca, operation = _locations(root, operation_id)
        if not material.absent(operation):
            record = _load(operation)
            if record["request"] != request:
                raise ValueError("CA operation already belongs to different revocation parameters")
            completed = _completed(record)
            if completed is not None:
                return completed
        else:
            record = None
        issuance.guard_pending(root)
        guard_pending(root, operation_id)
        certificate = _exact_certificate(root, request, executor)
        if record is None:
            record = {"format": FORMAT, "operation_id": operation_id, "request": request,
                      "before": _prepare(ca, request), "recorded_at": int(time.time()), "receipt": None}
            journal.directory(operation.parent)
            journal.directory(operation)
            journal.save(operation / "journal.json", record)
        baseline = {key: record["before"][key] for key in ("files", "serial")}
        _, prior_fields = _saved_row(record["before"], request["serial"])
        if material.snapshot(ca) == baseline and prior_fields[0] != b"R":
            executor.revoke(ca, certificate, lock_fd=lock_fd)
        _exact_certificate(root, request, executor)
        _recover(ca, record)
        record["receipt"] = _receipt(record)
        journal.save(operation / "journal.json", record)
        return dict(record["receipt"])
