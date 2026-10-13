"""Bridge one portal request to the existing recoverable CRL maintenance flow."""
import json
import os
from pathlib import Path
import stat
import time

import ca_mutation_guard
import crl_journal as journal
import issuance
import issuance_material as files
import renew_crl
import restore_fence


FORMAT = "qadra-ca-publication-v1"
REQUEST_FIELDS = {"operator_id", "expected_revision", "acknowledge_maintenance", "expected_root_fingerprint"}
BASELINE_FILES = {*files.LIMITS, "crl/crl.pem"}
STAGED_FILES = ("index.txt", "serial", "crlnumber", "ca.crt.pem", "index.txt.attr")


def _request(value):
    if not isinstance(value, dict) or set(value) != REQUEST_FIELDS:
        raise ValueError("CA publication request fields differ")
    result = dict(value)
    issuance.identifier(result["operator_id"])
    if (type(result["expected_revision"]) is not int or not 1 <= result["expected_revision"] < 2**32 - 1
            or result["acknowledge_maintenance"] is not True
            or not files.hex_digest(result["expected_root_fingerprint"])):
        raise ValueError("CA publication revision, root or maintenance acknowledgement is invalid")
    return result


def _digest(value):
    return files.digest(json.dumps(value, sort_keys=True, ensure_ascii=True).encode("ascii"))


def _baseline(root, head):
    ca = root / "data/ca"
    observed = files.snapshot(ca)["files"]
    if any(observed[name] is not None for name in observed if name.endswith(".new")):
        raise RuntimeError("CA has unfinished material before CRL preparation")
    observed["crl/crl.pem"] = files.digest(files.read(ca / "crl/crl.pem", 1048576))
    return {"files": observed, "head_digest": _digest(head), "counter": renew_crl.counter_value(ca / "crlnumber")}


def _locations(root, identifier):
    ca = root / "data/ca"
    for path in (root / "data", ca):
        files.directory(path)
    operation = ca / "publications" / identifier
    for path in (operation.parent, operation):
        if not files.absent(path):
            files.directory(path)
    return operation, journal.operation(root, identifier)


def _guard(root, identifier):
    restore_fence.guard(root)
    ca_mutation_guard.guard(root, allowed=("publications", identifier))
    if journal.pending(root) and journal.load(journal.fence_path(root)).get("operation_id") != identifier:
        raise RuntimeError("another CRL maintenance operation holds the fence")


def _load(operation):
    path = operation / "journal.json"
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) != 0o600:
        raise ValueError("CA publication journal must be a private regular file")
    record = json.loads(journal.read(path, 65536), object_pairs_hook=issuance.unique_object)
    if (not isinstance(record, dict)
            or set(record) != {"format", "operation_id", "request", "recorded_at", "release", "baseline", "receipt"}
            or record["format"] != FORMAT or issuance.identifier(record["operation_id"]) != operation.name
            or type(record["recorded_at"]) is not int or record["recorded_at"] <= 0
            or not isinstance(record["release"], dict)):
        raise ValueError("CA publication journal identity or format differs")
    _request(record["request"])
    baseline = record["baseline"]
    if (not isinstance(baseline, dict) or set(baseline) != {"files", "head_digest", "counter"}
            or not files.hex_digest(baseline["head_digest"])
            or type(baseline["counter"]) is not int or not 0 < baseline["counter"] < 2**64 - 1
            or not isinstance(baseline["files"], dict) or set(baseline["files"]) != BASELINE_FILES
            or any(not files.hex_digest(value) and not (name in files.OPTIONAL and value is None)
                   for name, value in baseline["files"].items())
            or any(baseline["files"][name] is not None for name in files.LIMITS if name.endswith(".new"))):
        raise ValueError("CA publication baseline is invalid")
    return record


def _receipt(record):
    request = record["request"]
    return {"operation_id": record["operation_id"], "operator_id": request["operator_id"],
            "recorded_at": record["recorded_at"], "expected_revision": request["expected_revision"],
            "revision": request["expected_revision"] + 1, "crl_number": record["baseline"]["counter"],
            "root_fingerprint": request["expected_root_fingerprint"], "state": "published"}


def _completed(record):
    if record["receipt"] is None:
        return None
    expected = _receipt(record)
    if record["receipt"] != expected:
        raise ValueError("CA publication receipt differs from its request")
    return expected


def _pristine(root, record):
    head = renew_crl.material.read_head(root)
    if _baseline(root, head) != record["baseline"]:
        raise RuntimeError("CRL preparation baseline changed; automatic regeneration is unavailable")
    return head


