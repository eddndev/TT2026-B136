#!/usr/bin/env python3
"""Exercise authorized hearing commands and exact historical responses over HTTP."""

import copy
import json
import os
from pathlib import Path
import sys
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen
from uuid import uuid4

from api_hearing_results_demo import capture_results

BASE = os.environ["TT_HEARING_API_BASE_URL"]
TOKEN = os.environ["TT_HEARING_API_TOKEN"]
WORK = Path(os.environ["TT_HEARING_API_WORK_DIR"])
REPO = Path(os.environ["TT_HEARING_API_REPO"])
STATE = WORK / "hearing-api-state.json"
INTERVAL = {"from": "2026-10-01T00:00:00Z", "until": "2026-11-01T00:00:00Z", "status": "all"}


def request(method, path, body=None, expected=200, token=TOKEN, headers=None, code=None):
    metadata = {"Authorization": "Bearer " + token}
    if isinstance(body, dict):
        body = json.dumps(body).encode("utf-8")
        metadata["Content-Type"] = "application/json"
    metadata.update(headers or {})
    query = Request(BASE + path, data=body, headers=metadata, method=method)
    try:
        response = urlopen(query, timeout=60)
    except HTTPError as error:
        response = error
    with response:
        raw = response.read(2 * 1024 * 1024)
        result = json.loads(raw) if raw else None
        actual_code = (result or {}).get("error", {}).get("code")
        assert response.status == expected, (method, path, response.status, expected, actual_code)
        if code:
            assert actual_code == code, (path, actual_code, code)
    return result


def enroll(role):
    email = "hearing-" + role + "@example.com"
    password = "synthetic hearing fixture password"
    enrollment = request("POST", "/api/v1/users", {"email": email, "password": password, "role": role}, 201)
    challenge = request("POST", "/api/v1/auth/login", {"email": email, "password": password})
    session = request("POST", "/api/v1/auth/mfa/recovery", {
        "challenge_token": challenge["challenge_token"], "code": enrollment["recovery_codes"][0],
    })
    return enrollment["user"]["id"], session["access_token"]


def penal(label):
    return request("POST", "/api/v1/penal-cases", {
        "title": "Synthetic hearing " + label, "reference": "HEARING-" + label,
        "profile": {"nuc": "NUC-HEARING-" + label, "nuc_authority": "Declared authority",
                    "judicial_case_number": "CJ-HEARING-" + label, "judicial_authority": "Declared court",
                    "offenses": ["Synthetic offense"], "general_information": None,
                    "complementary_identifiers": None},
    }, 201)["id"]


def upload(route, name="synthetic-support.pdf"):
    pdf = (REPO / "crates/infrastructure/tests/fixtures/stage-support.pdf").read_bytes()
    document = request("POST", route + "/documents", pdf, 201, headers={"X-Document-Name": name})
    return {"document_id": document["id"], "version": document["version"], "digest": document["digest"]}


def schedule(context, kind="initial", participants=None):
    return {"operation_id": str(uuid4()), "hearing_id": str(uuid4()), "change": {
        "action": "schedule", "expected_revision": 0,
        "expected_case_revision": context["case_revision"],
        "expected_stage_revision": context["stage_revision"],
        "values": {"kind": kind, "scheduled_at": "2026-10-01T09:00:00-06:00", "modality": "in_person",
                   "venue": " Synthetic court ", "note": " First line\r\nSecond line ",
                   "participants": participants or [], "conviction_basis": None},
    }}


def prepare(route, command, token=TOKEN):
    return request("POST", route + "/hearings/prepare", command, token=token)


def submit(route, draft, expected=201, token=TOKEN, code=None):
    command = draft["command"]
    action = command["change"]["action"]
    suffix = "" if action == "schedule" else "/" + command["hearing_id"]
    if action == "cancel":
        suffix += "/cancellation"
    method = "PUT" if action == "replace" else "POST"
    return request(method, route + "/hearings" + suffix, {
        "command": command, "expected_submission_digest": draft["submission_digest"],
    }, expected, token=token, code=code)


def typed_participant(route, support):
    locator = {**support, "locator": "Page 1 of synthetic fixture"}
    values = {"kind": "natural_person", "name": {"state": "known", "value": "Original Hearing Person"},
              "curp": {"state": "unknown", "reason": "Synthetic fixture"}, "identity_support": locator}
    review = request("POST", route + "/participants/proposals/review", {
        "subject": {"operation": "create", "values": values}, "participant": {"operation": "create"},
        "role": {"organization": None, "legal_status": None,
                 "profile": {"kind": "defendant", "custody": {"state": "unknown", "reason": "Synthetic fixture"}},
                 "role_support": locator}, "certificate_base64": None,
    })
    assert review["candidates"] == []
    body = {"proposal": review["proposal"], "review": {"directory_stamp": review["directory_stamp"],
            "selection_reason": "Reviewed synthetic hearing identity", "different": []}, "certificate_base64": None}
    request("POST", route + "/participants/proposals/prepare", body)
    result = request("POST", route + "/participants/proposals/commit", {"prepared": body, "signature_base64": None}, 201)
    return result, values


