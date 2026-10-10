"""Bounded public evidence checks for disposable individual signing acceptance."""
import base64
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
from urllib.request import Request, urlopen
from uuid import UUID
import zipfile

from api_procedural_facts_support import BASE, TOKEN, WORK, REPO, request, upload
from api_resource_hearings_support import enroll, login

STATE = WORK / "individual-signing-api-state.json"
PREFIX = "/api/v1/identity/signing-certificates/"


def command(arguments):
    result = subprocess.run(arguments, capture_output=True, timeout=60, check=False)
    assert result.returncode == 0, "Individual signing fixture command failed"
    return result.stdout


def encoded(value):
    return base64.b64encode(value).decode("ascii")


def decoded(value):
    raw = base64.b64decode(value, validate=True)
    assert encoded(raw) == value, "Noncanonical public base64"
    return raw


def prefix(value, context, purpose):
    fingerprint = value.get("certificate_fingerprint", context["certificate"]["fingerprint"])
    return (purpose + b"\x01\x01" + UUID(context["deployment_id"]).bytes
            + bytes.fromhex(context["root_fingerprint"]) + int(value["trust_revision"]).to_bytes(4, "big")
            + UUID(value["account_id"]).bytes + int(value["account_revision"]).to_bytes(8, "big")
            + int(value["auth_generation"]).to_bytes(8, "big") + UUID(value["binding_id"]).bytes
            + bytes.fromhex(fingerprint))


def binding_bytes(value):
    raw = prefix(value, value, b"DOCBIND1")
    assert len(raw) == 142 and decoded(value["statement_base64"]) == raw
    return raw


def consent_bytes(value, binding):
    raw = (prefix(value, binding, b"DOCSIGN1") + UUID(value["operation_id"]).bytes
           + UUID(value["case_id"]).bytes + UUID(value["document_id"]).bytes
           + int(value["version"]).to_bytes(4, "big") + bytes.fromhex(value["document_digest"])
           + bytes.fromhex(value["pdf_signature_digest"]))
    assert len(raw) == 258 and decoded(value["consent_base64"]) == raw
    return raw


def sign(raw, name):
    path = WORK / "individual-client" / name
    path.write_bytes(raw)
    result = command(["openssl", "dgst", "-sha256", "-sign",
                      os.environ["TT_INDIVIDUAL_KEY"], str(path)])
    assert len(result) == 384
    return encoded(result)


def save(value):
    with os.fdopen(os.open(STATE, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600), "w") as stream:
        os.fchmod(stream.fileno(), 0o600)
        json.dump(value, stream, sort_keys=True, ensure_ascii=True)
        stream.write("\n")


def document_path(saved):
    row = saved["document"]
    return f'/api/v1/cases/{saved["case_id"]}/documents/{row["document_id"]}/versions/{row["version"]}'


def binary(path, token):
    query = Request(BASE + path, headers={"Authorization": "Bearer " + token})
    with urlopen(query, timeout=60) as response:
        value = response.read(16 * 1024 * 1024 + 1)
        assert len(value) <= 16 * 1024 * 1024 and response.status == 200
        assert response.headers["Cache-Control"] == "no-store"
        return value, response.headers


def content(saved, token):
    value, headers = binary(document_path(saved) + "/content", token)
    row = saved["document"]
    assert headers["X-Document-Id"] == row["document_id"]
    assert headers["X-Case-Id"] == saved["case_id"]
    assert headers["X-Document-Version"] == str(row["version"])
    assert hashlib.sha256(value).hexdigest() == headers["X-Document-Digest"] == row["digest"]
    assert value == (REPO / "crates/infrastructure/tests/fixtures/stage-support.pdf").read_bytes()
    return value


