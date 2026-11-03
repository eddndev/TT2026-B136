#!/usr/bin/env python3
"""Bilateral contract signing over real HTTP, storage, keys, CRLs and time-stamps.

capture: an invited Client and a member Litigator bind their own keys; the
Litigator creates a contract and a request; the Client signs and keeps a
partial package; the Litigator counter-signs and both packages pass the
evidence validator and the OpenSSL check. Rejection, decline after the
Client signed and cancellation close other requests, and notice e-mail is
attempted without leaving the machine. restore: after a dump and restore,
fresh logins read the same request, receipts and final package.
"""
import hashlib
import json
import sys
from uuid import uuid4

from api_contract_support import (
    PDF, STATE, bind, bootstrap, call, decoded, encoded, invite_client, login, openssl_two_step, psql,
    save, sign, staff, validate, zip_entries,
)


def case_with(owner, litigator):
    case = call("POST", "/api/v1/cases", {"title": "Contratos bilaterales", "reference": "CONTRACTS-API"},
                token=owner["token"], expected=201)
    call("PUT", "/api/v1/cases/" + case["id"] + "/members/" + litigator["id"], token=owner["token"], expected=204)
    return "/api/v1/cases/" + case["id"]


def contract(route, litigator, name):
    upload = call("POST", route + "/documents", PDF.read_bytes(), token=litigator["token"], expected=201,
                  headers={"X-Document-Name": name + ".pdf"})
    operation = str(uuid4())
    created = call("POST", route + "/contracts", {"operation_id": operation, "document_id": upload["id"],
                   "title": "Contrato " + name}, token=litigator["token"], expected=201)
    assert created["event"]["kind"] == "created" and created["event"]["revision"] == 1
    assert call("GET", route + "/contracts/operations/" + operation, token=litigator["token"]) == created
    return created["contract_id"]


def request(route, contract_id, litigator, client, revision):
    body = {"request_id": str(uuid4()), "operation_id": str(uuid4()), "expected_revision": revision,
            "version": 1, "client_email": client["email"], "firm_signer_id": litigator["id"], "ttl_days": 7,
            "ceremony_reference": "HF-DEMO-0001"}
    receipt = call("POST", route + "/contracts/" + contract_id + "/requests", body,
                   token=litigator["token"], expected=201)
    assert receipt["event"]["kind"] == "requested"
    statement = decoded(receipt["request_statement_base64"])
    assert len(statement) == 240 and statement[:8] == b"CTRTREQ1"
    return body["request_id"]


def path(request_id, suffix=""):
    return "/api/v1/contract-requests/" + request_id + suffix


def signed(account, party, request_id):
    """Prepares, signs locally with OpenSSL and submits one party statement."""
    operation = str(uuid4())
    prepared = call("POST", path(request_id, "/prepare"), {"operation_id": operation}, token=account["token"])
    raw = decoded(prepared["statement_base64"])
    assert len(raw) == 303 and raw[:8] == b"CTRTSIG1" and prepared["account_id"] == account["id"]
    detail = call("GET", path(request_id), token=account["token"])
    assert raw[174:206] == hashlib.sha256(decoded(detail["statement_base64"])).digest()
    assert raw[206:238] == hashlib.sha256(PDF.read_bytes()).digest()
    body = {"operation_id": operation, "expected_revision": prepared["revision"],
            "statement_base64": prepared["statement_base64"], "signature_base64": sign(party, raw, "statement.bin")}
    receipt = call("POST", path(request_id, "/sign"), body, token=account["token"])
    # A lost answer is reconciled through the exact receipt, and an identical
    # resubmission returns that same receipt.
    assert call("GET", path(request_id, "/operations/" + operation), token=account["token"]) == receipt
    assert call("POST", path(request_id, "/sign"), body, token=account["token"]) == receipt
    conflict = {**body, "signature_base64": encoded(bytes(384))}
    call("POST", path(request_id, "/sign"), conflict, token=account["token"], expected=409, code="signing_conflict")
    return receipt


def close(account, request_id, outcome, reason=None):
    revision = call("GET", path(request_id), token=account["token"])["revision"]
    body = {"operation_id": str(uuid4()), "expected_revision": revision, "outcome": outcome}
    if reason:
        body["reason"] = reason
    receipt = call("POST", path(request_id, "/close"), body, token=account["token"])
    assert call("GET", path(request_id), token=account["token"])["status"] == receipt["event"]["kind"]
    return receipt


def package(account, request_id, stage):
    archive, headers = call("GET", path(request_id, "/evidence/" + stage), token=account["token"], raw=True)
    assert headers["Content-Type"] == "application/zip"
    return archive


