"""Independent CTRTREQ1/CTRTSIG1 vectors; no Rust code or encoder is invoked.

The layouts are those of docs/adr/0086-bilateral-contract-signing.md. The
position 1 signature and its time-stamp response are fixed placeholder bytes,
not a valid signature or token: the vector pins statement bytes only.
"""
from hashlib import sha256
from pathlib import Path
from uuid import UUID
import json
import struct


def uuid(text):
    return UUID(text).bytes


def digest(data):
    return sha256(data).digest()


DOCUMENT = b"%PDF-1.7\n% bilateral contract vector\n%%EOF\n"
CLIENT_CERTIFICATE = b"client certificate DER fixture"
FIRM_CERTIFICATE = b"firm signer certificate DER fixture"
ROOT_CERTIFICATE = b"credential root certificate DER fixture"
CLIENT_SIGNATURE = bytes([0x5A]) * 384
CLIENT_TOKEN = b"position 1 time-stamp response fixture"

REQUEST = {
    "deployment_id": "8f2b6c1e-0d4a-4e57-9b3c-1a2d3e4f5061",
    "request_id": "c0a80101-7e3f-4b2a-8c1d-2e3f4a5b6c7d",
    "case_id": "1b9d6bcd-bbfd-4b2d-9b5d-ab8dfbbd4bed",
    "document_id": "6ec0bd7f-11c0-43da-975e-2a8ad9ebae0b",
    "document_version": 7,
    "document_sha256": digest(DOCUMENT).hex(),
    "client_account_id": "3d6f4a52-9e1b-4c7d-8a0f-5b2c1d3e4f60",
    "firm_account_id": "a4e5f607-1829-4a3b-9c4d-5e6f708192a3",
    "requester_account_id": "b5f60718-293a-4b4c-8d5e-6f708192a3b4",
    "client_certificate_sha256": digest(CLIENT_CERTIFICATE).hex(),
    "firm_certificate_sha256": digest(FIRM_CERTIFICATE).hex(),
    "created_at_seconds": 1793577600,
    "expires_at_seconds": 1793577600 + 7 * 86400,
}

ROOT_SHA256 = digest(ROOT_CERTIFICATE).hex()

CLIENT_PARTY = {
    "position": 1,
    "account_id": REQUEST["client_account_id"],
    "account_revision": 5,
    "authentication_generation": 2,
    "binding_id": "d6071829-3a4b-4c5d-9e6f-708192a3b4c5",
    "certificate_sha256": REQUEST["client_certificate_sha256"],
    "root_sha256": ROOT_SHA256,
    "trust_revision": 3,
    "operation_id": "e718293a-4b5c-4d6e-8f70-8192a3b4c5d6",
    "prior_signature_sha256": "00" * 32,
    "prior_token_sha256": "00" * 32,
}

FIRM_PARTY = {
    "position": 2,
    "account_id": REQUEST["firm_account_id"],
    "account_revision": 11,
    "authentication_generation": 4,
    "binding_id": "f8293a4b-5c6d-4e7f-9081-92a3b4c5d6e7",
    "certificate_sha256": REQUEST["firm_certificate_sha256"],
    "root_sha256": ROOT_SHA256,
    "trust_revision": 4,
    "operation_id": "093a4b5c-6d7e-4f80-8192-a3b4c5d6e7f8",
    "prior_signature_hex": CLIENT_SIGNATURE.hex(),
    "prior_signature_sha256": digest(CLIENT_SIGNATURE).hex(),
    "prior_token_hex": CLIENT_TOKEN.hex(),
    "prior_token_sha256": digest(CLIENT_TOKEN).hex(),
}


def request_bytes(values):
    data = (
        b"CTRTREQ1"
        + bytes([1, 1])
        + uuid(values["deployment_id"])
        + uuid(values["request_id"])
        + uuid(values["case_id"])
        + uuid(values["document_id"])
        + struct.pack(">I", values["document_version"])
        + bytes.fromhex(values["document_sha256"])
        + uuid(values["client_account_id"])
        + uuid(values["firm_account_id"])
        + uuid(values["requester_account_id"])
        + bytes.fromhex(values["client_certificate_sha256"])
        + bytes.fromhex(values["firm_certificate_sha256"])
        + struct.pack(">q", values["created_at_seconds"])
        + struct.pack(">q", values["expires_at_seconds"])
        + bytes([2, 1])
    )
    assert len(data) == 240
    return data


def statement_bytes(party, request, deployment):
    prefix = (
        b"CTRTSIG1"
        + bytes([1, 1])
        + uuid(deployment)
        + bytes.fromhex(party["root_sha256"])
        + struct.pack(">I", party["trust_revision"])
        + uuid(party["account_id"])
        + struct.pack(">Q", party["account_revision"])
        + struct.pack(">Q", party["authentication_generation"])
        + uuid(party["binding_id"])
        + bytes.fromhex(party["certificate_sha256"])
    )
    assert len(prefix) == 142
    data = (
        prefix
        + uuid(party["operation_id"])
        + uuid(REQUEST["request_id"])
        + digest(request)
        + bytes.fromhex(REQUEST["document_sha256"])
        + bytes([party["position"]])
        + bytes.fromhex(party["prior_signature_sha256"])
        + bytes.fromhex(party["prior_token_sha256"])
    )
    assert len(data) == 303
    return data


def encoded(data):
    return {"length": len(data), "hex": data.hex(), "sha256": digest(data).hex()}


def build_vectors():
    request = request_bytes(REQUEST)
    client = statement_bytes(CLIENT_PARTY, request, REQUEST["deployment_id"])
    firm = statement_bytes(FIRM_PARTY, request, REQUEST["deployment_id"])
    assert client[239:303] == bytes(64)
    assert firm[239:271] == digest(CLIENT_SIGNATURE)
    assert firm[271:303] == digest(CLIENT_TOKEN)
    assert client[174:206] == firm[174:206] == digest(request)
    return {
        "request_inputs": dict(REQUEST, document_hex=DOCUMENT.hex()),
        "client_party": CLIENT_PARTY,
        "firm_party": FIRM_PARTY,
        "request": encoded(request),
        "client_statement": encoded(client),
        "firm_statement": encoded(firm),
    }


if __name__ == "__main__":
    vectors = build_vectors()
    Path(__file__).with_name("contract_statement_vectors.json").write_text(
        json.dumps(vectors, ensure_ascii=True, indent=2) + "\n", encoding="ascii")
    print(json.dumps({name: vectors[name]["sha256"]
                      for name in ("request", "client_statement", "firm_statement")}))
