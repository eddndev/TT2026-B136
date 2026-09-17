"""Independent PFRES1/PFNOT1 vectors; no Rust code or encoder is invoked."""
from copy import deepcopy
from datetime import date, datetime, timedelta
from hashlib import sha256
from pathlib import Path
from uuid import UUID
import json
import struct


LABEL_KEYS = {"label", "locator", "subtype", "value"}
TEXT_KEYS = {"reason", "note", "reference", "description", "scope", "summary", "statement"}
CATALOGS = {
    "class": ["order", "judgment", "other"],
    "character": ["personal", "publication", "other"],
    "medium": ["in_person", "electronic", "other"],
    "context": ["in_hearing", "outside_hearing", "other"],
    "outcome": ["practiced", "attempted"],
}


def normalize_text(value, limit, multiline):
    value = value.replace("\r\n", "\n") if multiline else value
    assert not any((ord(c) < 32 or 127 <= ord(c) <= 159)
                   and not (multiline and c == "\n") for c in value)
    value = value.strip()
    assert 1 <= len(value) <= limit
    assert not any(0xD800 <= ord(c) <= 0xDFFF for c in value)
    return value


def normalize(value, key=None):
    if isinstance(value, dict):
        return {k: normalize(v, k) for k, v in value.items()}
    if isinstance(value, list):
        return [normalize(v) for v in value]
    if isinstance(value, str) and key in LABEL_KEYS | TEXT_KEYS:
        return normalize_text(value, 200 if key in LABEL_KEYS else 1000, key in TEXT_KEYS)
    return value


def text(value):
    raw = value.encode("utf-8")
    return struct.pack(">I", len(raw)) + raw


def optional(value, encode):
    return b"\0" if value is None else b"\1" + encode(value)


def reference(value, key="id"):
    assert 1 <= value["revision"] <= 0xFFFFFFFF
    return UUID(value[key]).bytes + struct.pack(">I", value["revision"])


def declaration(value, encode):
    if value["kind"] == "unknown":
        return b"\0" + text(value["reason"])
    assert value["kind"] == "known"
    return b"\1" + encode(value["value"])


def catalog(value, field):
    tag = CATALOGS[field].index(value["kind"])
    return bytes([tag]) + (text(value["label"]) if value["kind"] == "other" else b"")


def declared_time(value):
    tag = ["unknown", "date", "minute", "second"].index(value["precision"])
    if tag == 0:
        assert value == {"precision": "unknown"}
        return b"\0"
    day = date(value["year"], value["month"], value["day"])
    hour, minute, second = (value.get(k, 0) for k in ["hour", "minute", "second"])
    start = datetime(day.year, day.month, day.day, hour, minute, second)
    end = start.replace(hour=23, minute=59, second=59) if tag == 1 else start
    if tag == 2:
        end = start.replace(second=59)
    offset = value["offset_seconds"]
    if offset is not None:
        assert offset % 60 == 0 and abs(offset) <= 14 * 3600
        for endpoint in [start, end]:
            assert 1 <= (endpoint - timedelta(seconds=offset)).year <= 9999
    body = bytes([tag]) + struct.pack(">HBB", day.year, day.month, day.day)
    if tag >= 2:
        body += bytes([hour, minute])
    if tag == 3:
        body += bytes([second])
    return body + optional(offset, lambda v: struct.pack(">i", v))


def person(value):
    if value["kind"] == "participant":
        return b"\0" + reference(value)
    assert value["kind"] == "unlinked"
    return b"\1" + text(value["label"]) + text(value["description"])


def evidence(value):
    assert 1 <= value["version"] <= 0xFFFFFFFF
    digest = bytes.fromhex(value["digest"])
    assert len(digest) == 32
    return (UUID(value["document_id"]).bytes + struct.pack(">I", value["version"])
            + digest + text(value["locator"]))


def provenance(value):
    if value["kind"] == "operator_note":
        return b"\0" + text(value["note"])
    if value["kind"] == "external_reference":
        body = b"\1" + text(value["reference"])
    else:
        assert value["kind"] == "hearing_result"
        ref = value["reference"]
        body = (b"\2" + UUID(ref["hearing_id"]).bytes + reference(ref, "result_id")
                + optional(ref["agreement_id"], lambda v: UUID(v).bytes)
                + text(value["locator"]))
    return body + optional(value["support"], evidence)