def _recover_prepared(root, operation, record):
    """Reconstruct only a complete staged candidate, never modify partial OpenSSL state."""
    before = _pristine(root, record)
    ca, staged = root / "data/ca", operation / "ca"
    files.directory(operation)
    files.directory(staged)
    for name in STAGED_FILES:
        expected = record["baseline"]["files"][name]
        if name == "crlnumber":
            continue
        if expected is None:
            if not files.absent(staged / name):
                raise ValueError("staged CA material differs from its baseline")
        elif files.digest(files.read(staged / name, files.LIMITS[name], empty=name == "index.txt")) != expected:
            raise ValueError("staged CA material differs from its baseline")
    for name in files.LIMITS:
        if name.endswith(".new") and not files.absent(staged / name):
            raise RuntimeError("staged CRL material is incomplete")
    counter = record["baseline"]["counter"]
    next_counter = renew_crl.counter_value(staged / "crlnumber")
    if (next_counter != counter + 1
            or files.digest(files.read(staged / "crlnumber.old", 64)) != record["baseline"]["files"]["crlnumber"]):
        raise ValueError("staged CRL counter is partial or differs from its baseline")
    original_path, candidate_path = operation / "original-crl.pem", operation / "candidate.pem"
    if files.digest(files.read(original_path, 1048576)) != record["baseline"]["files"]["crl/crl.pem"]:
        raise ValueError("saved original CRL differs from its baseline")
    files.read(candidate_path, 1048576)
    original = renew_crl.material.inspect(ca / "ca.crt.pem", original_path)
    candidate = renew_crl.material.inspect(ca / "ca.crt.pem", candidate_path)
    if not renew_crl.same_material(before, original) or candidate["crl_number"] != counter:
        raise ValueError("staged candidate differs from its exact baseline or reserved number")
    renew_crl.validate_candidate(original, candidate, next_counter)
    # Persist already-verified staging before admitting its recovered journal.
    for path in [original_path, candidate_path, staged / "crlnumber.old"] + [staged / name for name in STAGED_FILES]:
        if files.absent(path):
            continue
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        try:
            if not stat.S_ISREG(os.fstat(descriptor).st_mode):
                raise ValueError("staged CA material changed its file type")
            os.fsync(descriptor)
        finally:
            os.close(descriptor)
    journal.sync_directory(staged)
    journal.sync_directory(operation)
    maintained = {"operation_id": operation.name, "release": record["release"], "before": before,
                  "original": original, "candidate": candidate, "counter": counter,
                  "next_counter": next_counter, "state": "prepared", "backup": None}
    journal.save(operation / "journal.json", maintained)
    return maintained


def _bind(record, maintained):
    if (maintained["operation_id"] != record["operation_id"] or maintained["release"] != record["release"]
            or _digest(maintained["before"]) != record["baseline"]["head_digest"]
            or maintained["counter"] != record["baseline"]["counter"]
            or maintained["next_counter"] != maintained["counter"] + 1
            or maintained["candidate"]["crl_number"] != maintained["counter"]):
        raise ValueError("CRL maintenance journal differs from its portal request")


def _advance(root, operation, maintenance, record):
    completed = _completed(record)
    if completed is not None:
        return completed
    target, info = renew_crl.current_release(root)
    if info != record["release"]:
        raise ValueError("active release differs from the saved publication")
    if not files.absent(maintenance / "journal.json"):
        maintained = journal.load(maintenance / "journal.json")
    elif files.absent(maintenance):
        _pristine(root, record)
        _, maintained = renew_crl.prepared(root, target, info, record["request"]["expected_revision"],
                                          identifier=record["operation_id"])
    else:
        maintained = _recover_prepared(root, maintenance, record)
    _bind(record, maintained)
    if maintained["state"] != "accepted" or journal.pending(root):
        renew_crl.continue_operation(root, maintenance, maintained, target, renew_crl.Runtime(root))
        maintained = journal.load(maintenance / "journal.json")
        _bind(record, maintained)
    committed, expected = renew_crl.reconcile(root, maintenance, maintained, target)
    if (maintained["state"] != "accepted" or not committed or journal.pending(root)
            or expected["revision"] != record["request"]["expected_revision"] + 1
            or expected["root_fingerprint"] != record["request"]["expected_root_fingerprint"]):
        raise RuntimeError("CRL publication has not accepted its exact successor trust")
    record["receipt"] = _receipt(record)
    journal.save(operation / "journal.json", record)
    return dict(record["receipt"])


def publish(root, operation_id, request):
    """Publish once using the existing maintenance UUID and its complete workflow."""
    identifier, request = issuance.identifier(operation_id), _request(request)
    root = Path(root).resolve(strict=True)
    with journal.locked(root):
        _guard(root, identifier)
        operation, maintenance = _locations(root, identifier)
        if not files.absent(operation):
            record = _load(operation)
            if record["request"] != request:
                raise ValueError("CA operation already belongs to different publication parameters")
        else:
            if not files.absent(maintenance) or journal.pending(root):
                raise ValueError("maintenance identity already exists without this portal request")
            _, info = renew_crl.current_release(root)
            head = renew_crl.material.read_head(root)
            if (head["revision"] != request["expected_revision"]
                    or head["root_fingerprint"] != request["expected_root_fingerprint"]):
                raise ValueError("expected credential trust revision or root differs")
            baseline = _baseline(root, head)
            if not head["crl_number"] < baseline["counter"] < 2**64 - 1:
                raise ValueError("CA CRL counter is stale or exhausted")
            original = renew_crl.material.inspect(root / "data/ca/ca.crt.pem", root / "data/ca/crl/crl.pem")
            if not renew_crl.same_material(head, original):
                raise ValueError("installed credentials differ from persisted trust")
            record = {"format": FORMAT, "operation_id": identifier, "request": request,
                      "recorded_at": int(time.time()), "release": info, "baseline": baseline, "receipt": None}
            journal.directory(operation.parent)
            journal.directory(operation)
            journal.save(operation / "journal.json", record)
        return _advance(root, operation, maintenance, record)


def resume(root, operation_id):
    """Resume the immutable portal request without another candidate or rollback."""
    identifier = issuance.identifier(operation_id)
    root = Path(root).resolve(strict=True)
    with journal.locked(root):
        _guard(root, identifier)
        operation, maintenance = _locations(root, identifier)
        return _advance(root, operation, maintenance, _load(operation))
