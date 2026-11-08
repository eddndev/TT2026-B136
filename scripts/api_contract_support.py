"""HTTP, account, key and package helpers of the disposable bilateral contract demo.

The demo runs against isolated local services (scripts/api-contract-demo.sh);
the rules it checks are those of docs/adr/0086-bilateral-contract-signing.md.
"""
import base64
import hashlib
import hmac
import io
import json
import os
from pathlib import Path
import struct
import subprocess
import time
from urllib.error import HTTPError
from urllib.request import Request, urlopen
from uuid import UUID, uuid4
import zipfile

from cryptography.hazmat.primitives.ciphers.aead import AESGCM

BASE = os.environ["BASE_URL"]
WORK = Path(os.environ["WORK_DIR"])
REPO = Path(os.environ["REPO_ROOT"])
STATE = WORK / "contract-api-state.json"
PASSWORD = "synthetic bilateral contract fixture password"
PDF = REPO / "crates/infrastructure/tests/fixtures/stage-support.pdf"


def call(method, path, body=None, token=None, expected=200, code=None, headers=None, raw=False):
    metadata = dict(headers or {})
    if token:
        metadata["Authorization"] = "Bearer " + token
    if isinstance(body, (dict, list)):
        body = json.dumps(body, ensure_ascii=True, separators=(",", ":")).encode("ascii")
        metadata["Content-Type"] = "application/json"
    try:
        response = urlopen(Request(BASE + path, data=body, headers=metadata, method=method), timeout=120)
    except HTTPError as error:
        response = error
    with response:
        payload = response.read(8 * 1024 * 1024 + 1)
        assert len(payload) <= 8 * 1024 * 1024, "response exceeds the fixture budget"
        assert response.headers.get("Cache-Control") == "no-store", path
        result = payload if raw and response.status == 200 else (json.loads(payload) if payload else None)
        actual = None if raw and response.status == 200 else (result or {}).get("error", {}).get("code")
        assert response.status == expected, (method, path, response.status, expected, actual)
        if code:
            assert actual == code, (method, path, actual, code)
        if raw:
            return result, response.headers
    return result