def change_subject(route, participant, values):
    values = copy.deepcopy(values)
    values["name"]["value"] = "Updated Hearing Person"
    path = route + "/subjects/" + participant["subject"]["id"]
    body = {"expected_revision": 1, "values": values}
    review = request("POST", path + "/review", body)
    assert review["candidates"] == []
    body["review"] = {"directory_stamp": review["directory_stamp"], "selection_reason": "Synthetic correction", "different": []}
    assert request("PUT", path, body)["revision"] == 2


def advance(route, support, target):
    at = {"precision": "instant", "at": "2026-09-01T10:00:00-06:00"}
    body = {"expected_revision": 1, "target": "intermediate", "accusation_declared_at": at,
            "accusation": support, "note": "Synthetic stage evidence"}
    if target == "trial":
        body = {"expected_revision": 2, "target": "trial", "opening_order_issued_at": at,
                "opening_order": support, "received_at": at, "receiving_court": "Synthetic trial court",
                "receipt_reference": None, "receipt_support": None, "note": None}
    return request("POST", route + "/stage/transitions", body, 201)


def sentencing_fixture():
    route = "/api/v1/cases/" + penal("SENTENCING")
    support = upload(route, "declared-conviction.pdf")
    advance(route, support, "intermediate")
    advance(route, support, "trial")
    document = route + "/documents/" + support["document_id"]
    request("POST", document + "/versions/1/seal")
    request("POST", document + "/versions?expected_version=1", b"later synthetic version", 201,
            headers={"X-Document-Name": "later.txt"})
    command = schedule(request("GET", route + "/hearings/context"), "sentencing")
    request("POST", route + "/hearings/prepare", command, 422)
    command["change"]["values"]["conviction_basis"] = {
        "statement": "Operator declaration backed by synthetic fixture, not a judicial certification", "support": support}
    draft = prepare(route, command)
    result = submit(route, draft)
    assert result["support"]["version"] == 1 and result["support"]["format"] == "pdf"
    assert result["scheduling_context"]["stage"] == "trial"
    assert result["scheduling_context"]["stage_digest"] is not None
    return route, result["id"]


