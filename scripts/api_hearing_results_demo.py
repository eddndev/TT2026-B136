"""Verify declared hearing sessions inside the disposable HTTP restore campaign."""

import copy
from uuid import uuid4


def command(hearing_id, anchor_revision, values, continuation=None):
    return {"operation_id": str(uuid4()), "hearing_id": hearing_id, "result_id": str(uuid4()),
            "change": {"action": "record", "expected_revision": 0,
                       "anchor_revision": anchor_revision, "continuation": continuation,
                       "values": values}}


def capture_results(api, route, hearing_id, closed_route, tokens):
    request = api.request
    prefix = route + "/hearings/" + hearing_id + "/results"

    def prepare(value, token=api.TOKEN):
        path = route + "/hearings/" + value["hearing_id"] + "/results/prepare"
        return request("POST", path, value, token=token)

    def submit(draft, status=201, token=api.TOKEN, code=None):
        value = draft["command"]
        action = value["change"]["action"]
        path = route + "/hearings/" + value["hearing_id"] + "/results"
        if action != "record":
            path += "/" + value["result_id"]
        if action == "withdraw":
            path += "/withdrawal"
        return request("PUT" if action == "correct" else "POST", path,
                       {"command": value, "expected_submission_digest": draft["submission_digest"]},
                       status, token=token, code=code)

    users = {}
    for role, token in tokens.items():
        users[role] = request("GET", "/api/v1/auth/me", token=token)["id"]
        request("PUT", route + "/members/" + users[role], expected=204)
    support = api.upload(route, "declared-session.pdf")
    document = route + "/documents/" + support["document_id"]
    request("POST", document + "/versions/1/seal")
    request("POST", document + "/versions?expected_version=1", b"later synthetic version", 201,
            headers={"X-Document-Name": "later-session.txt"})
    person = request("POST", route + "/participants",
                     {"display_name": "Historical attendee", "procedural_role": "Witness"}, 201)
    archived = request("PUT", route + "/participants/" + person["id"] + "/directory-status",
                       {"expected_revision": 1, "directory_status": "archived"})
    values = {"occurrence": "occurred", "extent": "partial",
              "event_time": {"precision": "date", "date": "2026-09-01", "offset": "-06:00"},
              "summary": " Declared partial session\r\nCaptured after the event ",
              "attendees": [{"participant_id": person["id"], "revision": archived["revision"],
                             "capacity": " Declared witness ", "observation": "Reported attendance"}],
              "agreements": [{"id": str(uuid4()), "text": "Explicit agreement without computed deadlines"}],
              "provenance": {"kind": "oral_reference", "reference": "Operator communication", "support": support}}
    original = command(hearing_id, 1, values)
    request("POST", prefix + "/prepare", original, 403, token=tokens["paralegal"])
    request("GET", prefix, expected=403, token=tokens["client"])
    draft = prepare(original, tokens["litigator"])
    assert prepare(original, tokens["litigator"]) == draft
    assert request("GET", prefix)["results"] == []
    assert draft["attendees"][0]["directory_status"] == "archived"
    assert draft["support"]["version"] == 1 and draft["support"]["format"] == "pdf"
    submit({**draft, "submission_digest": "00" * 32}, 409,
           token=tokens["litigator"], code="hearing_result_submission_mismatch")
    first = submit(draft, token=tokens["litigator"])
    path = prefix + "/" + first["id"]
    assert first["values"]["summary"] == "Declared partial session\nCaptured after the event"
    assert first["values"]["attendees"][0]["capacity"] == "Declared witness"
    assert first["receipt"]["submission_digest"] == draft["submission_digest"]
    assert request("GET", path + "/revisions/1") == first
    submit(draft, 409, token=tokens["litigator"])
    request("GET", prefix + "/" + str(uuid4()), expected=404, code="hearing_result_not_found")

    # The programming head may change without replacing the exact result anchor.
    hearing_path = route + "/hearings/" + hearing_id
    old = request("GET", hearing_path)
    current = request("GET", route + "/hearings/context")
    replacement = api.schedule(current)
    replacement["hearing_id"] = hearing_id
    replacement["change"].update(action="replace", expected_revision=1, reason="Office adjustment")
    replacement["change"]["values"] = copy.deepcopy(old["values"])
    replacement["change"]["values"]["scheduled_at"] = "2026-10-03T10:00:00-06:00"
    api.submit(route, api.prepare(route, replacement))
    cancellation = {"operation_id": str(uuid4()), "hearing_id": hearing_id,
                    "change": {"action": "cancel", "expected_revision": 2, "reason": "Organizational cancellation"}}
    api.submit(route, api.prepare(route, cancellation))
    assert request("GET", path + "/revisions/1") == first
    api.advance(route, support, "intermediate")
    correction = {"operation_id": str(uuid4()), "hearing_id": hearing_id, "result_id": first["id"],
                  "change": {"action": "correct", "expected_revision": 1,
                             "values": first["values"], "reason": "Explicit corrected declaration"}}
    competing = copy.deepcopy(correction)
    competing["operation_id"] = str(uuid4())
    loser = prepare(competing)
    second = submit(prepare(correction))
    assert second["revision"] == 2 and second["anchor"] == first["anchor"]
    submit(loser, 409, code="hearing_result_revision_conflict")
    assert request("GET", path + "/revisions/1") == first
    withdrawal = {"operation_id": str(uuid4()), "hearing_id": hearing_id, "result_id": first["id"],
                  "change": {"action": "withdraw", "expected_revision": 2, "reason": "Administrative withdrawal"}}
    withdrawn = submit(prepare(withdrawal))
    assert withdrawn["status"] == "withdrawn" and withdrawn["values"] == first["values"]
    assert withdrawn["support"] == first["support"]
    request("POST", prefix + "/prepare", {**withdrawal, "operation_id": str(uuid4()),
            "change": {**withdrawal["change"], "expected_revision": 3}}, 409,
            code="hearing_result_already_withdrawn")

    # A continuation is another root and can cite an exact withdrawn antecedent.
    current = request("GET", route + "/hearings/context")
    later = api.submit(route, api.prepare(route, api.schedule(current, "intermediate")))
    next_values = {**copy.deepcopy(first["values"]), "occurrence": "not_started", "extent": "unspecified"}
    continuation = command(later["id"], 1, next_values, {"result_id": first["id"], "revision": 3})
    follow = submit(prepare(continuation))
    assert follow["continuation"]["status"] == "withdrawn"
    assert follow["continuation"]["hearing_id"] == hearing_id
    assert follow["continuation"]["submission_digest"] == withdrawn["receipt"]["submission_digest"]
    other = submit(prepare(command(hearing_id, 3, next_values)))
    assert other["anchor"]["status"] == "cancelled" and other["anchor"]["revision"] == 3
    page = request("GET", prefix + "?limit=1")
    assert len(page["results"]) == 1 and page["has_more"]
    tail = request("GET", prefix + "?after_id=" + page["next_after_id"])
    assert len(tail["results"]) == 1 and not tail["has_more"]
    assert "values" not in page["results"][0]
    history = request("GET", path + "/history?limit=2")
    assert [row["revision"] for row in history["revisions"]] == [3, 2]
    assert all("values" not in row and "attendees" not in row for row in history["revisions"])
    assert request("GET", path + "/history?before_revision=2")["revisions"][0]["revision"] == 1
    assert len(request("GET", prefix + "?status=withdrawn")["results"]) == 1
    missing = command(hearing_id, 99, next_values)
    request("POST", prefix + "/prepare", missing, 404, code="hearing_result_reference_not_found")
    request("GET", closed_route + "/hearings/" + hearing_id + "/results/" + first["id"],
            expected=404)

    # Current authorization is checked again after a proposal has been prepared.
    pending = prepare(command(hearing_id, 1, next_values), tokens["litigator"])
    request("DELETE", route + "/members/" + users["litigator"], expected=204)
    submit(pending, 404, token=tokens["litigator"], code="case_not_found")
    request("PUT", route + "/members/" + users["litigator"], expected=204)
    request("PUT", route + "/administrative-status", {"expected_revision": 1, "administrative_status": "closed"})
    submit(pending, 409, token=tokens["litigator"], code="case_closed")
    assert request("GET", path, token=tokens["paralegal"])["status"] == "withdrawn"
    request("PUT", route + "/administrative-status", {"expected_revision": 2, "administrative_status": "active"})
    assert request("GET", "/api/v1/audit/verify")["valid"]
    print("Hearing results API passed: sessions, sources, revalidation, receipts, history and role isolation.")
    return [prefix, prefix + "?status=withdrawn", path, path + "/history",
            *(path + "/revisions/" + str(revision) for revision in [1, 2, 3]),
            prefix + "/" + other["id"],
            route + "/hearings/" + later["id"] + "/results/" + follow["id"]]
