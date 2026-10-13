"""One recoverable CA issuance, with no private holder key or automatic repair.

The injected executor verifies CSR possession, root and certificate signatures,
and exact leaf profiles. Its issue method must pass lock_fd to every signing
subprocess so an orphaned signer cannot outlive the shared deployment lock.
"""
import json
from pathlib import Path
import stat
import time
import uuid

import crl_journal as journal
import issuance_material as material


FORMAT = "qadra-ca-issuance-v1"
REQUEST_FIELDS = {"csr_pem", "purpose", "holder", "reference", "criterion",
                  "operator_id", "expected_root_fingerprint"}
HISTORY_FIELDS = ("operator_id", "holder", "reference", "criterion")


def identifier(value):
    try:
        parsed = uuid.UUID(value)
        if not parsed.int or str(parsed) != value:
            raise ValueError("non-canonical identity")
    except (ValueError, TypeError, AttributeError):
        raise ValueError("CA operation and operator identities must be canonical non-nil UUIDs") from None
    return value


def request_value(value):
    if not isinstance(value, dict) or set(value) != REQUEST_FIELDS:
        raise ValueError("CA request fields differ")
    result = dict(value)
    csr = result["csr_pem"]
    if not isinstance(csr, str) or not csr.isascii() or not 0 < len(csr) <= 16384:
        raise ValueError("CSR must be bounded ASCII public material")
    if result["purpose"] not in ("document-signing", "partner"):
        raise ValueError("unsupported CA certificate purpose")
    for field in ("holder", "reference", "criterion"):
        text = result[field]
        if (not isinstance(text, str) or not text.strip()
                or len(text.encode("utf-8")) > 500
                or any(ord(character) < 32 or ord(character) == 127 for character in text)):
            raise ValueError("CA review text is empty, unbounded or contains control characters")
    identifier(result["operator_id"])
    if not material.hex_digest(result["expected_root_fingerprint"]):
        raise ValueError("adopted CA fingerprint must be lowercase SHA-256 hexadecimal")
    return result


def locations(root, operation_id):
    ca = root / "data/ca"
    for path in (root / "data", ca, ca / "certs", ca / "newcerts"):
        material.directory(path)
    operation = ca / "operations" / operation_id
    for path in (operation.parent, operation):
        if not material.absent(path):
            material.directory(path)
    return ca, operation


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("CA journal repeats a field")
        result[key] = value
    return result


def load(operation):
    path = operation / "journal.json"
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) != 0o600:
        raise ValueError("CA journal must be a private regular file")
    record = json.loads(journal.read(path, 128 * 1024), object_pairs_hook=unique_object)
    fields = {"format", "operation_id", "request", "admitted", "before", "recorded_at", "receipt"}
    if (not isinstance(record, dict) or set(record) != fields or record["format"] != FORMAT
            or identifier(record["operation_id"]) != operation.name
            or type(record["recorded_at"]) is not int or record["recorded_at"] <= 0):
        raise ValueError("CA journal identity or format differs")
    request_value(record["request"])
    facts = record["admitted"]
    if (not isinstance(facts, dict) or set(facts) != {"csr_sha256", "spki_sha256", "root_fingerprint"}
            or not all(material.hex_digest(value) for value in facts.values())
            or facts["root_fingerprint"] != record["request"]["expected_root_fingerprint"]):
        raise ValueError("CA journal admission differs")
    before = record["before"]
    if (not isinstance(before, dict) or set(before) != {"files", "serial"}
            or not isinstance(before["files"], dict) or set(before["files"]) != set(material.LIMITS)
            or material.serial(before["serial"]) != before["serial"]
            or any(not material.hex_digest(value) and not (name in material.OPTIONAL and value is None)
                   for name, value in before["files"].items())):
        raise ValueError("CA journal baseline is invalid")
    return record


def completed(record):
    receipt = record["receipt"]
    if receipt is None:
        return None
    fields = {"operation_id", "serial", "fingerprint", "purpose", "certificate_pem", "recorded_at",
              *HISTORY_FIELDS}
    if (not isinstance(receipt, dict) or set(receipt) != fields
            or receipt["operation_id"] != record["operation_id"]
            or receipt["serial"] != record["before"]["serial"]
            or receipt["purpose"] != record["request"]["purpose"]
            or receipt["recorded_at"] != record["recorded_at"]
            or not material.hex_digest(receipt["fingerprint"])
            or not isinstance(receipt["certificate_pem"], str)
            or not receipt["certificate_pem"].isascii()
            or not 0 < len(receipt["certificate_pem"]) <= 16384
            or any(receipt[field] != record["request"][field] for field in HISTORY_FIELDS)):
        raise ValueError("CA issuance receipt differs from its journal")
    return dict(receipt)


def guard_pending(root, current_operation=None):
    """Under deploy.lock, exclude changes while another issuance needs recovery."""
    operations = Path(root) / "data/ca/operations"
    if material.absent(operations):
        return
    material.directory(operations)
    for operation in operations.iterdir():
        identifier(operation.name)
        material.directory(operation)
        if operation.name != current_operation and completed(load(operation)) is None:
            raise RuntimeError("another CA issuance requires recovery before changing its baseline")


def continue_issuance(ca, operation, record, executor, lock_fd):
    receipt = completed(record)
    if receipt is not None:
        return receipt
    request = record["request"]
    if material.admit(ca, request, executor) != record["admitted"]:
        raise ValueError("saved CSR or adopted root differs from its admission")
    if material.pristine(ca, operation.name, record["before"]):
        executor.issue(ca, operation.name, request["csr_pem"].encode("ascii"),
                       request["purpose"], lock_fd=lock_fd)
    recovered = material.recover(ca, operation.name, record, executor)
    receipt = {**recovered, "operation_id": operation.name, "recorded_at": record["recorded_at"],
               **{field: request[field] for field in HISTORY_FIELDS}}
    record["receipt"] = receipt
    journal.save(operation / "journal.json", record)
    return dict(receipt)


def issue(root, operation_id, request, executor):
    """Issue once or reconcile the exact prior request under the shared lock."""
    operation_id, request = identifier(operation_id), request_value(request)
    root = Path(root).resolve(strict=True)
    with journal.locked(root) as lock_fd:
        journal.guard_mutation(root, ca_operation=("operations", operation_id))
        ca, operation = locations(root, operation_id)
        if not material.absent(operation):
            record = load(operation)
            if record["request"] != request:
                raise ValueError("CA operation already belongs to different request parameters")
            if completed(record) is None:
                guard_pending(root, operation_id)
        else:
            guard_pending(root, operation_id)
            admitted = material.admit(ca, request, executor)
            before = material.prepare(ca, operation_id)
            record = {"format": FORMAT, "operation_id": operation_id, "request": request,
                      "admitted": admitted, "before": before,
                      "recorded_at": int(time.time()), "receipt": None}
            journal.directory(operation.parent)
            journal.directory(operation)
            journal.save(operation / "journal.json", record)
        return continue_issuance(ca, operation, record, executor, lock_fd)


def resume(root, operation_id, executor):
    """Resume saved public material; never infer success or repair partial state."""
    operation_id = identifier(operation_id)
    root = Path(root).resolve(strict=True)
    with journal.locked(root) as lock_fd:
        journal.guard_mutation(root, ca_operation=("operations", operation_id))
        ca, operation = locations(root, operation_id)
        record = load(operation)
        if completed(record) is None:
            guard_pending(root, operation_id)
        return continue_issuance(ca, operation, record, executor, lock_fd)
