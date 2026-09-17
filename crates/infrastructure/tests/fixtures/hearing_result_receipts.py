#!/usr/bin/env python3
"""Reproduce HRTX1 boundary vectors independently of the Rust encoder."""

import hashlib
import json
import struct
from pathlib import Path
from uuid import UUID


def vector(maximum):
    ids = [UUID(int=value) for value in range(1, 6)]
    anchor = {
        "revision": 7,
        "values_digest": bytes(range(32)).hex(),
        "submission_digest": bytes(range(32, 64)).hex(),
    }
    continuation = None
    reason = None
    if maximum:
        continuation = {
            "hearing_id": str(UUID(int=40)),
            "result_id": str(UUID(int=50)),
            "revision": 6,
            "values_digest": bytes(range(64, 96)).hex(),
            "submission_digest": bytes(range(96, 128)).hex(),
        }
        reason = "\U0001f9ea" * 1000
    expected = 4294967294 if maximum else 0
    digest = bytes(range(128, 160))
    raw = bytearray(b"HRTX1")
    for identity in ids:
        raw.extend(identity.bytes)
    raw.extend(struct.pack(">BII", int(maximum), expected, anchor["revision"]))
    raw.extend(bytes.fromhex(anchor["values_digest"]))
    raw.extend(bytes.fromhex(anchor["submission_digest"]))
    raw.append(int(continuation is not None))
    if continuation:
        raw.extend(UUID(continuation["hearing_id"]).bytes)
        raw.extend(UUID(continuation["result_id"]).bytes)
        raw.extend(struct.pack(">I", continuation["revision"]))
        raw.extend(bytes.fromhex(continuation["values_digest"]))
        raw.extend(bytes.fromhex(continuation["submission_digest"]))
    raw.extend(digest)
    raw.append(int(reason is not None))
    if reason:
        encoded = reason.encode("utf-8")
        raw.extend(struct.pack(">I", len(encoded)))
        raw.extend(encoded)
    projection = dict(zip(
        ["operation_id", "actor_id", "case_id", "hearing_id", "result_id"],
        map(str, ids),
    ))
    projection.update({
        "action": "correct" if maximum else "record",
        "expected_revision": expected,
        "anchor": anchor,
        "continuation": continuation,
        "values_digest": digest.hex(),
        "reason": reason,
    })
    return {
        "name": "maximum" if maximum else "minimum",
        "hex": raw.hex(),
        "sha256": hashlib.sha256(raw).hexdigest(),
        "bytes": len(raw),
        "projection": projection,
    }


if __name__ == "__main__":
    expected = [vector(False), vector(True)]
    fixture = Path(__file__).with_suffix(".json")
    actual = json.loads(fixture.read_text(encoding="ascii"))
    assert actual == expected, "HRTX1 fixture differs from independent encoder"
    assert [value["bytes"] for value in expected] == [192, 4296]
    print("Independent HRTX1 fixtures match: 192 and 4296 bytes")
