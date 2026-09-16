#!/usr/bin/env python3
"""Exercise typed identities and public signature evidence against a disposable API."""

import base64
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
from urllib.error import HTTPError
from urllib.request import Request, urlopen


BASE = os.environ["TT_TYPED_API_BASE_URL"]
TOKEN = os.environ["TT_TYPED_API_TOKEN"]
WORK = Path(os.environ["TT_TYPED_API_WORK_DIR"])
REPO = Path(os.environ["TT_TYPED_API_REPO"])
STATE = WORK / "typed-participant-api-state.json"


def request(method, path, body=None, expected=200, headers=None):
    metadata = {"Authorization": "Bearer " + TOKEN}
    if isinstance(body, dict):
        body = json.dumps(body).encode("utf-8")
        metadata["Content-Type"] = "application/json"
    metadata.update(headers or {})
    query = Request(BASE + path, data=body, headers=metadata, method=method)
    try:
        response = urlopen(query, timeout=30)
    except HTTPError as error:
        response = error
    with response:
        data = response.read(1024 * 1024)
        assert response.status == expected, (method, path, response.status, expected, data)
    return json.loads(data) if data else None


def unknown(reason="Synthetic fixture does not assert this value"):
    return {"state": "unknown", "reason": reason}


def reviewed(route, subject, role, certificate=None, participant=None):
    review = request("POST", route + "/participants/proposals/review", {
        "subject": subject, "participant": participant or {"operation": "create"},
        "role": role, "certificate_base64": certificate,
    })
    assert review["candidates"] == [], review["candidates"]
    prepared = {
        "proposal": review["proposal"],
        "review": {"directory_stamp": review["directory_stamp"],
                   "selection_reason": "Reviewed synthetic identity and exact support",
                   "different": []},
        "certificate_base64": certificate,
    }
    draft = request("POST", route + "/participants/proposals/prepare", prepared)
    return prepared, draft


def external_signature(draft):
    declaration = draft["declaration"]
    statement = base64.b64decode(declaration["bytes_base64"], validate=True)
    assert len(statement) == 218 and statement[:6] == b"PCRED1"
    assert hashlib.sha256(statement).hexdigest() == declaration["digest"]
    statement_path = WORK / "typed-participant-declaration.bin"
    signature_path = WORK / "typed-participant-declaration.sig"
    statement_path.write_bytes(statement)
    subprocess.run([
        "openssl", "dgst", "-sha256", "-sign", os.environ["TT_TYPED_API_PRIVATE_KEY"],
        "-out", str(signature_path), str(statement_path),
    ], check=True, capture_output=True)
    signature = signature_path.read_bytes()
    assert len(signature) == 384
    return statement, signature


def verify_public_evidence(evidence):
    cert = base64.b64decode(evidence["certificate_der_base64"], validate=True)
    statement = base64.b64decode(evidence["declaration_base64"], validate=True)
    signature = base64.b64decode(evidence["signature_base64"], validate=True)
    assert hashlib.sha256(cert).hexdigest() == evidence["certificate_fingerprint"]
    assert hashlib.sha256(statement).hexdigest() == evidence["statement_digest"]
    assert isinstance(evidence["trust"]["crl_number"], str)
    (WORK / "typed-public-cert.der").write_bytes(cert)
    (WORK / "typed-public-statement.bin").write_bytes(statement)
    (WORK / "typed-public-signature.bin").write_bytes(signature)
    public = subprocess.run([
        "openssl", "x509", "-inform", "DER", "-in", str(WORK / "typed-public-cert.der"),
        "-pubkey", "-noout",
    ], check=True, capture_output=True).stdout
    (WORK / "typed-public-key.pem").write_bytes(public)
    subprocess.run([
        "openssl", "dgst", "-sha256", "-verify", str(WORK / "typed-public-key.pem"),
        "-signature", str(WORK / "typed-public-signature.bin"),
        str(WORK / "typed-public-statement.bin"),
    ], check=True, capture_output=True)