def archive(saved, token, label):
    value, _ = binary(document_path(saved) + "/evidence", token)
    directory = WORK / "individual-client" / label
    directory.mkdir(mode=0o700, exist_ok=True)
    with zipfile.ZipFile(io.BytesIO(value)) as archive:
        names = archive.namelist()
        assert len(names) == len(set(names)) and len(names) <= 32
        assert sum(item.file_size for item in archive.infolist()) <= 16 * 1024 * 1024
        for item in archive.infolist():
            assert Path(item.filename).name == item.filename and item.file_size <= 16 * 1024 * 1024
            (directory / item.filename).write_bytes(archive.read(item))
    pdf = directory / "individual-selected.pdf"
    assert hashlib.sha256(pdf.read_bytes()).hexdigest() == saved["document"]["digest"]
    assert (directory / "document-consent.bin").read_bytes() == decoded(saved["intent"]["consent_base64"])
    assert (directory / "certificate-binding.bin").read_bytes() == binding_bytes(saved["binding"])
    public = directory / "holder.pub.pem"
    public.write_bytes(command(["openssl", "x509", "-in", str(directory / "certificado.pem"), "-pubkey", "-noout"]))
    for name in ["individual-selected.pdf", "certificate-binding.bin", "document-consent.bin"]:
        signature = name + ".sig" if name.endswith(".pdf") else name.removesuffix(".bin") + ".sig"
        command(["openssl", "dgst", "-sha256", "-verify", str(public), "-signature", str(directory / signature), str(directory / name)])
    command(["openssl", "ts", "-verify", "-data", str(pdf), "-in", str(directory / "individual-selected.pdf.tsr"),
             "-CAfile", str(directory / "tsa-chain.pem")])
    proof = json.loads((directory / "individual-signing.json").read_text())
    assert bytes(proof["consent_signature"]) == decoded(saved["intent"]["consent_signature_base64"])
    captured_trust = directory / "acceptance-trust.pem"
    captured_trust.write_bytes((directory / "ca.pem").read_bytes() + (directory / "crl.pem").read_bytes())
    command(["openssl", "verify", "-attime", str(proof["accepted_at"]), "-crl_check",
             "-CAfile", str(captured_trust), str(directory / "certificado.pem")])
    return hashlib.sha256(value).hexdigest()


def durable(saved):
    signer = str(UUID(saved["account"]["id"]))
    query = f"""WITH bindings AS (SELECT * FROM document_signing_bindings WHERE signer_id='{signer}'),
      withdrawals AS (SELECT * FROM document_signing_withdrawals WHERE binding_id IN (SELECT binding_id FROM bindings)),
      receipts AS (SELECT * FROM individual_document_signatures WHERE signer_id='{signer}')
      SELECT jsonb_build_object(
        'bindings',COALESCE((SELECT jsonb_agg(to_jsonb(b) ORDER BY binding_id) FROM bindings b),'[]'),
        'withdrawals',COALESCE((SELECT jsonb_agg(to_jsonb(w) ORDER BY binding_id) FROM withdrawals w),'[]'),
        'receipts',COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY operation_id) FROM receipts r),'[]'),
        'events',COALESCE((SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a WHERE sequence IN
          (SELECT audit_sequence FROM bindings UNION SELECT audit_sequence FROM withdrawals UNION SELECT audit_sequence FROM receipts)),'[]'))"""
    return json.loads(command(["psql", os.environ["TT_INDIVIDUAL_DATABASE"], "-X", "-qAt", "-v", "ON_ERROR_STOP=1", "-c", query]))


def history(saved, token, current):
    route = document_path(saved) + "/individual-signing"
    receipt = request("GET", route + "/receipts/" + saved["prepared"]["operation_id"], token=token)
    assert receipt == saved["receipt"]
    assert request("POST", route + "/submit", saved["intent"], token=token) == receipt
    report = request("GET", route + "/verification", token=token)
    assert report["kind"] == "individual" and report["signing"] == receipt
    assert report["historical"]["verdict"] == "valid"
    assert report["certificate_current"]["status"] == current
    assert request("GET", "/api/v1/audit/verify")["valid"] is True
    return report
