"""Independent HEAR1 wire vectors; run this file to reproduce the JSON fixture."""
from copy import deepcopy
from datetime import datetime, timedelta, timezone
from hashlib import sha256
from pathlib import Path
from uuid import UUID
import json
import struct


def text(value):
    encoded = value.encode("utf-8")
    return struct.pack(">I", len(encoded)) + encoded


def encode(value):
    kinds = ["initial", "intermediate", "oral_trial", "sentencing"]
    modes = ["in_person", "videoconference"]
    stamp = datetime.fromisoformat(value["time"])
    epoch = datetime(1970, 1, 1, tzinfo=timezone.utc)
    delta = stamp.astimezone(timezone.utc) - epoch
    seconds = delta.days * 86400 + delta.seconds
    offset = int(stamp.utcoffset().total_seconds())
    body = b"HEAR1" + bytes([kinds.index(value["kind"])])
    body += struct.pack(">qiB", seconds, offset, modes.index(value["modality"]))
    body += text(value["venue"])
    body += b"\0" if value["note"] is None else b"\1" + text(value["note"])
    participants = sorted(value["participants"], key=lambda item: UUID(item["id"]).bytes)
    body += bytes([len(participants)])
    for participant in participants:
        body += UUID(participant["id"]).bytes + struct.pack(">I", participant["revision"])
    basis = value["conviction_basis"]
    body += b"\0" if basis is None else (
        b"\1" + text(basis["statement"]) + UUID(basis["document_id"]).bytes
        + struct.pack(">I", basis["version"]) + bytes.fromhex(basis["digest"])
    )
    return body


def row(name, value):
    encoded = encode(value)
    return {"name": name, "input": value, "bytes": len(encoded),
            "hex": encoded.hex(), "sha256": sha256(encoded).hexdigest()}


base = {"kind": "initial", "time": "2026-09-15T09:00:00-06:00",
        "modality": "in_person", "venue": "Court A", "note": None,
        "participants": [], "conviction_basis": None}
rows = [row("initial_minimal", base)]
intermediate = deepcopy(base)
intermediate.update(kind="intermediate", time="1900-01-01T12:34:56+05:30",
                    modality="videoconference", venue="https://example.test/room",
                    note="First\nSecond")
intermediate["participants"] = [{"id": str(UUID(int=9)), "revision": 7},
                                {"id": str(UUID(int=1)), "revision": 0xffffffff}]
rows.append(row("intermediate_negative_epoch", intermediate))
trial = deepcopy(base)
trial.update(kind="oral_trial", time="2026-12-31T23:50:00-14:00", venue="Court \u00e1")
rows.append(row("trial_year_crossing", trial))
sentencing = deepcopy(base)
sentencing.update(kind="sentencing", time="2026-01-01T00:10:00+14:00", note="\u00e1\nb")
sentencing["participants"] = deepcopy(intermediate["participants"])
sentencing["conviction_basis"] = {"statement": "Declared\nConviction \u00e1",
    "document_id": "00112233-4455-6677-8899-aabbccddeeff", "version": 7,
    "digest": bytes(range(32)).hex()}
rows.append(row("sentencing_exact_support", sentencing))
maximum = deepcopy(sentencing)
wide = chr(0x1f642)
maximum.update(venue=wide * 500, note=wide * 1000)
maximum["participants"] = [{"id": str(UUID(int=n)), "revision": 0xffffffff} for n in range(32, 0, -1)]
maximum["conviction_basis"]["statement"] = wide * 1000
assert len(encode(maximum)) == 10726
rows.append(row("maximum_utf8", maximum))
Path(__file__).with_name("hearing_vectors.json").write_text(
    json.dumps(rows, ensure_ascii=True, indent=2) + "\n", encoding="ascii")
print(json.dumps({"vectors": len(rows), "maximum_bytes": len(encode(maximum))}))
