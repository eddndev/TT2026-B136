#!/usr/bin/env python3
"""Real external signing, immutable receipts, revocation and restored HTTP reads."""
import copy
import hashlib
import json
import os
from pathlib import Path
import sys
from uuid import uuid4

from api_individual_signing_support import (
    PREFIX, STATE, WORK, REPO, TOKEN, request, upload, enroll, login, encoded, decoded,
    binding_bytes, consent_bytes, sign, save, document_path, content, archive, binary,
    durable, history,
)


def preparation(saved, operation, signature, version=1):
    route = document_path(saved).rsplit("/", 1)[0] + "/" + str(version) + "/individual-signing"
    prepared = request("POST", route + "/prepare", {"operation_id": operation,
                       "pdf_signature_base64": signature}, token=saved["account"]["token"])
    assert prepared["account_id"] == saved["account"]["id"]
    assert prepared["document_id"] == saved["document"]["document_id"]
    assert prepared["case_id"] == saved["case_id"] and prepared["version"] == version
    assert prepared["document_digest"] == saved["document"]["digest"]
    assert prepared["pdf_signature_digest"] == hashlib.sha256(decoded(signature)).hexdigest()
    canonical = consent_bytes(prepared, saved["binding"])
    return prepared, {"consent_base64": encoded(canonical), "pdf_signature_base64": signature,
                      "consent_signature_base64": sign(canonical, "document-consent.bin")}


def legacy(saved):
    evidence, _ = binary(saved["legacy_path"] + "/evidence", TOKEN)
    assert hashlib.sha256(evidence).hexdigest() == saved["legacy_digest"]
    report = request("GET", saved["legacy_path"] + "/individual-signing/verification")
    assert report["kind"] == "legacy_server" and report["signing"] is None
    assert report["historical"]["verdict"] == "valid"
    assert report["certificate_current"]["status"] == "not_individual"


def capture():
    case = request("POST", "/api/v1/cases", {"title": "External individual signing",
                    "reference": "INDIVIDUAL-API"}, expected=201)
    route = "/api/v1/cases/" + case["id"]
    account = enroll("individual-signing", "litigator", case["id"])
    saved = {"case_id": case["id"], "account": account}
    saved["document"] = upload(route, "pdf", "individual-selected.pdf")
    source = (REPO / "crates/infrastructure/tests/fixtures/stage-support.pdf").read_bytes()
    request("POST", route + "/documents/" + saved["document"]["document_id"] + "/versions?expected_version=1",
            source, expected=201, headers={"X-Document-Name": "individual-current.pdf"})
    old = upload(route, "pdf", "individual-legacy.pdf")
    saved["legacy_path"] = route + "/documents/" + old["document_id"] + "/versions/1"
    request("POST", saved["legacy_path"] + "/seal")
    saved["legacy_digest"] = hashlib.sha256(binary(saved["legacy_path"] + "/evidence", TOKEN)[0]).hexdigest()
    binding_id = str(uuid4())
    prepared = request("POST", PREFIX + binding_id + "/prepare", {
        "certificate_base64": encoded(Path(os.environ["TT_INDIVIDUAL_CERTIFICATE"]).read_bytes())}, token=account["token"])
    assert prepared["account_id"] == account["id"] and prepared["account_label"] == account["email"]
    assert hashlib.sha256(decoded(prepared["certificate"]["der_base64"])).hexdigest() == prepared["certificate"]["fingerprint"]
    raw = binding_bytes(prepared)
    intent = {"statement_base64": encoded(raw), "certificate_der_base64": prepared["certificate"]["der_base64"],
              "signature_base64": sign(raw, "certificate-binding.bin")}
    bad = {**intent, "signature_base64": encoded(bytes(384))}
    before = durable(saved)
    request("POST", PREFIX + binding_id + "/register", bad, token=account["token"], expected=422,
            code="signing_credential_rejected")
    assert durable(saved) == before
    binding = request("POST", PREFIX + binding_id + "/register", intent, token=account["token"])
    assert binding["statement_base64"] == intent["statement_base64"]
    assert request("GET", PREFIX + "current", token=account["token"]) == {"binding": binding}
    saved.update(binding=binding, binding_intent=intent)
    payload = content(saved, account["token"])
    pdf_signature = sign(payload, "document.pdf")
    failed, invalid = preparation(saved, str(uuid4()), encoded(bytes(384)))
    endpoint = document_path(saved) + "/individual-signing"
    before = durable(saved)
    request("POST", endpoint + "/submit", invalid, token=account["token"], expected=422,
            code="signing_credential_rejected")
    assert durable(saved) == before
    request("GET", endpoint + "/receipts/" + failed["operation_id"], token=account["token"],
            expected=404, code="signing_not_found")
    prepared, intent = preparation(saved, str(uuid4()), pdf_signature)
    saved.update(prepared=prepared, intent=intent)
    # Discard the successful POST response and recover its exact durable result.
    request("POST", endpoint + "/submit", intent, token=account["token"])
    saved["receipt"] = request("GET", endpoint + "/receipts/" + prepared["operation_id"], token=account["token"])
    assert saved["receipt"]["status"] == "accepted"
    assert saved["receipt"]["consent_base64"] == prepared["consent_base64"]
    report = history(saved, account["token"], "valid")
    assert report["binding_withdrawn_at"] is None
    saved["archive_digest"] = archive(saved, account["token"], "captured")
    saved["next_prepared"], saved["next_intent"] = preparation(saved, str(uuid4()), pdf_signature, 2)
    conflict = copy.deepcopy(intent)
    conflict["pdf_signature_base64"] = encoded(bytes(384))
    request("POST", endpoint + "/submit", conflict, token=account["token"], expected=409, code="signing_conflict")
    current = request("GET", route + "/documents/" + saved["document"]["document_id"])
    assert current["version"] == 2 and current["sealed"] is False
    saved["durable"] = durable(saved)
    assert len(saved["durable"]["bindings"]) == len(saved["durable"]["receipts"]) == 1
    assert len(saved["durable"]["events"]) == 2
    legacy(saved)
    save(saved)
    print("Individual HTTP capture passed: CSR certificate, double proof, exact V1, real TSA, public ZIP and recovered receipt.")