def capture():
    actor = request("GET", "/api/v1/auth/me")["id"]
    route = "/api/v1/cases/" + penal("MAIN")
    other = "/api/v1/cases/" + penal("FOREIGN")
    tokens = {}
    for role in ["litigator", "paralegal", "client"]:
        user, tokens[role] = enroll(role)
        request("PUT", route + "/members/" + user, expected=204)
    context = request("GET", route + "/hearings/context", token=tokens["litigator"])
    assert context["case_revision"] == context["stage_revision"] == 1
    assert context["stage"] == "investigation" and context["stage_values_digest"] is None
    support = upload(route)
    manual = request("POST", route + "/participants", {"display_name": "Original Witness", "procedural_role": "Witness"}, 201)
    typed, subject_values = typed_participant(route, support)
    refs = [{"participant_id": person["id"], "revision": 1} for person in [manual, typed]]
    command = schedule(context, participants=refs)
    request("POST", route + "/hearings/prepare", command, 403, token=tokens["paralegal"])
    request("GET", route + "/hearings/context", expected=403, token=tokens["client"])
    draft = prepare(route, command)
    assert prepare(route, command) == draft
    assert request("GET", route + "/hearings")["hearings"] == []
    wrong = {**draft, "submission_digest": "00" * 32}
    submit(route, wrong, 409, code="hearing_submission_mismatch")
    first = submit(route, draft)
    assert first["revision"] == 1 and first["recorded_by"]["id"] == actor
    assert first["receipt"]["operation_id"] == command["operation_id"]
    assert first["receipt"]["submission_digest"] == draft["submission_digest"]
    hearing = route + "/hearings/" + first["id"]
    assert request("GET", hearing + "/revisions/1") == first
    submit(route, draft, 409)
    request("GET", other + "/hearings/" + first["id"], expected=404)
    hidden = request("GET", other + "/hearings", expected=404, token=tokens["litigator"])
    unknown = request("GET", "/api/v1/cases/" + str(uuid4()) + "/hearings", expected=404, token=tokens["litigator"])
    assert hidden == unknown
    participant_path = route + "/participants/" + manual["id"]
    request("PUT", participant_path, {"expected_revision": 1, "display_name": "Edited Witness",
            "procedural_role": "Witness", "organization": None, "legal_status": None, "directory_status": "active"})
    request("PUT", participant_path + "/directory-status", {"expected_revision": 2, "directory_status": "archived"})
    change_subject(route, typed, subject_values)
    assert request("GET", hearing + "/revisions/1") == first
    request("POST", route + "/hearings/prepare", schedule(context, participants=refs), 409, code="hearing_participant_changed")
    replace = {"operation_id": str(uuid4()), "hearing_id": first["id"], "change": {
        **copy.deepcopy(draft["command"]["change"]), "action": "replace", "expected_revision": 1, "reason": "Office changed communicated date"}}
    replace["change"]["values"]["scheduled_at"] = "2026-10-02T10:00:00-06:00"
    stale = copy.deepcopy(replace)
    stale["change"]["expected_revision"] = 2
    request("POST", route + "/hearings/prepare", stale, 409, code="hearing_revision_conflict")
    stale = copy.deepcopy(replace)
    stale["change"]["expected_case_revision"] = 99
    request("POST", route + "/hearings/prepare", stale, 409, code="hearing_context_conflict")
    second = submit(route, prepare(route, replace))
    assert second["revision"] == 2 and second["participants"] == first["participants"]
    advance(route, support, "intermediate")
    cancel = {"operation_id": str(uuid4()), "hearing_id": first["id"], "change": {
        "action": "cancel", "expected_revision": 2, "reason": "Organizational cancellation"}}
    cancelled = submit(route, prepare(route, cancel))
    assert cancelled["revision"] == 3 and cancelled["status"] == "cancelled"
    assert cancelled["values"] == second["values"] and cancelled["participants"] == first["participants"]
    assert cancelled["scheduling_context"] == first["scheduling_context"]
    assert request("GET", hearing + "/revisions/1") == first
    assert request("GET", hearing + "/revisions/2") == second
    history = request("GET", hearing + "/history?limit=2")
    assert [item["revision"] for item in history["revisions"]] == [3, 2] and history["has_more"]
    assert request("GET", hearing + "/history?before_revision=2")["revisions"] == [first]
    current = request("GET", route + "/hearings/context")
    prepared_before_close = prepare(route, schedule(current, "intermediate"))
    request("PUT", route + "/administrative-status", {"expected_revision": 1, "administrative_status": "closed"})
    submit(route, prepared_before_close, 409, code="case_closed")
    assert request("GET", hearing, token=tokens["paralegal"])["status"] == "cancelled"
    foreign = submit(other, prepare(other, schedule(request("GET", other + "/hearings/context"))))
    trial_route, trial_id = sentencing_fixture()
    agenda_path = "/api/v1/hearings?" + urlencode(INTERVAL)
    staff = request("GET", agenda_path, token=tokens["litigator"])["hearings"]
    assert len(staff) == 1 and staff[0]["id"] == first["id"] and staff[0]["case_status"] == "closed"
    request("GET", agenda_path, expected=403, token=tokens["client"])
    paged = []
    params = {**INTERVAL, "limit": 1}
    while True:
        page = request("GET", "/api/v1/hearings?" + urlencode(params))
        paged.extend(item["id"] for item in page["hearings"])
        if not page["has_more"]:
            break
        cursor = page["next_after"]
        params.update(after_time=cursor["at"], after_id=cursor["id"])
        assert len(paged) <= 3
    assert len(paged) == len(set(paged)) == 3
    assert set(paged) == {first["id"], foreign["id"], trial_id}
    result_paths = capture_results(sys.modules[__name__], other, foreign["id"], route, tokens)
    paths = [route + "/hearings/context", route + "/hearings?status=all", hearing,
             hearing + "/history", hearing + "/revisions/1", hearing + "/revisions/2", hearing + "/revisions/3",
             other + "/hearings/" + foreign["id"], trial_route + "/hearings/" + trial_id,
             participant_path + "/revisions/1", route + "/subjects/" + typed["subject"]["id"], agenda_path] + result_paths
    STATE.write_text(json.dumps({"records": {path: request("GET", path) for path in paths}}, sort_keys=True), encoding="utf-8")
    assert request("GET", "/api/v1/audit/verify")["valid"]
    print("Hearing API passed: receipts, history, typed identities, archive, stages, exact sealed support, permissions and agenda.")


def restore():
    state = json.loads(STATE.read_text(encoding="utf-8"))
    for path, expected in state["records"].items():
        assert request("GET", path) == expected, "Restored hearing response differs: " + path
    assert request("GET", "/api/v1/audit/verify")["valid"]
    print("Hearing restore passed: exact revisions, receipts, participant identities, supports and agenda.")


if __name__ == "__main__":
    {"capture": capture, "restore": restore}[sys.argv[1]]()
