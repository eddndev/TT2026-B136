"""Renew the current CA CRL and resume uncertain publication without rollback."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
import time
import uuid

import crl_backup
import crl_journal as journal
import crl_material as material
from release import linked, metadata
from runtime import Runtime


MATERIAL_FIELDS = ("root_der", "crl_der", "root_fingerprint", "crl_digest", "crl_number",
                   "crl_this_update", "crl_next_update", "valid_from", "valid_until")


def same_material(left, right):
    return all(left.get(key) == right.get(key) for key in MATERIAL_FIELDS)


def same_head(left, right):
    return (same_material(left, right) and left.get("deployment_id") == right.get("deployment_id")
            and left.get("revision") == right.get("revision"))


def current_release(root):
    target = linked(root, "current")
    if target is None:
        raise ValueError("CRL maintenance requires an active release")
    info = metadata(target)
    if (root / "config/schema").read_text().strip() != info["schema"]:
        raise ValueError("active release schema differs from the initialized database")
    return target, info


def counter_value(path):
    value = journal.read(path, 64).decode("ascii").strip()
    if not re.fullmatch(r"[0-9A-Fa-f]{1,16}", value):
        raise ValueError("invalid CA CRL counter")
    return int(value, 16)


def validate_candidate(before, candidate, next_counter):
    now = int(time.time())
    for name, digest in (("root_der", "root_fingerprint"), ("crl_der", "crl_digest")):
        if hashlib.sha256(bytes.fromhex(candidate[name])).hexdigest() != candidate[digest]:
            raise ValueError("candidate credential digest differs")
    if (candidate["root_der"] != before["root_der"]
            or candidate["root_fingerprint"] != before["root_fingerprint"]):
        raise ValueError("candidate changes the original CA")
    number = candidate["crl_number"]
    if (type(number) is not int or not before["crl_number"] < number < 2**64 - 1
            or next_counter != number + 1):
        raise ValueError("candidate CRL number must increase exactly once from its counter")
    if (not before["crl_this_update"] <= candidate["crl_this_update"] <= now
            or not candidate["crl_this_update"] < candidate["crl_next_update"]
            or not candidate["crl_this_update"] <= candidate["valid_from"] <= now
            < candidate["valid_until"] <= candidate["crl_next_update"]):
        raise ValueError("candidate CRL is not currently valid or regresses its issue time")
    if not set(before["revoked_serials"]).issubset(candidate["revoked_serials"]):
        raise ValueError("candidate removes an existing revocation")


def prepared(root, target, info, expected):
    before = material.read_head(root)
    if type(expected) is not int or not 1 <= expected < 2**32 - 1 or before["revision"] != expected:
        raise ValueError("expected credential trust revision differs")
    ca = root / "data/ca"
    original = material.inspect(ca / "ca.crt.pem", ca / "crl/crl.pem")
    if not same_material(before, original):
        raise ValueError("installed CA and CRL differ from persisted trust")
    counter = counter_value(ca / "crlnumber")
    if not before["crl_number"] < counter < 2**64 - 1:
        raise ValueError("CA CRL counter is stale or exhausted")
    for directory in (root / "maintenance", root / "maintenance/crl"):
        journal.directory(directory)
    identifier = str(uuid.uuid4())
    operation = journal.operation(root, identifier)
    journal.directory(operation)
    journal.write(operation / "original-crl.pem", journal.read(ca / "crl/crl.pem", 1048576))
    generated = material.generate(root, target, operation)
    candidate_path = Path(generated["path"])
    if candidate_path != operation / "candidate.pem" or candidate_path.is_symlink():
        raise ValueError("candidate must remain inside its private operation")
    candidate = material.inspect(ca / "ca.crt.pem", candidate_path)
    next_counter = int(generated["next_counter"], 16)
    validate_candidate(original, candidate, next_counter)
    if candidate["crl_number"] != counter:
        raise ValueError("candidate number differs from the saved CA counter")
    record = {"operation_id": identifier, "release": info, "before": before,
              "original": original, "candidate": candidate, "counter": counter,
              "next_counter": next_counter, "state": "prepared", "backup": None}
    journal.save(operation / "journal.json", record)
    return operation, record


def persist(operation, record, state):
    record["state"] = state
    journal.save(operation / "journal.json", record)


def reconcile(root, operation, record, target):
    if record["operation_id"] != operation.name or record["release"] != metadata(target):
        raise ValueError("maintenance identity or active release differs")
    ca = root / "data/ca"
    original = material.inspect(ca / "ca.crt.pem", operation / "original-crl.pem")
    candidate = material.inspect(ca / "ca.crt.pem", operation / "candidate.pem")
    if original != record["original"] or candidate != record["candidate"]:
        raise ValueError("saved credential material differs from the journal")
    validate_candidate(original, candidate, record["next_counter"])
    before = record["before"]
    if not same_material(before, original):
        raise ValueError("original credential material differs from persisted baseline")
    expected = {**candidate, "deployment_id": before["deployment_id"], "revision": before["revision"] + 1}
    head = material.read_head(root)
    committed = same_head(head, expected)
    if not committed and not same_head(head, before):
        raise ValueError("persisted trust does not belong to this maintenance operation")
    installed = material.inspect(ca / "ca.crt.pem", ca / "crl/crl.pem")
    counter = counter_value(ca / "crlnumber")
    if committed:
        if not same_material(installed, candidate) or counter != record["next_counter"]:
            raise ValueError("committed trust differs from installed CRL or counter")
    elif (not (same_material(installed, original) or same_material(installed, candidate))
          or counter not in (record["counter"], record["next_counter"])):
        raise ValueError("uncommitted maintenance material was modified")
    return committed, expected


def continue_operation(root, operation, record, target, runtime):
    identifier = operation.name
    try:
        if record["backup"]:
            journal.fenced(root, identifier)
        runtime.stop()
        committed, expected = reconcile(root, operation, record, target)
        if not record["backup"]:
            ca = root / "data/ca"
            installed = material.inspect(ca / "ca.crt.pem", ca / "crl/crl.pem")
            if (committed or not same_material(installed, record["original"])
                    or counter_value(ca / "crlnumber") != record["counter"]):
                raise ValueError("original maintenance state is required before capturing its backup")
            # The snapshot must not restore a fence whose journal lives outside its archive.
            if journal.pending(root):
                journal.unfence(root)
            backup = runtime.backup()
            record["backup"] = crl_backup.capture(root, backup)
            persist(operation, record, "backed_up")
            journal.fenced(root, identifier)
            runtime.stop()
            committed, expected = reconcile(root, operation, record, target)
        else:
            crl_backup.validate(root, record["backup"])
        if not committed:
            ca = root / "data/ca"
            journal.write(ca / "crlnumber", (format(record["next_counter"], "X") + "\n").encode("ascii"))
            journal.write(ca / "crl/crl.pem", journal.read(operation / "candidate.pem", 1048576))
            persist(operation, record, "publishing")
            material.publish(root, target, operation / "candidate.pem", record["before"]["revision"])
            committed, expected = reconcile(root, operation, record, target)
            if not committed:
                raise ValueError("publication did not advance the exact persisted trust")
        persist(operation, record, "committed")
        journal.unfence(root)
        runtime.start(target)
        runtime.check(target)
        persist(operation, record, "accepted")
        return {key: expected[key] for key in ("deployment_id", "revision", "crl_number")}
    except BaseException:
        # SQL publication is immutable. Never restore an earlier CRL or counter.
        try:
            journal.fenced(root, identifier)
        finally:
            runtime.stop()
        raise


def renew(root, expected_revision, runtime=None):
    root = Path(root).resolve(strict=True)
    with journal.locked(root):
        journal.guard(root)
        target, info = current_release(root)
        operation, record = prepared(root, target, info, expected_revision)
        return continue_operation(root, operation, record, target, runtime or Runtime(root))


def resume(root, operation_id, runtime=None):
    root = Path(root).resolve(strict=True)
    with journal.locked(root):
        operation = journal.operation(root, operation_id)
        if journal.pending(root) and journal.load(journal.fence_path(root)).get("operation_id") != operation_id:
            raise ValueError("another CRL maintenance operation holds the fence")
        target, _ = current_release(root)
        record = journal.load(operation / "journal.json")
        if record["state"] == "accepted" and not journal.pending(root):
            raise ValueError("maintenance operation is already accepted")
        return continue_operation(root, operation, record, target, runtime or Runtime(root))


def status(root):
    root = Path(root).resolve(strict=True)
    with journal.locked(root):
        head = material.read_head(root)
        result = {key: head[key] for key in ("deployment_id", "revision", "crl_number")}
        checked_at = int(time.time())
        result.update(checked_at=checked_at, valid_from=head["valid_from"],
                      valid_until=head["valid_until"],
                      expires_in_seconds=max(0, head["valid_until"] - checked_at),
                      valid_now=head["valid_from"] <= checked_at <= head["valid_until"])
        result["maintenance_pending"] = journal.pending(root)
        if result["maintenance_pending"]:
            result["operation_id"] = journal.load(journal.fence_path(root))["operation_id"]
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    commands = parser.add_subparsers(dest="command", required=True)
    renew_command = commands.add_parser("renew")
    renew_command.add_argument("--expected-revision", type=int, required=True)
    resume_command = commands.add_parser("resume")
    resume_command.add_argument("operation_id")
    commands.add_parser("status")
    args = parser.parse_args()
    if args.command == "renew":
        result = renew(args.root, args.expected_revision)
    elif args.command == "resume":
        result = resume(args.root, args.operation_id)
    else:
        result = status(args.root)
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except Exception:
        # External command errors can contain environment-specific private details.
        print("CRL maintenance failed; inspect its private journal before resuming", file=sys.stderr)
        sys.exit(1)