def representation(value):
    if value["kind"] == "not_recorded":
        return b"\0" + text(value["reason"])
    assert value["kind"] == "declared"
    return (b"\1" + person(value["represented"]) + person(value["representative"])
            + text(value["scope"]) + provenance(value["provenance"]))


def stated_effect(value):
    return declared_time(value["at"]) + text(value["statement"]) + text(value["locator"])


def direct_supports(value):
    sources = [value["provenance"]]
    if value.get("representation", {}).get("kind") == "declared":
        sources.append(value["representation"]["provenance"])
    unique = {}
    for source in sources:
        if source.get("support") is None:
            continue
        support = source["support"]
        key = (support["document_id"], support["version"])
        record = {k: support[k] for k in ["document_id", "version", "digest"]}
        assert key not in unique or unique[key] == record
        unique[key] = record
    assert len(unique) <= 2
    return list(unique.values())


def encode(family, value):
    direct_supports(value)
    if family == "resolution":
        return (b"PFRES1" + declaration(value["class"], lambda v: catalog(v, "class"))
                + optional(value["subtype"], text) + declaration(value["issuer"], text)
                + declared_time(value["issued_at"]) + text(value["summary"])
                + provenance(value["provenance"]))
    assert family == "notification"
    body = b"PFNOT1" + reference(value["resolution"])
    for field in ["character", "medium", "context", "outcome"]:
        body += declaration(value[field], lambda v: catalog(v, field))
    return (body + optional(value["subtype"], text) + declared_time(value["practiced_at"])
            + optional(value["received_at"], declared_time)
            + optional(value["stated_effect"], stated_effect)
            + declaration(value["intended_recipient"], person)
            + declaration(value["actual_receiver"], person)
            + representation(value["representation"]) + text(value["summary"])
            + provenance(value["provenance"]))


def known(value):
    return {"kind": "known", "value": value}


def unknown(reason="x"):
    return {"kind": "unknown", "reason": reason}


def stamp(precision, offset=None, year=2026, month=9, day=16):
    if precision == "unknown":
        return {"precision": precision}
    value = dict(precision=precision, year=year, month=month, day=day, offset_seconds=offset)
    if precision in ["minute", "second"]:
        value.update(hour=12, minute=34)
    if precision == "second":
        value["second"] = 56
    return value


def participant(number, revision=1):
    return dict(kind="participant", id=str(UUID(int=number)), revision=revision)


def support(number=1, version=1, locator="p. 1"):
    return dict(document_id=str(UUID(int=number)), version=version,
                digest=bytes(range(32)).hex(), locator=locator)


def external(file=None, reference_text="External record"):
    return dict(kind="external_reference", reference=reference_text, support=file)


def hearing(file=None, agreement=None):
    return dict(kind="hearing_result", reference=dict(hearing_id=str(UUID(int=2)),
                result_id=str(UUID(int=3)), revision=0xFFFFFFFF, agreement_id=agreement),
                locator="Agreement 2", support=file)


def row(name, family, value):
    normalized = normalize(value)
    wire = encode(family, normalized)
    assert normalized == normalize(normalized)
    return dict(name=name, family=family, input=value, normalized=normalized,
                bytes=len(wire), hex=wire.hex(), sha256=sha256(wire).hexdigest(),
                direct_supports=direct_supports(normalized))


def rejects(call):
    try:
        call()
    except (AssertionError, ValueError, OverflowError):
        return
    raise AssertionError("expected an independently invalid input")