def signing_flow(saved, route, owner, litigator, client, outsider):
    contract_id = contract(route, litigator, "servicios")
    request_id = request(route, contract_id, litigator, client, 1)
    mine = call("GET", "/api/v1/contract-requests", token=client["token"])["requests"]
    assert [row["request_id"] for row in mine] == [request_id] and mine[0]["status"] == "awaiting_client"
    content, headers = call("GET", path(request_id, "/content"), token=client["token"], raw=True)
    assert content == PDF.read_bytes() and headers["X-Document-Digest"] == hashlib.sha256(content).hexdigest()
    call("GET", path(request_id), token=outsider["token"], expected=404, code="contract_not_found")
    call("POST", path(request_id, "/prepare"), {"operation_id": str(uuid4())}, token=litigator["token"],
         expected=409, code="contract_out_of_turn")
    first = signed(client, "client", request_id)
    assert first["event"]["kind"] == "client_signed" and first["signature"]["timestamp_base64"]
    partial = package(client, request_id, "partial")
    validate(partial, "partial.zip", "BilateralContractPartialV1", complete=False)
    openssl_two_step(zip_entries(partial), 1)
    second = signed(litigator, "firm", request_id)
    assert second["event"]["kind"] == "completed" and len(second["manifest_sha256"]) == 64
    final = package(client, request_id, "final")
    report = validate(final, "final.zip", "BilateralContractV1", complete=True)
    entries = zip_entries(final)
    assert hashlib.sha256(entries["contrato-manifiesto.json"]).hexdigest() == second["manifest_sha256"]
    openssl_two_step(entries, 1)
    openssl_two_step(entries, 2)
    assert package(owner, request_id, "final") == final
    listed = call("GET", route + "/contracts", token=litigator["token"])
    assert listed["contracts"][0]["state"] == "firmado"
    assert litigator["id"] in [row["account_id"] for row in listed["firm_signers"]]
    saved.update(request_id=request_id, final_sha256=hashlib.sha256(final).hexdigest(),
                 detail=call("GET", path(request_id), token=client["token"]),
                 completed=second, report_profile=report.splitlines()[0])


def closing_flows(saved, route, litigator, client):
    rejected = contract(route, litigator, "arrendamiento")
    first = request(route, rejected, litigator, client, 1)
    assert close(client, first, "reject")["event"]["kind"] == "rejected"
    second = request(route, rejected, litigator, client, 3)
    signed(client, "client", second)
    close(litigator, second, "decline", "Condiciones no aceptadas por el despacho")
    validate(package(client, second, "partial"), "declined.zip", "BilateralContractPartialV1", complete=False)
    call("GET", path(second, "/evidence/final"), token=client["token"], expected=409, code="contract_not_complete")
    cancelled = contract(route, litigator, "honorarios")
    third = request(route, cancelled, litigator, client, 1)
    call("POST", path(third, "/close"), {"operation_id": str(uuid4()), "expected_revision": 2,
         "outcome": "cancel"}, token=litigator["token"], expected=400, code="invalid_contract_request")
    close(litigator, third, "cancel", "Se sustituye por otro instrumento")
    abandon = {"operation_id": str(uuid4()), "expected_revision": 3, "reason": "Sin acuerdo"}
    receipt = call("POST", route + "/contracts/" + cancelled + "/abandon", abandon, token=litigator["token"])
    assert receipt["event"]["kind"] == "abandoned"
    saved["closed"] = [first, second, third]


def notices():
    rows = psql("SELECT kind||':'||delivery_state FROM contract_notifications ORDER BY created_at_seconds,id")
    kinds = [row.split(":") for row in rows.splitlines()]
    assert {kind for kind, _ in kinds} == {"requested", "client_signed", "completed", "rejected", "declined",
                                           "cancelled"}
    assert all(state != "sent" for _, state in kinds), "a notice left the isolated namespace"
    assert psql("SELECT count(*) FROM member_invitations WHERE delivery_state='sent'") == "0"
    return len(kinds)


def capture():
    owner = bootstrap()
    litigator = staff(owner, "litigator", "litigator")
    outsider = staff(owner, "outsider", "litigator")
    client = invite_client(owner)
    route = case_with(owner, litigator)
    call("PUT", route + "/members/" + client["id"], token=owner["token"], expected=422, code="invalid_input")
    saved = {"accounts": {"owner": owner, "litigator": litigator, "client": client}, "route": route}
    saved["bindings"] = {"client": bind(client, "client"), "firm": bind(litigator, "firm")}
    signing_flow(saved, route, owner, litigator, client, outsider)
    closing_flows(saved, route, litigator, client)
    saved["notices"] = notices()
    save(saved)
    print("Contract capture passed: invited Client, own keys, partial and final packages validated, "
          "rejection, decline, cancellation and %d undelivered notices." % saved["notices"])


def restore():
    saved = json.loads(STATE.read_text())
    accounts = saved["accounts"]
    for account in accounts.values():
        account["token"] = login(account, 1)
    client, litigator = accounts["client"], accounts["litigator"]
    request_id = saved["request_id"]
    assert call("GET", path(request_id), token=client["token"]) == saved["detail"]
    operation = saved["completed"]["event"]["operation_id"]
    assert call("GET", path(request_id, "/operations/" + operation), token=litigator["token"]) == saved["completed"]
    final = package(client, request_id, "final")
    assert hashlib.sha256(final).hexdigest() == saved["final_sha256"]
    validate(final, "restored.zip", "BilateralContractV1", complete=True)
    rows = call("GET", "/api/v1/contract-requests", token=client["token"])["requests"]
    assert {row["request_id"] for row in rows} == {request_id, *saved["closed"]}
    print("Contract restore passed: fresh logins, identical request, receipt and final package.")


if __name__ == "__main__":
    assert len(sys.argv) == 2 and sys.argv[1] in ("capture", "restore")
    {"capture": capture, "restore": restore}[sys.argv[1]]()
