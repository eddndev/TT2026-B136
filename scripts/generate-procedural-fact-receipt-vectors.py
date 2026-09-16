#!/usr/bin/env python3
"""Independent PFSRC1/PFTXN1 oracle; see docs/procedural-facts-receipts.md."""
import argparse
import copy
import hashlib
import json
import struct
import uuid
from pathlib import Path

KINDS = ["defendant", "victim", "defense_counsel", "prosecutor", "victim_counsel",
         "control_judge", "trial_court", "expert", "police", "precautionary_supervisor", "other"]
ASTRAL = "\U00010000"
MAX_REV = 2**32 - 1


def uid(number):
    return str(uuid.UUID(int=number))


def ident(value):
    return uuid.UUID(value).bytes


def u32(value):
    return struct.pack(">I", value)


def text(value):
    raw = value.encode("utf-8")
    return u32(len(raw)) + raw


def option(value, encode):
    return b"\0" if value is None else b"\1" + encode(value)


def digest(value):
    result = bytes.fromhex(value)
    assert len(result) == 32
    return result


def temporal(value, hearing=False):
    precision = value["precision"]
    if precision == "unknown":
        assert not hearing
        return b"\0"
    tag = ({"date": 0, "instant": 1} if hearing else
           {"date": 1, "minute": 2, "second": 3})[precision]
    year, month, day = map(int, value["date"].split("-"))
    raw = bytes([tag]) + struct.pack(">HBB", year, month, day)
    if precision != "date":
        raw += bytes([value["hour"], value["minute"]])
        if precision != "minute":
            raw += bytes([value["second"]])
    offset = value.get("offset_seconds")
    encode = lambda v: struct.pack(">i", v)
    return raw + (encode(offset) if hearing else option(offset, encode))


def declaration(value, encode):
    if "unknown" in value:
        return b"\0" + text(value["unknown"])
    return b"\1" + encode(value["known"])


def resolution_class(value):
    return {"order": b"\0", "judgment": b"\1"}.get(value["kind"], b"\2") + (
        text(value["label"]) if value["kind"] == "other" else b"")


def source_resolution(value):
    return (ident(value["case"]) + ident(value["id"]) + u32(value["revision"])
            + digest(value["values_digest"]) + digest(value["submission_digest"])
            + bytes([value["status"] == "withdrawn"])
            + declaration(value["class"], resolution_class)
            + declaration(value["issuer"], text) + temporal(value["issued_at"])
            + text(value["summary"]))


def source_participant(value):
    def subject(item):
        return ident(item["id"]) + u32(item["revision"]) + digest(item["values_digest"])
    return (ident(value["case"]) + ident(value["id"]) + u32(value["revision"])
            + digest(value["values_digest"]) + bytes([value["status"] == "archived"])
            + option(value["subject"], subject) + text(value["display_name"])
            + text(value["procedural_role"]) + option(value["organization"], text)
            + option(value["kind"], lambda v: bytes([KINDS.index(v)])))


def source_hearing(value):
    return (ident(value["case"]) + ident(value["hearing_id"]) + ident(value["result_id"])
            + u32(value["revision"]) + option(value["agreement_id"], ident)
            + digest(value["values_digest"]) + digest(value["submission_digest"])
            + bytes([value["status"] == "withdrawn", value["occurrence"] == "not_started"])
            + temporal(value["event_time"], hearing=True) + text(value["summary"])
            + option(value["agreement_text"], text))


def source_support(value):
    return (ident(value["id"]) + u32(value["version"]) + digest(value["digest"])
            + text(value["name"]) + bytes([value["format"] == "docx", 0]))


def source_bytes(value):
    raw = b"PFSRC1" + option(value["resolution"], source_resolution)
    for field, limit, encoder in [("participants", 4, source_participant),
                                  ("hearing_results", 2, source_hearing),
                                  ("direct_supports", 2, source_support)]:
        entries = value[field]
        assert len(entries) <= limit
        raw += u32(len(entries)) + b"".join(encoder(item) for item in entries)
    return raw


