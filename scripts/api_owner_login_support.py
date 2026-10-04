"""Private material and bounded observations for disposable Owner login HTTP."""
import base64
import hashlib
import hmac
import json
import os
from pathlib import Path
import re
import struct
import subprocess
from uuid import UUID

from api_owner_certificate_evidence import (
    WORK, command, decoded, encoded, request, sql_state,
)


STATE = WORK / "login-state.json"
START = "/api/v1/auth/certificate-login/start"
PROOF = "/api/v1/auth/certificate-login/proof"
BINDINGS = "/api/v1/auth/certificate-bindings/"
READ = """
local kind = redis.call('TYPE', KEYS[1]).ok
local expiry = redis.call('PEXPIRETIME', KEYS[1])
local function hex(value)
    return (value:gsub('.', function(c) return string.format('%02x', string.byte(c)) end))
end
if kind == 'string' then
    if redis.call('STRLEN', KEYS[1]) > 2097152 then return {'oversized', expiry} end
    return {kind, expiry, hex(redis.call('DUMP', KEYS[1]))}
end
if kind == 'hash' then
    if redis.call('HLEN', KEYS[1]) > 64 then return {'oversized', expiry} end
    local fields = redis.call('HGETALL', KEYS[1])
    for index, value in ipairs(fields) do fields[index] = hex(value) end
    return {kind, expiry, fields}
end
return {kind, expiry}
"""


def redis(*arguments):
    info = arguments == ("INFO", "server")
    result = subprocess.run([
        "redis-cli", "-h", "127.0.0.1", "-p", os.environ["TT_OWNER_LOGIN_REDIS_PORT"],
        "-3", "--raw" if info else "--json", *map(str, arguments)],
        capture_output=True, timeout=15, check=False)
    assert result.returncode == 0 and len(result.stdout) <= 4 * 1024 * 1024, \
        "Disposable Redis observation failed"
    try:
        # RESP3 INFO is a verbatim reply; only this exact read uses raw text.
        return result.stdout.decode("ascii") if info else json.loads(result.stdout)
    except (ValueError, UnicodeError):
        raise AssertionError("Invalid disposable Redis response") from None


def redis_guard(stopped=False):
    info = redis("INFO", "server")
    assert isinstance(info, str), "Missing Redis server identity"
    process = dict(line.split(":", 1) for line in info.splitlines() if ":" in line)
    assert process["process_id"] == os.environ["TT_OWNER_LOGIN_REDIS_PID"], "Redis PID changed"
    directory = redis("CONFIG", "GET", "dir")
    assert directory == {"dir": str(Path(os.environ["TT_OWNER_LOGIN_WORK"]).resolve())}, \
        "Redis directory is not owned by this campaign"
    if stopped:
        assert not os.environ.get("TT_OWNER_LOGIN_SERVER_PID"), "Stop the capture server first"


def now_ms():
    seconds, micros = redis("TIME")
    return int(seconds) * 1000 + int(micros) // 1000


def read(key):
    row = redis("EVAL", READ, 1, key)
    assert isinstance(row, list) and len(row) in [2, 3], "Invalid private observation"
    value = {"type": row[0], "expires_at_unix_ms": row[1]}
    if row[0] == "string":
        value["dump_base64"] = encoded(bytes.fromhex(row[2]))
    elif row[0] == "hash":
        assert len(row[2]) % 2 == 0, "Invalid hash observation"
        value["fields"] = [[encoded(bytes.fromhex(row[2][i])), encoded(bytes.fromhex(row[2][i + 1]))]
                           for i in range(0, len(row[2]), 2)]
    return value


def token_key(namespace, token):
    return "identity:" + namespace + ":" + hashlib.sha256(token.encode("ascii")).hexdigest()


def hash_key(kind, context, value):
    return "identity:certificate-login-rate:v1:" + kind + ":" + hashlib.sha256(context + value).hexdigest()


def private_save(value):
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW
    with os.fdopen(os.open(STATE, flags, 0o600), "w") as stream:
        os.fchmod(stream.fileno(), 0o600)
        json.dump(value, stream, ensure_ascii=True, sort_keys=True)
        stream.write("\n")


def private_load():
    assert STATE.is_file() and not STATE.is_symlink(), "Missing owned private state"
    assert STATE.stat().st_mode & 0o777 == 0o600, "Private fixture permissions changed"
    assert STATE.stat().st_size <= 2 * 1024 * 1024, "Private fixture exceeds its budget"
    return json.loads(STATE.read_text())


def signing(bytes_to_sign):
    path = WORK / "owner-client" / "login-statement.bin"
    path.write_bytes(bytes_to_sign)
    signature = command(["openssl", "dgst", "-sha256", "-sign",
                         os.environ["TT_OWNER_CERT_KEY"], str(path)])
    assert len(signature) == 384, "Unexpected detached signature width"
    return encoded(signature)


def mfa(challenge, code, method="recovery", expected=200):
    return request("POST", "/api/v1/auth/mfa/" + method,
                   {"challenge_token": challenge, "code": code}, token="", expected=expected,
                   code="mfa_rejected" if expected == 401 else None)


