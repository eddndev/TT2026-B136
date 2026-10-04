"""Validate private Redis evidence across disposable authentication invalidation."""
import base64
import copy
import re


AUTH_NAMES = {"first": "certificate-login", "mfa": "challenge", "session": "session"}
MAX_BYTES = 2 * 1024 * 1024


def encoded_bytes(value):
    assert isinstance(value, str) and len(value) <= MAX_BYTES * 2, "Invalid private bytes"
    result = base64.b64decode(value, validate=True)
    assert len(result) <= MAX_BYTES, "Private evidence exceeds its byte budget"
    assert base64.b64encode(result).decode("ascii") == value, "Noncanonical private bytes"
    return result


def normalized(value):
    assert isinstance(value, dict), "Missing private Redis observation"
    kind, expiry = value.get("type"), value.get("expires_at_unix_ms")
    assert type(expiry) is int and -2 <= expiry <= 9_007_199_254_740_991, "Invalid expiry"
    if kind == "none":
        assert value == {"type": "none", "expires_at_unix_ms": -2}, "Invalid absence"
        return copy.deepcopy(value)
    if kind == "string":
        assert set(value) == {"type", "expires_at_unix_ms", "dump_base64"}, "Invalid scalar"
        encoded_bytes(value["dump_base64"])
        return copy.deepcopy(value)
    assert kind == "hash" and set(value) == {"type", "expires_at_unix_ms", "fields"}, \
        "Unsupported Redis evidence type"
    rows = value["fields"]
    assert isinstance(rows, list) and 1 <= len(rows) <= 64, "Invalid hash field budget"
    decoded = []
    for row in rows:
        assert isinstance(row, list) and len(row) == 2, "Invalid hash field pair"
        decoded.append((encoded_bytes(row[0]), encoded_bytes(row[1])))
    assert len({row[0] for row in decoded}) == len(decoded), "Duplicate hash field"
    assert sum(len(k) + len(v) for k, v in decoded) <= MAX_BYTES, "Hash evidence too large"
    return {"type": kind, "expires_at_unix_ms": expiry,
            "fields": [[base64.b64encode(k).decode("ascii"), base64.b64encode(v).decode("ascii")]
                       for k, v in sorted(decoded)]}


def authentication_keys(authentication):
    assert isinstance(authentication, dict) and set(authentication) == set(AUTH_NAMES), \
        "Three exact authentication records are required"
    for role, namespace in AUTH_NAMES.items():
        key = authentication[role]
        assert isinstance(key, str) and re.fullmatch("identity:" + namespace + r":[a-f0-9]{64}", key), \
            "Authentication evidence must identify an exact owned key"
    return set(authentication.values())


def control_keys(controls, authentication):
    assert isinstance(controls, list) and 1 <= len(controls) <= 64, "Invalid control inventory"
    assert all(isinstance(key, str) and key.isascii() and 0 < len(key) <= 256
               and not any(ord(c) < 33 or ord(c) > 126 for c in key) for key in controls), \
        "Invalid control key"
    assert len(set(controls)) == len(controls), "Duplicate control"
    assert not set(controls) & authentication, "Authentication cannot be a retained control"
    assert not any(re.match(r"identity:(session|challenge|certificate-login):", key)
                   for key in controls), "Authentication namespace cannot be retained"


def live(value, now_ms, authentication=False):
    assert value["type"] != "none", "Expected private Redis record is absent"
    deadline = value["expires_at_unix_ms"]
    assert deadline > now_ms or (not authentication and deadline == -1), \
        "Expired evidence cannot establish restore invalidation"


def capture_boundary(authentication, controls, read, now_ms):
    assert type(now_ms) is int and now_ms >= 0, "Invalid Redis clock"
    keys = authentication_keys(authentication)
    control_keys(controls, keys)
    records = {}
    for role, key in authentication.items():
        value = normalized(read(key))
        assert value["type"] == ("hash" if role == "session" else "string"), \
            "Wrong stored authentication type"
        live(value, now_ms, authentication=True)
        records[key] = value
    for key in controls:
        value = normalized(read(key))
        live(value, now_ms)
        records[key] = value
    return {"captured_at_unix_ms": now_ms, "authentication": copy.deepcopy(authentication),
            "controls": list(controls), "records": records}


def verify_invalidation(snapshot, read, now_ms):
    assert isinstance(snapshot, dict) and set(snapshot) == {
        "captured_at_unix_ms", "authentication", "controls", "records"}, "Invalid boundary snapshot"
    captured = snapshot["captured_at_unix_ms"]
    assert type(captured) is int and type(now_ms) is int and 0 <= captured <= now_ms, \
        "Restored Redis clock precedes capture"
    keys = authentication_keys(snapshot["authentication"])
    controls = snapshot["controls"]
    control_keys(controls, keys)
    records = snapshot["records"]
    assert isinstance(records, dict) and set(records) == keys | set(controls), "Incomplete evidence"
    before = {key: normalized(value) for key, value in records.items()}
    # Validate the entire saved boundary before inspecting the restored instance.
    for role, key in snapshot["authentication"].items():
        assert before[key]["type"] == ("hash" if role == "session" else "string"), \
            "Wrong saved authentication type"
    for key, value in before.items():
        live(value, now_ms, authentication=key in keys)
    for key in keys:
        assert normalized(read(key))["type"] == "none", "Old authentication survived invalidation"
    for key in controls:
        assert normalized(read(key)) == before[key], "Retained Redis control changed"