def submission_bytes(value):
    action = ["record", "correct", "withdraw"].index(value["action"])
    assert (action == 0 and value["expected_revision"] == 0 and value["reason"] is None) or (
        action > 0 and 0 < value["expected_revision"] < MAX_REV and value["reason"])
    notification = value["family"] == "notification"
    raw = (b"PFTXN1" + ident(value["operation_id"]) + ident(value["actor_id"])
           + ident(value["case_id"]) + bytes([notification]) + ident(value["target_id"]))
    if notification:
        raw += ident(value["parent_id"])
    return (raw + bytes([action]) + u32(value["expected_revision"])
            + digest(value["values_digest"]) + digest(value["sources_digest"])
            + option(value["reason"], text))


def vector(name, value, encoder, field):
    raw = encoder(value)
    return {"name": name, field: value, "length": len(raw), "hex": raw.hex(),
            "sha256": hashlib.sha256(raw).hexdigest()}


def empty():
    return {"resolution": None, "participants": [], "hearing_results": [], "direct_supports": []}


def resolution():
    return {"case": uid(1), "id": uid(2), "revision": 1, "values_digest": "11" * 32,
            "submission_digest": "22" * 32, "status": "recorded",
            "class": {"known": {"kind": "order"}}, "issuer": {"unknown": "Not recorded"},
            "issued_at": {"precision": "unknown"}, "summary": "Declared resolution"}


def participant(number, revision=1, kind=None):
    return {"case": uid(1), "id": uid(number), "revision": revision,
            "values_digest": "33" * 32, "status": "active",
            "subject": None if kind is None else {"id": uid(4), "revision": MAX_REV,
                                                   "values_digest": "44" * 32},
            "display_name": "Name e\u0301 \u00e9", "procedural_role": kind or "Declared role",
            "organization": None, "kind": kind}


def hearing(agreement=None):
    return {"case": uid(1), "hearing_id": uid(0), "result_id": uid(0), "revision": 1,
            "agreement_id": agreement, "values_digest": "55" * 32,
            "submission_digest": "66" * 32, "status": "recorded", "occurrence": "occurred",
            "event_time": {"precision": "date", "date": "2026-09-16", "offset_seconds": -21600},
            "summary": "Session\nStatement", "agreement_text": None if agreement is None else "Agreement"}


def support(version=1):
    return {"id": uid(0), "version": version, "digest": "77" * 32,
            "name": "support.pdf" if version == 1 else "support.docx",
            "format": "pdf" if version == 1 else "docx", "policy": "pdf_docx_v1"}


