"""Finite CA portal commands with server-supplied actor and adopted root."""
from pathlib import Path
import time

import inventory
import issuance
import revocation
import publication
import issuance_material as material
from public_state import public_state


FIELDS = {
    "status": set(), "inventory": {"offset", "limit"}, "history": {"offset", "limit"},
    "certificate": {"serial", "fingerprint"}, "preview": {"csr_pem", "purpose"},
    "issue": {"operation_id", "csr_pem", "purpose", "holder", "reference", "criterion",
              "expected_root_fingerprint"}, "resume-issue": {"operation_id"},
    "revoke": {"operation_id", "serial", "fingerprint", "reason", "expected_root_fingerprint"},
    "resume-revoke": {"operation_id"},
    "renew-crl": {"operation_id", "expected_revision", "acknowledge_maintenance"},
    "resume-crl": {"operation_id"},
}


def page(items, command):
    offset, limit = command["offset"], command["limit"]
    if type(offset) is not int or offset < 0 or type(limit) is not int or not 1 <= limit <= 100:
        raise ValueError("CA page bounds are invalid")
    return {"items": items[offset:offset + limit], "total": len(items), "offset": offset, "limit": limit}


def history(root):
    records = []
    directory = root / "data/ca/operations"
    with inventory._read_locked(root):
        if not material.absent(directory):
            material.directory(directory)
            for operation in directory.iterdir():
                issuance.identifier(operation.name)
                material.directory(operation)
                record = issuance.load(operation)
                receipt = issuance.completed(record)
                rejected = receipt is not None and receipt.get("state") == "rejected"
                records.append({"operation_id": operation.name, "kind": "issue",
                    "state": "rejected" if rejected else "completed" if receipt else "pending",
                    "recorded_at": record["recorded_at"],
                    **{field: record["request"][field] for field in issuance.HISTORY_FIELDS},
                    "serial": receipt["serial"] if receipt and not rejected else None,
                    "fingerprint": receipt["fingerprint"] if receipt and not rejected else None,
                    **({"reason": receipt["reason"]} if rejected else {})})
        revoked = root / "data/ca/revocations"
        if not material.absent(revoked):
            material.directory(revoked)
            for operation in revoked.iterdir():
                issuance.identifier(operation.name)
                material.directory(operation)
                record = revocation._load(operation)
                receipt = revocation._completed(record)
                records.append({"operation_id": operation.name, "kind": "revoke",
                    "state": "completed" if receipt else "pending", "recorded_at": record["recorded_at"],
                    **{field: record["request"][field] for field in
                       ("operator_id", "serial", "fingerprint", "reason")}})
        publications = root / "data/ca/publications"
        if not material.absent(publications):
            material.directory(publications)
            for operation in publications.iterdir():
                issuance.identifier(operation.name)
                material.directory(operation)
                record = publication._load(operation)
                receipt = publication._completed(record)
                records.append({"operation_id": operation.name, "kind": "publication",
                    "state": "completed" if receipt else "pending", "recorded_at": record["recorded_at"],
                    **{field: record["request"][field] for field in ("operator_id", "expected_revision")},
                    "revision": receipt["revision"] if receipt else None})
    return sorted(records, key=lambda record: (record["recorded_at"], record["operation_id"]), reverse=True)


def execute(root, actor, command, executor, expected_root_fingerprint):
    issuance.identifier(actor)
    root = Path(root).resolve(strict=True)
    if not material.hex_digest(expected_root_fingerprint):
        raise ValueError("the configured CA fingerprint is invalid")
    action = command.get("action") if isinstance(command, dict) else None
    if action not in FIELDS or set(command) != {"action", *FIELDS[action]}:
        raise ValueError("unsupported CA command or fields")
    if action in ("inventory", "history"):
        page([], command)
    if action == "status":
        return public_state(root, expected_root_fingerprint)
    if action == "renew-crl":
        return publication.publish(root, command["operation_id"], {
            "operator_id": actor, "expected_revision": command["expected_revision"],
            "acknowledge_maintenance": command["acknowledge_maintenance"],
            "expected_root_fingerprint": expected_root_fingerprint})
    if action == "resume-crl":
        operation_id = issuance.identifier(command["operation_id"])
        record = publication._load(root / "data/ca/publications" / operation_id)
        if record["request"]["expected_root_fingerprint"] != expected_root_fingerprint:
            raise ValueError("saved publication belongs to another authority")
        return publication.resume(root, operation_id)
    if action == "preview":
        if command["purpose"] not in ("partner", "document-signing"):
            raise ValueError("unsupported certificate purpose")
        csr = command["csr_pem"]
        if not isinstance(csr, str) or not csr.isascii() or not 0 < len(csr) <= 16384:
            raise ValueError("CSR must be bounded public ASCII material")
        with inventory._read_locked(root):
            found = executor.describe_request(csr.encode("ascii"), material.read(root / "data/ca/ca.crt.pem", 16384))
        if found["root_fingerprint"] != expected_root_fingerprint:
            raise ValueError("CSR preview selected a different authority")
        return {**found, "purpose": command["purpose"]}
    if action == "issue":
        public_state(root, expected_root_fingerprint)
        if command["expected_root_fingerprint"] != expected_root_fingerprint:
            raise ValueError("request names another authority")
        request = {key: command[key] for key in FIELDS["issue"] - {"operation_id"}}
        return issuance.issue(root, command["operation_id"], {**request, "operator_id": actor}, executor)
    if action == "revoke":
        public_state(root, expected_root_fingerprint)
        if command["expected_root_fingerprint"] != expected_root_fingerprint:
            raise ValueError("request names another authority")
        request = {key: command[key] for key in FIELDS["revoke"] - {"operation_id"}}
        return revocation.revoke(root, command["operation_id"], {**request, "operator_id": actor}, executor)
    if action == "resume-revoke":
        public_state(root, expected_root_fingerprint)
        operation_id = issuance.identifier(command["operation_id"])
        record = revocation._load(root / "data/ca/revocations" / operation_id)
        if record["request"]["expected_root_fingerprint"] != expected_root_fingerprint:
            raise ValueError("saved revocation belongs to another authority")
        return revocation.revoke(root, operation_id, record["request"], executor)
    if action == "resume-issue":
        public_state(root, expected_root_fingerprint)
        # The saved request, actor and root remain immutable on recovery.
        operation_id = issuance.identifier(command["operation_id"])
        record = issuance.load(root / "data/ca/operations" / operation_id)
        if record["request"]["expected_root_fingerprint"] != expected_root_fingerprint:
            raise ValueError("saved operation belongs to another authority")
        return issuance.resume(root, operation_id, executor)
    if action == "history":
        return page(history(root), command)
    if action == "certificate":
        certificate = inventory.download_certificate(root, command["serial"], command["fingerprint"], executor)
        facts = executor.describe_certificate(certificate, material.read(root / "data/ca/ca.crt.pem", 16384))
        if facts["root_fingerprint"] != expected_root_fingerprint:
            raise ValueError("certificate belongs to another authority")
        return {"serial": facts["serial"], "fingerprint": facts["fingerprint"],
                "certificate_pem": certificate.decode("ascii")}
    state = public_state(root, expected_root_fingerprint)
    items = inventory.list_certificates(root, executor, now_unix=int(time.time()))
    for item in items:
        if item.get("root_fingerprint", expected_root_fingerprint) != expected_root_fingerprint:
            raise ValueError("inventory contains a different authority")
        item["published_status"] = ("revoked" if item["serial"] in state["revoked_serials"] else
                                    "pending" if item["local_status"] == "revoked" else "not-revoked")
    return page(items, command)