def rotated():
    saved = json.loads(STATE.read_text())
    token = saved["account"]["token"]
    assert durable(saved) == saved["durable"]
    report = history(saved, token, "rejected")
    assert report["certificate_current"]["reason"] == "revoked" and report["binding_withdrawn_at"] is None
    endpoint = document_path(saved).rsplit("/", 1)[0] + "/2/individual-signing"
    request("POST", endpoint + "/submit", saved["next_intent"], token=token, expected=409, code="signing_trust_changed")
    _, current = preparation(saved, str(uuid4()), saved["intent"]["pdf_signature_base64"], 2)
    request("POST", endpoint + "/submit", current, token=token, expected=422, code="signing_credential_rejected")
    assert durable(saved) == saved["durable"]
    binding_id = saved["binding"]["binding_id"]
    withdrawn = request("POST", PREFIX + binding_id + "/withdraw", token=token)
    assert withdrawn["withdrawn_at"] and {**withdrawn, "withdrawn_at": None} == saved["binding"]
    saved["binding"] = withdrawn
    assert request("GET", PREFIX + "current", token=token) == {"binding": None}
    assert request("POST", PREFIX + binding_id + "/withdraw", token=token) == withdrawn
    assert history(saved, token, "rejected")["binding_withdrawn_at"] == withdrawn["withdrawn_at"]
    assert archive(saved, token, "withdrawn") == saved["archive_digest"]
    saved["durable"] = durable(saved)
    assert len(saved["durable"]["withdrawals"]) == 1 and len(saved["durable"]["events"]) == 3
    legacy(saved)
    save(saved)
    print("Individual revocation passed: historical validity preserved, current revocation visible, withdrawal and exact replay.")


def restore():
    saved = json.loads(STATE.read_text())
    saved["account"]["token"] = login(saved["account"], 1)
    token = saved["account"]["token"]
    before = durable(saved)
    assert before == saved["durable"]
    assert request("GET", PREFIX + saved["binding"]["binding_id"], token=token) == saved["binding"]
    report = history(saved, token, "rejected")
    assert report["certificate_current"]["reason"] == "revoked"
    assert report["binding_withdrawn_at"] == saved["binding"]["withdrawn_at"]
    assert archive(saved, token, "restored") == saved["archive_digest"]
    assert durable(saved) == before
    legacy(saved)
    print("Individual restore passed: fresh MFA, original receipt and ZIP, current revoked status and immutable audited history.")


if __name__ == "__main__":
    assert len(sys.argv) == 2 and sys.argv[1] in ["capture", "rotated", "restore"]
    {"capture": capture, "rotated": rotated, "restore": restore}[sys.argv[1]]()