def totp(secret):
    digest = hmac.new(base64.b32decode(secret), struct.pack(">Q", int(time.time()) // 30), hashlib.sha1).digest()
    offset = digest[-1] & 15
    return f"{(struct.unpack('>I', digest[offset:offset + 4])[0] & 0x7FFFFFFF) % 1_000_000:06d}"


def login(account, index):
    challenge = call("POST", "/api/v1/auth/login", {"email": account["email"], "password": PASSWORD})
    return call("POST", "/api/v1/auth/mfa/recovery", {
        "challenge_token": challenge["challenge_token"], "code": account["recovery_codes"][index]})["access_token"]


def bootstrap():
    account = {"email": "owner@contracts.example.test"}
    result = call("POST", "/api/v1/auth/bootstrap", {"email": account["email"], "name": "Contract owner",
                  "password": PASSWORD}, expected=201)
    account.update(recovery_codes=result["recovery_codes"], role="owner")
    account["token"] = login(account, 0)
    account["id"] = call("GET", "/api/v1/auth/me", token=account["token"])["id"]
    return account


def staff(owner, label, role):
    account = {"email": label + "@contracts.example.test", "role": role}
    result = call("POST", "/api/v1/users", {"email": account["email"], "name": "Contract " + label,
                  "password": PASSWORD, "role": role}, token=owner["token"], expected=201)
    account.update(id=result["user"]["id"], recovery_codes=result["recovery_codes"])
    account["token"] = login(account, 0)
    return account


def psql(sql):
    result = subprocess.run(["psql", os.environ["ADMIN_DSN"], "-v", "ON_ERROR_STOP=1", "-Atc", sql],
                            capture_output=True, timeout=60, check=False)
    assert result.returncode == 0, "fixture query failed"
    return result.stdout.decode("ascii").strip()


def delivered_token(email):
    """Opens the stored delivery capability with the disposable runtime key.

    The provider is unreachable inside the demo namespace, so this replaces
    reading the invitation e-mail. The layout is that of
    crates/infrastructure/src/identity/invitation_secret.rs.
    """
    row = psql("SELECT id||'|'||revision||'|'||encode(protected_token,'hex') FROM member_invitations "
               "WHERE email='" + email + "'")
    invitation, revision, sealed = row.split("|")
    sealed = bytes.fromhex(sealed)
    aad = (b"qadra:invitation-protection:v1\x00" + b"\x01" + UUID(invitation).bytes
           + int(revision).to_bytes(8, "big"))
    key = base64.b64decode(os.environ["KEK_BASE64"])
    token = AESGCM(key).decrypt(sealed[:12], sealed[12:], aad)
    assert len(token) == 32
    return base64.urlsafe_b64encode(token).rstrip(b"=").decode("ascii")


def invite_client(owner):
    account = {"email": "client@contracts.example.test", "role": "client"}
    issued = call("POST", "/api/v1/member-invitations", {"operation_id": str(uuid4()),
                  "email": account["email"], "name": "Contract client", "role": "client"},
                  token=owner["token"], expected=201)
    assert issued["role"] == "client" and issued["status"] == "pending"
    token = delivered_token(account["email"])
    body = {"token": token, "operation_id": str(uuid4()), "password": PASSWORD}
    prepared = call("POST", "/api/v1/auth/invitations/prepare", body)
    finish = {**body, "operation_id": str(uuid4()), "code": totp(prepared["totp_secret_base32"])}
    receipt = call("POST", "/api/v1/auth/invitations/finish", finish)
    assert receipt["role"] == "client" and receipt["email"] == account["email"]
    account.update(id=receipt["user_id"], recovery_codes=prepared["recovery_codes"])
    account["token"] = login(account, 0)
    return account


def encoded(value):
    return base64.b64encode(value).decode("ascii")


def decoded(value):
    raw = base64.b64decode(value, validate=True)
    assert encoded(raw) == value, "noncanonical base64"
    return raw


def sign(party, raw, name):
    path = WORK / party / name
    path.write_bytes(raw)
    result = subprocess.run(["openssl", "dgst", "-sha256", "-sign", str(WORK / party / "private.key.pem"),
                             str(path)], capture_output=True, timeout=60, check=False)
    assert result.returncode == 0 and len(result.stdout) == 384
    return encoded(result.stdout)


def bind(account, party):
    """Registers the party's own certificate with a DOCBIND1 possession proof."""
    prefix = "/api/v1/identity/signing-certificates/"
    binding = str(uuid4())
    certificate = Path(os.environ["PKI_CA_DIR"]) / "certs" / ("contract-" + party + ".crt.pem")
    prepared = call("POST", prefix + binding + "/prepare", {"certificate_base64": encoded(certificate.read_bytes())},
                    token=account["token"])
    raw = decoded(prepared["statement_base64"])
    assert len(raw) == 142 and raw[:8] == b"DOCBIND1" and UUID(bytes=raw[62:78]) == UUID(account["id"])
    receipt = call("POST", prefix + binding + "/register", {"statement_base64": prepared["statement_base64"],
                   "certificate_der_base64": prepared["certificate"]["der_base64"],
                   "signature_base64": sign(party, raw, "binding.bin")}, token=account["token"])
    assert receipt["withdrawn_at"] is None
    return receipt


def zip_entries(archive):
    with zipfile.ZipFile(io.BytesIO(archive)) as package:
        return {name: package.read(name) for name in package.namelist()}


def validate(archive, name, expected_profile, complete):
    path = WORK / name
    path.write_bytes(archive)
    root = subprocess.run(["openssl", "x509", "-in", os.environ["CA"], "-outform", "DER"],
                          capture_output=True, timeout=60, check=True).stdout
    result = subprocess.run([os.environ["VALIDATE"], str(path), hashlib.sha256(root).hexdigest()],
                            capture_output=True, timeout=120, check=False)
    report = result.stdout.decode("utf-8")
    assert "profile: Some(" + expected_profile + ")" in report, report
    assert "[failed]" not in report, report
    if complete:
        assert result.returncode == 0 and "overall: complete" in report, report
    else:
        assert result.returncode == 1 and "overall: partial" in report, report
        assert "contract.partial" in report, report
    return report


def openssl_two_step(entries, position):
    """The OpenSSL check of INSTRUCCIONES.md: signature, then the PDF digest."""
    folder = WORK / ("package-" + str(position))
    folder.mkdir(exist_ok=True)
    for name in ("parte-%d-declaracion.bin" % position, "parte-%d-declaracion.sig" % position,
                 "parte-%d-certificado.pem" % position):
        (folder / name).write_bytes(entries[name])
    key = subprocess.run(["openssl", "x509", "-pubkey", "-noout", "-in",
                          str(folder / ("parte-%d-certificado.pem" % position))],
                         capture_output=True, timeout=60, check=True).stdout
    (folder / "public.pem").write_bytes(key)
    result = subprocess.run(["openssl", "dgst", "-sha256", "-verify", str(folder / "public.pem"), "-signature",
                             str(folder / ("parte-%d-declaracion.sig" % position)),
                             str(folder / ("parte-%d-declaracion.bin" % position))],
                            capture_output=True, timeout=60, check=False)
    assert result.returncode == 0 and b"Verified OK" in result.stdout
    statement = entries["parte-%d-declaracion.bin" % position]
    assert statement[206:238] == hashlib.sha256(entries["contrato.pdf"]).digest()


def save(value):
    with os.fdopen(os.open(STATE, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600), "w") as stream:
        json.dump(value, stream, sort_keys=True, ensure_ascii=True)
