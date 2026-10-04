#!/usr/bin/env python3
"""Exercise real Owner login and SQL restore in the existing disposable campaign."""
import hashlib
import os
from pathlib import Path
import secrets
import sys
from uuid import UUID, uuid4

from api_owner_certificate_evidence import encoded, request, sign, sql_state, verify_receipt, verify_sql
from api_owner_login_restore import capture_boundary, verify_invalidation
from api_owner_login_support import (
    BINDINGS, STATE, admitted, hash_key, mfa, now_ms, password_login,
    private_load, private_save, proof, read, redis, redis_guard, start, stored_origin,
    token_key, totp,
)


GLOBALS = ["identity:certificate-login-rate:v1:start:global",
           "identity:certificate-login-rate:v1:proof:global"]


def prepare():
    redis_guard()
    assert not STATE.exists(), "Owner login fixture already exists"
    assert redis("EXISTS", *GLOBALS) == 0, "Login budgets are not owned by this campaign"
    password = secrets.token_urlsafe(32)
    enrollment = request("POST", "/api/v1/users", {
        "email": "owner-login-" + uuid4().hex + "@example.test",
        "password": password, "role": "owner"}, expected=201)
    owner = enrollment["user"]
    assert owner["role"] == "owner"
    saved = {"owner": owner, "password": password, "enrollment": enrollment,
             "binding": str(uuid4())}
    saved["password_session"] = password_login(saved, 0)
    stored_origin(saved, saved["password_session"], certificate=False)
    certificate = Path(os.environ["TT_OWNER_CERT_CERTIFICATE"]).read_bytes()
    assert 0 < len(certificate) <= 16384 and b"PRIVATE KEY" not in certificate
    route = BINDINGS + saved["binding"]
    prepared = request("POST", route + "/prepare", {"certificate_base64": encoded(certificate)},
                       token=saved["password_session"])
    intent = sign(prepared)
    receipt = request("POST", route + "/register", intent, token=saved["password_session"])
    assert receipt["owner_id"] == owner["id"] and receipt["binding_id"] == saved["binding"]
    assert receipt["revision"] == 1 and receipt["withdrawal"] is None
    verify_receipt(receipt, intent)
    saved.update(receipt=receipt, intent=intent)
    admitted(saved, saved["password_session"])
    saved["registered_state"] = verify_sql(receipt, intent, owner)
    private_save(saved)
    print("Owner login fixture prepared: dedicated account, public Partner and exact live binding.")


def capture():
    redis_guard()
    saved = private_load()
    assert "boundary" not in saved, "Owner login restore boundary already captured"
    admitted(saved, saved["password_session"])
    first = start(saved)
    challenge = proof(first)
    code = totp(saved["enrollment"]["totp_secret_base32"])
    session = mfa(challenge, code, method="totp")
    assert session["user"] == saved["owner"]
    saved["certificate_session"] = session["access_token"]
    admitted(saved, saved["certificate_session"])
    stored_origin(saved, saved["certificate_session"], certificate=True)
    pending = start(saved)
    saved["pending_mfa"] = proof(pending)
    saved["pending_first"] = start(saved)
    saved["captured_state"] = sql_state(saved["owner"]["id"])
    assert saved["captured_state"] == saved["registered_state"], "TOTP changed captured account state"
    # A failed password attempt creates a real independent failure budget.
    request("POST", "/api/v1/auth/login", {"email": saved["owner"]["email"],
            "password": "deliberately wrong disposable password"}, token="",
            expected=401, code="invalid_credentials")
    authentication = {
        "first": token_key("certificate-login", saved["pending_first"]["token"]),
        "mfa": token_key("challenge", saved["pending_mfa"]),
        "session": token_key("session", saved["certificate_session"]),
    }
    controls = list(GLOBALS)
    controls.append(hash_key("start:owner-binding", b"qadra:owner-certificate-login-start-limit:v1\0",
                             UUID(saved["owner"]["id"]).bytes + UUID(saved["binding"]).bytes))
    for capture_value in [first, pending]:
        controls.append(hash_key("proof:token", b"qadra:owner-certificate-login-proof-limit:v1\0",
                                 capture_value["token"].encode("ascii")))
    controls.append(token_key("password-failures", saved["owner"]["email"]))
    controls.append("identity:totp-used:" + hashlib.sha256(
        UUID(saved["owner"]["id"]).bytes + code.encode("ascii")).hexdigest())
    # This opaque reset-namespace control grants no password-reset authority.
    reset = "identity:password-reset:v1:request:email:" + hashlib.sha256(
        b"qadra:password-reset-request-limit:v1\0" + saved["owner"]["email"].encode("ascii")).hexdigest()
    sentinel = "owner-login-restore:" + uuid4().hex + ":control"
    assert redis("SET", reset, "owned restore namespace control", "NX", "EX", 3600) == "OK"
    assert redis("SET", sentinel, "owned unchanged sentinel", "NX") == "OK"
    controls.extend([reset, sentinel])
    saved["boundary"] = capture_boundary(authentication, controls, read, now_ms())
    assert saved["boundary"]["records"][authentication["first"]]["expires_at_unix_ms"] == \
        saved["pending_first"]["expires_at_unix_ms"]
    private_save(saved)
    print("Owner login capture passed: persisted session, pending MFA, pending RSA proof and independent controls.")


