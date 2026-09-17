mod deadline_profile_http_support;
use application::deadline_profiles::*;
use deadline_profile_http_support::*;
use serde_json::{json, Value};
use std::sync::Arc;

#[tokio::test]
async fn queries_and_exact_paths_reject_ambiguous_selection_before_workflow() {
    let mut paths = vec![];
    for base in [BASE.to_owned(), private_base()] {
        for q in [
            "limit=0",
            "limit=101",
            "limit=1&limit=2",
            "scope=global",
            "status=active",
            "after_id=nope",
        ] {
            paths.push(format!("{base}?{q}"));
        }
        for q in [
            "limit=21",
            "before_revision=0",
            "before_revision=1&before_revision=2",
            "scope=global",
        ] {
            paths.push(format!("{base}/{ID}/history?{q}"));
        }
        for suffix in [
            format!("/{ID}?x=1"),
            format!("/{ID}/revisions/1?limit=1"),
            format!("/{ID}/revisions/0"),
            "/bad-id".into(),
        ] {
            paths.push(format!("{base}{suffix}"));
        }
    }
    paths.push(format!("/api/v1/cases/bad/deadline-profiles/{ID}"));
    for path in paths {
        let w = Arc::new(Workflow::default());
        let (s, b) = request(w.clone(), "GET", &path, Some("owner"), None, &[]).await;
        assert_eq!(s, 400, "{path}: {b}");
        assert!(w.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn bearer_prevalidation_and_workflow_authorization_apply_to_both_collections() {
    for base in [BASE.to_owned(), private_base()] {
        let w = Arc::new(Workflow::default());
        let (s, _) = request(
            w.clone(),
            "POST",
            &format!("{base}/prepare?unknown=1"),
            None,
            Some("broken JSON".into()),
            &[],
        )
        .await;
        assert_eq!(s, 401);
        assert!(w.calls.lock().unwrap().is_empty());
        for token in ["owner", "litigator", "paralegal", "client"] {
            let (s, b) = request(
                Arc::new(Workflow::default()),
                "GET",
                &base,
                Some(token),
                None,
                &[],
            )
            .await;
            assert_eq!(s, if token == "client" { 403 } else { 200 }, "{b}");
            let c = command_json(if base == BASE { None } else { Some(CASE) });
            let (s, b) = request(
                Arc::new(Workflow::default()),
                "POST",
                &format!("{base}/prepare"),
                Some(token),
                Some(c.to_string()),
                &["application/json"],
            )
            .await;
            assert_eq!(s, if token == "owner" { 200 } else { 403 }, "{b}");
        }
    }
}
#[tokio::test]
async fn mutation_scope_route_action_and_identity_cannot_be_overridden() {
    let mut variants = vec![];
    let mut c = submission("replace", None);
    c["command"]["profile_id"] = json!(CASE);
    variants.push((format!("{BASE}/{ID}"), c));
    variants.push((format!("{BASE}/{ID}"), submission("publish", None)));
    variants.push((
        format!("{BASE}/{ID}?scope=global"),
        submission("replace", None),
    ));
    variants.push((
        format!("{}/{ID}", private_base()),
        submission("replace", None),
    ));
    variants.push((format!("{BASE}/{ID}"), submission("replace", Some(CASE))));
    for (path, c) in variants {
        let w = Arc::new(Workflow::default());
        let (s, b) = request(
            w.clone(),
            "PUT",
            &path,
            Some("owner"),
            Some(c.to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(s, 400, "{path}: {b}");
        assert!(w.calls.lock().unwrap().is_empty());
    }
    let mut c = submission("retire", None);
    c["command"]["change"]["definition"] = definition_json(None);
    let (s, _) = request(
        Arc::new(Workflow::default()),
        "POST",
        &format!("{BASE}/{ID}/retirement"),
        Some("owner"),
        Some(c.to_string()),
        &["application/json"],
    )
    .await;
    assert_eq!(s, 400);
}
#[tokio::test]
async fn application_errors_preserve_conflict_absence_and_closed_semantics() {
    for (failure, status, code) in [
        ("case", 404, "case_not_found"),
        ("closed", 409, "case_closed"),
        ("session", 401, "invalid_session"),
        ("conflict", 409, "deadline_profile_revision_conflict"),
        ("mismatch", 409, "deadline_profile_submission_mismatch"),
        ("retired", 409, "deadline_profile_retired"),
        ("not_found", 404, "deadline_profile_not_found"),
    ] {
        let w = Arc::new(Workflow::default());
        *w.failure.lock().unwrap() = Some(failure);
        let (s, b) = prepare_json(w.clone(), command_json(None)).await;
        assert_eq!(s, status, "{b}");
        assert_eq!(b["error"]["code"], code);
        assert_eq!(w.calls.lock().unwrap().len(), 1);
    }
}
#[tokio::test]
async fn wrong_response_scope_revision_or_receipt_is_not_projected_as_success() {
    for mutation in ["scope", "id", "revision", "receipt"] {
        let w = Arc::new(Workflow::default());
        let mut row = published(DeadlineProfileCollection::Global);
        match mutation {
            "scope" => row = published(DeadlineProfileCollection::ForCase(case())),
            "id" => row.id = DeadlineProfileId::new(),
            "revision" => row.revision = DeadlineProfileRevision::new(2).unwrap(),
            _ => row.receipt.operation_id = DeadlineProfileOperationId::new(),
        }
        *w.response.lock().unwrap() = Some(row);
        let (s, b) = request(
            w,
            "POST",
            BASE,
            Some("owner"),
            Some(submission("publish", None).to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(s, 500, "{mutation}: {b}");
        assert!(b.get("definition").is_none());
    }
    let w = Arc::new(Workflow::default());
    *w.response.lock().unwrap() = Some(published(DeadlineProfileCollection::ForCase(case())));
    let (s, b) = request(w, "GET", BASE, Some("owner"), None, &[]).await;
    assert_eq!(s, 500);
    assert_eq!(b["profiles"], Value::Null);
}
