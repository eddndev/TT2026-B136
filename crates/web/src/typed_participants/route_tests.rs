use super::{proposal_tests::preparation, route_support::*};
use axum::{body::Body, http::StatusCode};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn reviewed_commit_returns_exact_snapshot_and_unsigned_reconciliation_without_gets() {
    let w = Arc::new(Workflow::default());
    let p = preparation();
    let review = json!({"subject":{"operation":"keep","reference":p["proposal"]["values"]["subject"]},
        "participant":{"operation":"create"},"role":p["proposal"]["values"]["role"],"certificate_base64":"AQIDBA=="});
    let r = request(
        &w,
        "POST",
        "participants/proposals/review",
        Some("owner"),
        review.to_string(),
    )
    .await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(body(r).await["case_id"], CASE);
    let r = request(
        &w,
        "POST",
        "participants/proposals/prepare",
        Some("owner"),
        p.to_string(),
    )
    .await;
    assert_eq!(r.status(), StatusCode::OK);
    let draft = body(r).await;
    assert_eq!(draft["submission_digest"], "66".repeat(32));
    assert_eq!(draft["submission_revision"], 1);
    let r = request(
        &w,
        "POST",
        "participants/proposals/commit",
        Some("owner"),
        json!({"prepared":p,"signature_base64":null}).to_string(),
    )
    .await;
    assert_eq!(r.status(), StatusCode::CREATED);
    assert_eq!(r.headers()["cache-control"], "no-store");
    let saved = body(r).await;
    assert_eq!(saved["display_name"], "Captured Ana");
    assert_eq!(saved["subject"]["revision"], 1);
    assert_eq!(saved["canonical_format"], "part2");
    assert_eq!(saved["profile"]["kind"], "control_judge");
    assert_eq!(saved["submission_digest"], draft["submission_digest"]);
    assert_eq!(
        *w.calls.lock().unwrap(),
        vec![
            json!(["review", CASE, "control_judge", [1, 2, 3, 4]]),
            json!(["prepare", CASE, PARTICIPANT, [1, 2, 3, 4]]),
            json!(["commit", CASE, PARTICIPANT, null])
        ]
    );
}

#[tokio::test]
async fn subjects_preserve_scope_cursors_and_private_details_are_explicit() {
    let w = Arc::new(Workflow::default());
    let r = request(
        &w,
        "GET",
        "subjects?limit=2&kind=natural_person&name=Ana%25_",
        Some("reader"),
        Body::empty(),
    )
    .await;
    assert_eq!(r.status(), StatusCode::OK);
    let page = body(r).await;
    assert_eq!(page["next_after_id"], SUBJECT);
    assert!(page["subjects"][0].get("values").is_none());
    for path in [
        format!("subjects/{SUBJECT}"),
        format!("subjects/{SUBJECT}/revisions/1"),
        format!("subjects/{SUBJECT}/history?limit=1&before_revision=2"),
    ] {
        assert_eq!(
            request(&w, "GET", &path, Some("reader"), Body::empty())
                .await
                .status(),
            StatusCode::OK
        );
    }
    let input = json!({"expected_revision":1,"values":subject_values()});
    assert_eq!(
        request(
            &w,
            "POST",
            &format!("subjects/{SUBJECT}/review"),
            Some("owner"),
            input.to_string()
        )
        .await
        .status(),
        StatusCode::OK
    );
    let input =
        json!({"expected_revision":1,"values":subject_values(),"review":preparation()["review"]});
    assert_eq!(
        request(
            &w,
            "PUT",
            &format!("subjects/{SUBJECT}"),
            Some("owner"),
            input.to_string()
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        w.calls.lock().unwrap()[0],
        json!(["subjects", CASE, 2, null, "Ana%_", "natural_person"])
    );
    assert_eq!(w.calls.lock().unwrap().len(), 6);
}

#[tokio::test]
async fn unauthenticated_or_invalid_input_never_enters_workflow() {
    let w = Arc::new(Workflow::default());
    for (method, path, body) in [
        ("POST", "participants/proposals/review".into(), "{}".into()),
        (
            "POST",
            "participants/proposals/prepare".into(),
            preparation().to_string(),
        ),
        ("POST", "participants/proposals/commit".into(), "{}".into()),
        ("GET", "subjects".into(), String::new()),
        ("GET", format!("subjects/{SUBJECT}"), String::new()),
        (
            "GET",
            format!("participants/{PARTICIPANT}/revisions/1/credential"),
            String::new(),
        ),
    ] {
        assert_eq!(
            request(&w, method, &path, None, body).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    for path in [
        "subjects?limit=0",
        "subjects?limit=1&limit=2",
        "subjects?curp=secret",
        "subjects?kind=company",
        "subjects/bad",
        "subjects/22222222-2222-4222-8222-222222222222/revisions/0",
    ] {
        let r = request(&w, "GET", path, Some("reader"), Body::empty()).await;
        assert!(r.status().is_client_error(), "{path}");
    }
    assert!(w.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn body_limit_unknown_fields_and_trailing_content_reject_without_mutation() {
    let w = Arc::new(Workflow::default());
    for body in [
        format!("{}{{}}", preparation()),
        " ".repeat(256 * 1024 + 1),
        preparation()
            .to_string()
            .replace("\"proposal\":", "\"unknown\":1,\"proposal\":"),
    ] {
        let r = request(
            &w,
            "POST",
            "participants/proposals/prepare",
            Some("owner"),
            body,
        )
        .await;
        assert!(r.status().is_client_error());
    }
    assert!(w.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn credential_conflicts_and_internal_failures_have_static_error_codes() {
    let w = Arc::new(Workflow::default());
    for (token, status, code) in [
        ("forbidden", 403, "permission_denied"),
        (
            "review-changed",
            409,
            "participant_identity_review_conflict",
        ),
        ("revoked", 422, "participant_credential_revoked"),
        ("internal", 500, "internal_error"),
    ] {
        let r = request(
            &w,
            "POST",
            "participants/proposals/prepare",
            Some(token),
            preparation().to_string(),
        )
        .await;
        assert_eq!(r.status().as_u16(), status);
        let value = body(r).await;
        assert_eq!(value["error"]["code"], code);
        assert!(!value.to_string().contains("secret"));
    }
    let r = request(
        &w,
        "GET",
        &format!("participants/{PARTICIPANT}/revisions/1/credential"),
        Some("reader"),
        Body::empty(),
    )
    .await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body(r).await["error"]["code"],
        "participant_credential_not_found"
    );
}