def password_login(saved, recovery_index):
    first = request("POST", "/api/v1/auth/login", {
        "email": saved["owner"]["email"], "password": saved["password"]}, token="")
    assert set(first) == {"challenge_token", "expires_in_seconds"}, "Password bypassed MFA"
    result = mfa(first["challenge_token"], saved["enrollment"]["recovery_codes"][recovery_index])
    assert result["user"] == saved["owner"], "Password authenticated another principal"
    return result["access_token"]


def start(saved):
    value = request("POST", START, {"owner_id": saved["owner"]["id"],
                    "binding_id": saved["binding"]}, token="")
    assert set(value) == {"challenge_token", "statement_base64", "expires_in_seconds"}
    token = value["challenge_token"]
    assert re.fullmatch(r"[A-Za-z0-9_-]{43}", token), "Invalid capture token"
    raw_token = base64.urlsafe_b64decode(token + "=")
    assert len(raw_token) == 32 and base64.urlsafe_b64encode(raw_token).decode().rstrip("=") == token
    statement = decoded(value["statement_base64"])
    trust, certificate = saved["receipt"]["registration"]["trust"], saved["receipt"]["registration"]["certificate"]
    account = sql_state(saved["owner"]["id"])["account"]
    prefix = (b"OWNAUTH1\x01\x01" + UUID(trust["deployment_id"]).bytes
              + bytes.fromhex(trust["root_fingerprint"]) + trust["revision"].to_bytes(4, "big")
              + UUID(saved["owner"]["id"]).bytes + int(account["auth_generation"]).to_bytes(8, "big")
              + UUID(saved["binding"]).bytes + bytes.fromhex(certificate["fingerprint"]))
    assert len(statement) == 182 and statement[:134] == prefix, "Login public context differs"
    issued, expiry = (int.from_bytes(statement[i:i + 8], "big", signed=True) for i in [166, 174])
    assert 1 <= expiry - issued <= 300 and issued * 1000 <= now_ms() < expiry * 1000
    assert 1 <= value["expires_in_seconds"] <= 300
    assert redis("PEXPIRETIME", token_key("certificate-login", token)) == expiry * 1000
    return {"token": token, "statement_base64": value["statement_base64"],
            "signature_base64": signing(statement), "expires_at_unix_ms": expiry * 1000}


def proof(capture, expected=200):
    value = request("POST", PROOF, {"challenge_token": capture["token"],
                    "signature_base64": capture["signature_base64"]}, token="", expected=expected,
                    code="invalid_credentials" if expected == 401 else None)
    if expected == 200:
        assert set(value) == {"challenge_token", "expires_in_seconds"}, "RSA bypassed MFA"
        assert 1 <= value["expires_in_seconds"] <= 300
        return value["challenge_token"]
    return None


def totp(secret):
    counter = now_ms() // 30_000
    digest = hmac.new(base64.b32decode(secret), struct.pack(">Q", counter), hashlib.sha1).digest()
    offset = digest[-1] & 15
    value = (struct.unpack(">I", digest[offset:offset + 4])[0] & 0x7fffffff) % 1_000_000
    return f"{value:06d}"


def admitted(saved, token):
    assert request("GET", "/api/v1/auth/me", token=token) == saved["owner"]
    request("GET", "/api/v1/auth/session", token=token)
    request("POST", "/api/v1/auth/activity", token=token)
    assert request("GET", BINDINGS + saved["binding"], token=token) == saved["receipt"]
    assert request("GET", BINDINGS + "current", token=token) == saved["receipt"]


def stored_origin(saved, token, certificate):
    value = read(token_key("session", token))
    assert value["type"] == "hash", "Session was not persisted"
    fields = {decoded(k).decode("ascii"): decoded(v) for k, v in value["fields"]}
    assert fields["version"] == (b"2" if certificate else b"1"), "Session origin downgraded"
    identity = json.loads(fields["identity_json"])
    assert identity["principal"] == saved["owner"]
    if not certificate:
        assert "authentication_json" not in fields, "Password inherited certificate provenance"
        return
    origin = json.loads(fields["authentication_json"])
    trust = saved["receipt"]["registration"]["trust"]
    leaf = saved["receipt"]["registration"]["certificate"]
    assert origin["kind"] == "certificate"
    assert origin["provenance"] == {
        "principal": saved["owner"], "auth_generation": identity["auth_generation"],
        "binding_id": saved["binding"], "deployment_id": trust["deployment_id"],
        "trust_revision": trust["revision"], "root_fingerprint": trust["root_fingerprint"],
        "leaf_fingerprint": leaf["fingerprint"], "crl_digest": trust["crl_digest"],
        "valid_from_unix_seconds": max(leaf["summary"]["not_before_unix"], trust["valid_from_unix"]),
        "valid_until_unix_seconds": min(leaf["summary"]["not_after_unix"], trust["valid_until_unix"]),
    }, "Stored certificate provenance differs"