def build_vectors():
    res = dict(class_=known({"kind": "order"}), subtype=None, issuer=known("x"),
               issued_at=stamp("unknown"), summary="x", provenance=dict(kind="operator_note", note="x"))
    res["class"] = res.pop("class_")
    rows = [row("resolution_minimum", "resolution", res)]
    explicit = deepcopy(res)
    explicit.update({"class": unknown(), "issuer": unknown(), "provenance": external()})
    rows.append(row("resolution_unknown_declarations", "resolution", explicit))
    for precision in ["date", "minute", "second"]:
        for offset in [None, 0, 19800, -50400]:
            value = deepcopy(res)
            value["issued_at"] = stamp(precision, offset)
            suffix = "absent" if offset is None else str(offset).replace("-", "minus")
            rows.append(row("resolution_" + precision + "_offset_" + suffix, "resolution", value))
    rich = deepcopy(res)
    rich.update({"class": known({"kind": "judgment"}), "subtype": "  Declared subtype  ",
                 "issuer": known("\u2003Declared issuer\u2003"), "summary": "  Summary\r\nSecond line  ",
                 "provenance": hearing(support(), str(UUID(int=4)))})
    rows.append(row("resolution_hearing_agreement_support_normalized", "resolution", rich))
    rich = deepcopy(rich)
    rich.update({"class": known({"kind": "other", "label": "Other class"}), "provenance": hearing()})
    rows.append(row("resolution_hearing_without_agreement_or_support", "resolution", rich))
    for year, month, day, hour, minute, second in [(1, 1, 1, 0, 0, 0), (9999, 12, 31, 23, 59, 59)]:
        value = deepcopy(res)
        value["issued_at"] = stamp("second", 0, year, month, day)
        value["issued_at"].update(hour=hour, minute=minute, second=second)
        rows.append(row("resolution_year_" + str(year), "resolution", value))
    for label, summary in [("composed", "\u00e9"), ("decomposed", "e\u0301")]:
        value = deepcopy(res)
        value["summary"] = summary
        rows.append(row("resolution_unicode_" + label, "resolution", value))
    notification = dict(resolution=dict(id=str(UUID(int=0)), revision=1),
        character=known({"kind": "personal"}), medium=known({"kind": "in_person"}),
        context=known({"kind": "in_hearing"}), outcome=known({"kind": "practiced"}),
        subtype=None, practiced_at=stamp("unknown"), received_at=None, stated_effect=None,
        intended_recipient=unknown(), actual_receiver=unknown(),
        representation=dict(kind="not_recorded", reason="x"), summary="x",
        provenance=dict(kind="operator_note", note="x"))
    rows.append(row("notification_minimum", "notification", notification))
    value = deepcopy(notification)
    for field in ["character", "medium", "context", "outcome"]:
        value[field] = unknown("Not declared")
    value.update(received_at=stamp("unknown"), stated_effect=dict(at=stamp("unknown"), statement="Effect stated", locator="p. 3"))
    rows.append(row("notification_explicit_unknown_receipt_effect", "notification", value))
    people = deepcopy(notification)
    for field in ["character", "medium", "context", "outcome"]:
        people[field] = known({"kind": CATALOGS[field][1]})
    people.update(subtype="Declared subtype", practiced_at=stamp("minute", -21600),
        received_at=stamp("second"), intended_recipient=known(participant(9, 0xFFFFFFFF)),
        actual_receiver=known(dict(kind="unlinked", label="Receiver", description="Declared identity")),
        representation=dict(kind="declared", represented=participant(9, 7),
            representative=participant(8, 3), scope="Declared scope", provenance=external(support(locator="p. 2"))),
        provenance=external(support(locator="p. 1")))
    rows.append(row("notification_shared_support_distinct_locators", "notification", people))
    swapped = deepcopy(people)
    swapped["actual_receiver"], swapped["intended_recipient"] = swapped["intended_recipient"], swapped["actual_receiver"]
    rows.append(row("notification_swapped_person_functions", "notification", swapped))
    versions = deepcopy(people)
    versions["representation"]["provenance"]["support"]["version"] = 2
    rows.append(row("notification_two_versions_same_document", "notification", versions))
    others = deepcopy(people)
    for field in ["character", "medium", "context"]:
        others[field] = known(dict(kind="other", label="Other " + field))
    others["provenance"] = hearing(support())
    others["representation"]["provenance"] = dict(kind="operator_note", note="Relationship declared")
    rows.append(row("notification_other_catalogs_hearing", "notification", others))
    wide = chr(0x1F642)
    label, long = wide * 200, wide * 1000
    max_source = external(support(1, 0xFFFFFFFF, label), long)
    max_res = deepcopy(res)
    max_res.update({"class": unknown(long), "subtype": label, "issuer": unknown(long),
                    "issued_at": stamp("second", 50400), "summary": long, "provenance": max_source})
    rows.append(row("resolution_maximum_utf8", "resolution", max_res))
    max_not = deepcopy(notification)
    for field in ["character", "medium", "context", "outcome"]:
        max_not[field] = unknown(long)
    unlinked = dict(kind="unlinked", label=label, description=long)
    max_not.update(resolution=dict(id=str(UUID(int=(1 << 128) - 1)), revision=0xFFFFFFFF),
        subtype=label, practiced_at=stamp("second", -50400), received_at=stamp("second", 0),
        stated_effect=dict(at=stamp("second", 50400), statement=long, locator=label),
        intended_recipient=known(unlinked), actual_receiver=known(unlinked),
        representation=dict(kind="declared", represented=unlinked, representative=unlinked,
            scope=long, provenance=external(support(2, 0xFFFFFFFF, label), long)),
        summary=long, provenance=max_source)
    rows.append(row("notification_maximum_utf8", "notification", max_not))
    assert len(encode("resolution", res)) == 6 + 2 + 1 + 6 + 1 + 5 + 6 == 27
    assert len(encode("notification", notification)) == 6 + 20 + 8 + 4 + 12 + 6 + 5 + 6 == 67
    assert len(encode("resolution", max_res)) == 6 + 4005 + 805 + 4005 + 13 + 4004 + 4862 == 17700
    assert len(encode("notification", max_not)) == 6 + 20 + 16020 + 805 + 13 + 14 + 4822 + 9620 + 18485 + 4004 + 4862 == 58671
    assert len(direct_supports(people)) == 1 and len(direct_supports(versions)) == 2
    assert encode("notification", people) != encode("notification", swapped)
    conflict = deepcopy(people)
    conflict["representation"]["provenance"]["support"]["digest"] = "ff" * 32
    rejects(lambda: direct_supports(conflict))
    for invalid in ["\tx", "x\ry", "x\x00", "\n"]:
        rejects(lambda: normalize_text(invalid, 1000, True))
    rejects(lambda: normalize_text("x\ny", 200, False))
    rejects(lambda: normalize_text(wide * 201, 200, False))
    rejects(lambda: normalize_text(wide * 1001, 1000, True))
    rejects(lambda: declared_time(stamp("date", 60, 1, 1, 1)))
    rejects(lambda: declared_time(stamp("date", -60, 9999, 12, 31)))
    indexed = {item["name"]: item for item in rows}
    assert indexed["resolution_unicode_composed"]["hex"] != indexed["resolution_unicode_decomposed"]["hex"]
    assert normalize_text("  a  b\r\nc  ", 1000, True) == "a  b\nc"
    for precision, size in [("date", 6), ("minute", 8), ("second", 9)]:
        absent, utc = declared_time(stamp(precision)), declared_time(stamp(precision, 0))
        assert len(absent) == size and len(utc) == size + 4
        assert absent[:-1] == utc[:-5] and absent[-1:] == b"\0" and utc[-5:] == b"\1" + bytes(4)
    absent_receipt = deepcopy(notification)
    absent_receipt["received_at"] = stamp("unknown")
    assert encode("notification", notification) != encode("notification", absent_receipt)
    changed_locator = deepcopy(people)
    changed_locator["representation"]["provenance"]["support"]["locator"] = "p. 9"
    assert direct_supports(changed_locator) == direct_supports(people)
    assert encode("notification", changed_locator) != encode("notification", people)
    local, utc = stamp("second", 50400), stamp("second", 0)
    local.update(hour=14, minute=0, second=0)
    utc.update(hour=0, minute=0, second=0)
    assert declared_time(local) != declared_time(utc)
    for family, lower, upper in [("resolution", 27, 17700), ("notification", 67, 58671)]:
        lengths = [item["bytes"] for item in rows if item["family"] == family]
        assert min(lengths) == lower and max(lengths) == upper
    assert len({r["name"] for r in rows}) == len(rows)
    return rows


if __name__ == "__main__":
    vectors = build_vectors()
    Path(__file__).with_name("procedural_fact_vectors.json").write_text(
        "[\n" + ",\n".join(json.dumps(row, ensure_ascii=True, separators=(",", ":"))
                            for row in vectors) + "\n]\n", encoding="ascii")
    print(json.dumps({"vectors": len(vectors), "resolution_bounds": [27, 17700],
                      "notification_bounds": [67, 58671]}))