def capture():
    case = request("POST", "/api/v1/cases", {
        "title": "Typed participant recovery", "reference": "TYPED-RESTORE-001",
    }, 201)
    route = "/api/v1/cases/" + case["id"]
    pdf = (REPO / "crates/infrastructure/tests/fixtures/stage-support.pdf").read_bytes()
    document = request("POST", route + "/documents", pdf, 201,
                       {"X-Document-Name": "participant-identity.pdf"})
    locator = {"document_id": document["id"], "version": document["version"],
               "digest": document["digest"], "locator": "Page 1"}
    values = {"kind": "natural_person", "name": {"state": "known", "value": "Fixture Person"},
              "curp": unknown(), "identity_support": locator}
    role = {"organization": None, "legal_status": None,
            "profile": {"kind": "defendant", "custody": unknown()}, "role_support": locator}
    prepared, draft = reviewed(route, {"operation": "create", "values": values}, role)
    assert draft["declaration"] is None and len(draft["submission_digest"]) == 64
    submission = {"prepared": prepared, "signature_base64": None}
    first = request("POST", route + "/participants/proposals/commit", submission, 201)
    assert first["submission_digest"] == draft["submission_digest"]
    request("POST", route + "/participants/proposals/commit", submission, 409)
    subject = first["subject"]
    reference = {"id": subject["id"], "revision": subject["revision"],
                 "values_digest": subject["values_digest"]}
    certificate = base64.b64encode(Path(os.environ["TT_TYPED_API_CERTIFICATE"]).read_bytes()).decode()
    role["profile"] = {"kind": "control_judge", "court": "Synthetic Court"}
    prepared, draft = reviewed(route, {"operation": "keep", "reference": reference}, role, certificate)
    assert draft["submission_digest"] is None
    statement, signature = external_signature(draft)
    invalid = bytes([signature[0] ^ 1]) + signature[1:]
    request("POST", route + "/participants/proposals/commit", {
        "prepared": prepared, "signature_base64": base64.b64encode(invalid).decode(),
    }, 422)
    signed = request("POST", route + "/participants/proposals/commit", {
        "prepared": prepared, "signature_base64": base64.b64encode(signature).decode(),
    }, 201)
    credential_path = route + "/participants/" + signed["id"] + "/revisions/1/credential"
    evidence = request("GET", credential_path)
    assert base64.b64decode(evidence["declaration_base64"]) == statement
    assert base64.b64decode(evidence["signature_base64"]) == signature
    verify_public_evidence(evidence)
    updated_values = copy.deepcopy(values)
    updated_values["name"]["value"] = "Updated Fixture Person"
    subject_path = route + "/subjects/" + subject["id"]
    replacement = {"expected_revision": 1, "values": updated_values}
    review = request("POST", subject_path + "/review", replacement)
    assert review["candidates"] == []
    replacement["review"] = {"directory_stamp": review["directory_stamp"],
                             "selection_reason": "Corrected synthetic name", "different": []}
    updated = request("PUT", subject_path, replacement)
    assert updated["revision"] == 2
    participant_path = route + "/participants/" + signed["id"]
    archived = request("PUT", participant_path + "/directory-status", {
        "expected_revision": 1, "directory_status": "archived",
    })
    assert archived["revision"] == 2 and archived["subject"]["revision"] == 1
    assert archived["credential_origin"] == signed["credential_origin"]
    assert request("GET", credential_path) == evidence
    paths = [route + "/participants?status=all", route + "/subjects", subject_path,
             subject_path + "/revisions/1", subject_path + "/history", participant_path,
             participant_path + "/history", participant_path + "/revisions/1", credential_path,
             route + "/participants/" + first["id"]]
    records = {path: request("GET", path) for path in paths}
    STATE.write_text(json.dumps({"records": records, "credential": credential_path}, sort_keys=True))
    assert request("GET", "/api/v1/audit/verify")["valid"]
    print("Typed participant API passed: exact identities, external signature, rejection, history and public proof.")


def restore():
    state = json.loads(STATE.read_text())
    for path, expected in state["records"].items():
        assert request("GET", path) == expected, "Restored response differs: " + path
    verify_public_evidence(request("GET", state["credential"]))
    assert request("GET", "/api/v1/audit/verify")["valid"]
    print("Typed participant restore passed: exact values, subjects, origins, public signature and trust.")


if __name__ == "__main__":
    {"capture": capture, "restore": restore}[sys.argv[1]]()
