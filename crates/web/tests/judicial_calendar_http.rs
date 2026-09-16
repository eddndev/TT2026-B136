#[allow(dead_code)]
mod judicial_calendar_support;
use judicial_calendar_support::*;
use serde_json::json;
use std::sync::Arc;
#[tokio::test]
async fn prepare_returns_normalized_values_command_and_exact_receipt_identity() {
    let (status, body, calls) = raw(command_json().to_string()).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(calls, 1);
    assert_eq!(body["actor_id"], actor().to_string());
    assert_eq!(body["command"]["calendar_id"], ID);
    assert_eq!(body["result_revision"], 1);
    assert_eq!(
        body["values"],
        vector("leap_unicode_unordered")["normalized"]
    );
    assert_eq!(body["command"]["change"]["values"], body["values"]);
    assert_eq!(body["initial_scope"], body["values"]["scope"]);
    assert_eq!(body["submission_digest"], digest().to_hex());
}
#[tokio::test]
async fn matching_mutation_routes_return_created_historical_receipts() {
    for action in ["publish", "replace", "retire"] {
        let (method, path, payload) = mutation(action);
        let (status, body) = request(
            Arc::new(Workflow::default()),
            method,
            &path,
            Some("owner"),
            Some(payload.to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(status, 201, "{body}");
        assert_eq!(body["id"], ID);
        assert_eq!(body["revision"], if action == "publish" { 1 } else { 2 });
        assert_eq!(body["receipt"]["action"], action);
        assert_eq!(body["receipt"]["submission_digest"], digest().to_hex());
        assert_eq!(
            body["status"],
            if action == "retire" {
                "retired"
            } else {
                "published"
            }
        );
        if action != "publish" {
            assert_eq!(body["reason"], "Explicit reason\nMore");
        }
    }
}
#[tokio::test]
async fn staff_global_listing_exact_detail_and_light_history_have_no_case_fields() {
    for token in ["owner", "litigator", "paralegal"] {
        for suffix in [
            String::new(),
            format!("/{ID}"),
            format!("/{ID}/revisions/1"),
            format!("/{ID}/history"),
        ] {
            let w = Arc::new(Workflow::default());
            let (status, body) = request(
                w.clone(),
                "GET",
                &format!("{BASE}{suffix}"),
                Some(token),
                None,
                &[],
            )
            .await;
            assert_eq!(status, 200, "{body}");
            assert!(!body.to_string().contains("case_id"));
            if suffix.is_empty() {
                assert_eq!(body["calendars"][0]["scope"]["jurisdiction"], "local");
                assert!(body["calendars"][0].get("values").is_none());
                assert_eq!(w.calls.lock().unwrap()[0][1], 20);
                assert_eq!(w.calls.lock().unwrap()[0][3], "published");
            } else if suffix.ends_with("history") {
                assert_eq!(body["revisions"][0]["revision"], 1);
                assert!(body["revisions"][0].get("values").is_none());
                assert_eq!(w.calls.lock().unwrap()[0][2], 10);
            } else {
                assert_eq!(
                    body["values"],
                    vector("leap_unicode_unordered")["normalized"]
                );
            }
        }
    }
}
#[tokio::test]
async fn exact_days_preserve_outside_unresolved_exceptions_and_year_9999() {
    let path = format!("{BASE}/{ID}/revisions/1/days?from=2000-02-26&through=2000-03-05");
    let (status, body) = request(
        Arc::new(Workflow::default()),
        "GET",
        &path,
        Some("owner"),
        None,
        &[],
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["calendar_id"], ID);
    assert_eq!(body["revision"], 1);
    assert_eq!(body["days"][0]["state"], "outside_coverage");
    assert_eq!(body["days"][0]["origin"], json!(null));
    assert_eq!(body["days"][0]["source_ids"], json!([]));
    assert_eq!(body["days"][3]["origin"], "exception");
    assert_eq!(body["days"][3]["exception_id"], ID);
    assert_eq!(body["days"][3]["state"], "excluded");
    assert_eq!(body["days"][5]["state"], "unresolved");
    let path = format!("{BASE}/{ID}/revisions/1/days?from=9999-12-31&through=9999-12-31");
    let (status, body) = request(
        Arc::new(Workflow::default()),
        "GET",
        &path,
        Some("owner"),
        None,
        &[],
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["days"].as_array().unwrap().len(), 1);
    assert_eq!(body["days"][0]["date"], "9999-12-31");
}
#[tokio::test]
async fn errors_are_specific_and_storage_details_never_escape() {
    for (token, status, code) in [
        ("client", 403, "permission_denied"),
        ("expired", 401, "invalid_session"),
        ("missing", 404, "judicial_calendar_not_found"),
        ("revision", 409, "judicial_calendar_revision_conflict"),
        ("operation", 409, "judicial_calendar_operation_conflict"),
        ("retired", 409, "judicial_calendar_retired"),
        ("exhausted", 409, "judicial_calendar_revision_exhausted"),
        ("scope", 422, "judicial_calendar_scope_change_forbidden"),
        ("mismatch", 409, "judicial_calendar_submission_mismatch"),
        ("internal", 500, "internal_error"),
    ] {
        let (s, b) = request(
            Arc::new(Workflow::default()),
            "GET",
            &format!("{BASE}/{ID}"),
            Some(token),
            None,
            &[],
        )
        .await;
        assert_eq!(s, status, "{token}: {b}");
        assert_eq!(b["error"]["code"], code);
        assert!(!b.to_string().contains("secret SQL"));
    }
}
#[tokio::test]
async fn wrong_target_action_revision_or_receipt_is_never_projected() {
    let (_, _, payload) = mutation("publish");
    for (method, path) in [
        ("PUT", format!("{BASE}/{ID}")),
        ("POST", format!("{BASE}/{ID}/retirement")),
    ] {
        let w = Arc::new(Workflow::default());
        let (s, b) = request(
            w.clone(),
            method,
            &path,
            Some("owner"),
            Some(payload.to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(s, 400, "{b}");
        assert!(w.calls.lock().unwrap().is_empty());
    }
    for mode in 0..4 {
        let w = Arc::new(Workflow::default());
        let mut row = detail(&command());
        match mode {
            0 => row.id = application::judicial_calendars::JudicialCalendarId::new(),
            1 => {
                row.revision =
                    application::judicial_calendars::JudicialCalendarRevision::new(2).unwrap()
            }
            2 => {
                row.receipt.operation_id =
                    application::judicial_calendars::JudicialCalendarOperationId::new()
            }
            _ => row.receipt.submission_digest = domain::crypto::Sha256Digest::from_array([1; 32]),
        }
        *w.response.lock().unwrap() = Some(row);
        let (s, b) = request(
            w,
            "POST",
            BASE,
            Some("owner"),
            Some(payload.to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(s, 500, "{b}");
    }
}