def invalidated():
    redis_guard(stopped=True)
    saved = private_load()
    assert "invalidation_checked_at_unix_ms" not in saved, "Invalidation already checked"
    observed = now_ms()
    verify_invalidation(saved["boundary"], read, observed)
    saved["invalidation_checked_at_unix_ms"] = observed
    private_save(saved)
    print("Owner login invalidation passed before expiry: three credentials absent, exact controls and deadlines retained.")


def restored():
    redis_guard()
    saved = private_load()
    assert "invalidation_checked_at_unix_ms" in saved and "restored" not in saved
    # Reconfirm the boundary before HTTP attempts consume proof budgets.
    verify_invalidation(saved["boundary"], read, now_ms())
    assert sql_state(saved["owner"]["id"]) == saved["captured_state"], "Restored Owner authority differs"
    proof(saved["pending_first"], expected=401)
    assert now_ms() < saved["pending_first"]["expires_at_unix_ms"], \
        "Old first-factor response arrived after expiry"
    recovery = saved["enrollment"]["recovery_codes"]
    mfa(saved["pending_mfa"], recovery[1], expected=401)
    mfa_key = saved["boundary"]["authentication"]["mfa"]
    assert now_ms() < saved["boundary"]["records"][mfa_key]["expires_at_unix_ms"], \
        "Old MFA response arrived after expiry"
    for token in [saved["password_session"], saved["certificate_session"]]:
        for method, path in [("GET", "/api/v1/auth/me"), ("GET", "/api/v1/auth/session"),
                             ("POST", "/api/v1/auth/activity")]:
            request(method, path, token=token, expected=401, code="invalid_session")
    assert sql_state(saved["owner"]["id"]) == saved["captured_state"], \
        "Rejected restored credentials changed account or binding state"
    fresh = start(saved)
    certificate = mfa(proof(fresh), recovery[1])["access_token"]
    admitted(saved, certificate)
    stored_origin(saved, certificate, certificate=True)
    # The same password remains an independent first factor after restoration.
    password = password_login(saved, 2)
    admitted(saved, password)
    stored_origin(saved, password, certificate=False)
    admitted(saved, certificate)
    before, after = saved["captured_state"], sql_state(saved["owner"]["id"])
    assert all(after[key] == before[key] for key in ["registrations", "withdrawals", "events"])
    assert after["account"] == {**before["account"],
                                "revision": str(int(before["account"]["revision"]) + 2)}, \
        "Fresh recovery MFA changed more than two expected account revisions"
    verify_receipt(saved["receipt"], saved["intent"])
    verify_sql(saved["receipt"], saved["intent"], saved["owner"])
    saved["restored"] = True
    private_save(saved)
    print("Owner login restore passed: old credentials denied, exact binding retained, fresh RSA/MFA and password/MFA admitted.")


if __name__ == "__main__":
    actions = {"prepare": prepare, "capture": capture, "invalidated": invalidated, "restored": restored}
    assert len(sys.argv) == 2 and sys.argv[1] in actions
    actions[sys.argv[1]]()
