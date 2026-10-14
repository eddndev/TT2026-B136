"""Do not mutate a CA whose portal operation still needs reconciliation."""
import stat
import hashlib
import re
import uuid


FIELDS = {"state", "reason", "operation_id", "operator_id", "recorded_at", "rejected_at",
          "csr_sha256", "root_fingerprint", "reserved_serial", "holder", "reference",
          "criterion", "purpose"}
HISTORY = ("operator_id", "holder", "reference", "criterion", "purpose")


def _identity(value):
    parsed = uuid.UUID(value)
    return bool(parsed.int) and str(parsed) == value


def _digest(value):
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def validate_rejected_receipt(record, operation_id):
    """Correlate the strict rejected union member with its immutable journal."""
    try:
        receipt, request, facts = record["receipt"], record["request"], record["admitted"]
        serial, recorded_at = record["before"]["serial"], record["recorded_at"]
        csr = request["csr_pem"]
        valid = (
            record["format"] == "qadra-ca-issuance-v1"
            and _identity(operation_id) and record["operation_id"] == operation_id
            and isinstance(receipt, dict) and set(receipt) == FIELDS
            and receipt["state"] == "rejected" and receipt["reason"] == "csr_subject_policy"
            and receipt["operation_id"] == operation_id and _identity(receipt["operator_id"])
            and type(recorded_at) is int and recorded_at > 0
            and type(receipt["recorded_at"]) is int and receipt["recorded_at"] == recorded_at
            and type(receipt["rejected_at"]) is int and receipt["rejected_at"] >= recorded_at
            and isinstance(facts, dict) and set(facts) == {"csr_sha256", "spki_sha256", "root_fingerprint"}
            and all(_digest(value) for value in facts.values())
            and isinstance(csr, str) and csr.isascii() and 0 < len(csr) <= 16384
            and facts["csr_sha256"] == hashlib.sha256(csr.encode("ascii")).hexdigest()
            and receipt["csr_sha256"] == facts["csr_sha256"]
            and receipt["root_fingerprint"] == facts["root_fingerprint"] == request["expected_root_fingerprint"]
            and isinstance(serial, str) and re.fullmatch(r"[0-9A-F]{1,40}", serial) is not None
            and 0 < int(serial, 16) < 2**159 - 1 and format(int(serial, 16), "X") == serial
            and receipt["reserved_serial"] == serial
            and receipt["purpose"] in ("partner", "document-signing")
            and all(receipt[field] == request[field] for field in HISTORY)
            and all(isinstance(request[field], str) and request[field].strip()
                    and len(request[field].encode("utf-8")) <= 500
                    and all(ord(character) >= 32 and ord(character) != 127 for character in request[field])
                    for field in ("holder", "reference", "criterion")))
    except (KeyError, TypeError, ValueError, AttributeError, UnicodeError):
        valid = False
    if not valid:
        raise ValueError("CA rejected issuance receipt differs from its journal")
    return dict(receipt)


def guard(root, allowed=None):
    from crl_journal import load
    ca = root / "data/ca"
    for group in ("operations", "revocations", "publications"):
        directory = ca / group
        try:
            info = directory.lstat()
        except FileNotFoundError:
            continue
        if not stat.S_ISDIR(info.st_mode):
            raise ValueError("CA operation directory must be a directory")
        for operation in directory.iterdir():
            if not stat.S_ISDIR(operation.lstat().st_mode):
                raise ValueError("CA operation must be a directory")
            if allowed == (group, operation.name):
                continue
            record = load(operation / "journal.json")
            if not isinstance(record.get("receipt"), dict) or not record["receipt"]:
                raise RuntimeError("A pending CA portal operation must be reconciled before another mutation")
            receipt = record["receipt"]
            rejection = receipt.get("state") == "rejected" or (group == "operations" and
                ("reserved_serial" in receipt or receipt.get("reason") == "csr_subject_policy"))
            if rejection:
                if group != "operations":
                    raise ValueError("CA subject policy rejection is only valid for issuance")
                validate_rejected_receipt(record, operation.name)