def source_vectors():
    vectors = []
    def add(name, value):
        vectors.append(vector(name, value, source_bytes, "sources"))
    add("empty", empty())
    full = empty()
    full["resolution"] = resolution()
    full["participants"] = [participant(0), participant(0, 2, "defendant"),
                            participant(1, MAX_REV, "trial_court"), participant(2, 1, "other")]
    full["participants"][1]["status"] = "archived"
    full["participants"][2]["organization"] = "Declared court"
    full["hearing_results"] = [hearing(), hearing(uid(0))]
    full["direct_supports"] = [support(), support(2)]
    add("complete_mixed", full)
    historical = empty()
    entry = hearing()
    entry.update(status="withdrawn", occurrence="not_started")
    entry["event_time"] = {"precision": "instant", "date": "2028-02-29", "hour": 23,
                           "minute": 59, "second": 58, "offset_seconds": 50400}
    historical["hearing_results"] = [entry]
    add("historical_hearing_instant", historical)
    for precision in ["date", "minute", "second"]:
        for offset in [None, 0, -50400]:
            value = empty()
            value["resolution"] = resolution()
            value["resolution"].update(status="withdrawn", issuer={"known": "Issuer"})
            value["resolution"]["class"] = {"known": {"kind": "other", "label": "Other \u00e9"}}
            value["resolution"]["issued_at"] = {
                "precision": precision, "date": "2028-02-29", "hour": 12, "minute": 34,
                "second": 56, "offset_seconds": offset}
            add(f"resolution_{precision}_{offset}", value)
    value = empty()
    value["resolution"] = resolution()
    value["resolution"]["class"] = {"known": {"kind": "judgment"}}
    add("resolution_judgment", value)
    for kind in KINDS:
        value = empty()
        value["participants"] = [participant(0, MAX_REV, kind)]
        add(f"participant_{kind}", value)
    maximum = copy.deepcopy(full)
    maximum["resolution"].update({"class": {"unknown": ASTRAL * 1000},
        "issuer": {"unknown": ASTRAL * 1000}, "summary": ASTRAL * 1000,
        "issued_at": {"precision": "second", "date": "9999-12-31", "hour": 23,
                      "minute": 59, "second": 59, "offset_seconds": 50400}})
    maximum["participants"] = [participant(0, revision) for revision in range(1, 5)]
    for entry in maximum["participants"]:
        entry.update(display_name=ASTRAL * 200, procedural_role=ASTRAL * 80,
                     organization=ASTRAL * 200)
    maximum["hearing_results"] = [hearing(uid(0)), hearing(uid(1))]
    for entry in maximum["hearing_results"]:
        entry.update(summary=ASTRAL * 1000, agreement_text=ASTRAL * 1000,
                     event_time={"precision": "instant", "date": "0001-01-01",
                                 "hour": 0, "minute": 0, "second": 0, "offset_seconds": -50400})
    for entry in maximum["direct_supports"]:
        entry["name"] = "a" * 128
    add("maximum_utf8", maximum)
    assert vectors[0]["length"] == 19
    assert vectors[-1]["length"] == 36847
    return vectors


def submission(family="resolution", action="record"):
    return {"family": family, "operation_id": uid(0), "actor_id": uid(0),
            "case_id": uid(0), "target_id": uid(0),
            "parent_id": uid(0) if family == "notification" else None,
            "action": action, "expected_revision": 0 if action == "record" else 1,
            "values_digest": "00" * 32, "sources_digest": "ff" * 32,
            "reason": None if action == "record" else "Correction\n\u00e9 e\u0301"}


def submission_vectors():
    vectors = []
    def add(name, value):
        vectors.append(vector(name, value, submission_bytes, "submission"))
    for family in ["resolution", "notification"]:
        for action in ["record", "correct", "withdraw"]:
            add(f"{family}_{action}_nil", submission(family, action))
        value = submission(family, "correct")
        value.update(expected_revision=MAX_REV - 1, reason=ASTRAL * 1000)
        add(f"{family}_maximum", value)
        assert vectors[-1]["length"] == (4145 if family == "resolution" else 4161)
    base = submission("notification", "correct")
    for field in ["operation_id", "actor_id", "case_id", "target_id", "parent_id",
                  "action", "expected_revision", "values_digest", "sources_digest", "reason"]:
        value = copy.deepcopy(base)
        value[field] = (uid(1) if field.endswith("_id") else {
            "action": "withdraw", "expected_revision": 2, "values_digest": "01" * 32,
            "sources_digest": "01" * 32, "reason": "Different reason"}.get(field))
        add(f"notification_changed_{field}", value)
        assert vectors[-1]["hex"] != submission_bytes(base).hex()
    assert vectors[0]["length"] == 141 and vectors[4]["length"] == 157
    return vectors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    payload = {"format": "procedural-fact-receipts-v1", "sources": source_vectors(),
               "submissions": submission_vectors()}
    output = Path(__file__).resolve().parents[1] / "crates/application/tests/fixtures/procedural-fact-receipts.json"
    data = (json.dumps(payload, indent=2, ensure_ascii=True) + "\n").encode("ascii")
    if args.check:
        assert output.read_bytes() == data, "Fixture differs from the independent generator"
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(data)
    print(json.dumps({"sources": len(payload["sources"]), "submissions": len(payload["submissions"]),
                      "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)}))


if __name__ == "__main__":
    main()
