mod deadline_http_support;
use deadline_http_support::*;
use std::sync::Arc;

#[tokio::test]
async fn bearer_prevalidation_precedes_invalid_json_and_private_routes() {
    for (method, path, body) in [
        ("GET", BASE.to_string(), None),
        ("POST", format!("{BASE}/prepare"), Some("{".into())),
    ] {
        let workflow = Arc::new(Workflow::default());
        let (status, value) = request(
            workflow.clone(),
            method,
            &path,
            None,
            body,
            &["application/json"],
        )
        .await;
        assert_eq!(status, 401, "{value}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn duplicate_unknown_and_invalid_queries_fail_without_calling_workflow() {
    for path in [
        format!("{BASE}?limit=1&limit=2"),
        format!("{BASE}?scope=all"),
        format!("{BASE}?limit=101"),
        format!("{BASE}?limit=+1"),
        format!("{BASE}?status=all&status=active"),
        format!("{BASE}/{ID}?limit=1"),
        format!("{BASE}/{ID}/revisions/1?x=1"),
        format!("{BASE}/{ID}/history?limit=21"),
        format!("{BASE}/{ID}/history?before_revision=0"),
    ] {
        let workflow = Arc::new(Workflow::default());
        let (status, body) =
            request(workflow.clone(), "GET", &path, Some("owner"), None, &[]).await;
        assert_eq!(status, 400, "{path}: {body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
    let workflow = Arc::new(Workflow::default());
    let (status, body) = request(
        workflow.clone(),
        "POST",
        &format!("{BASE}/prepare?x=1"),
        Some("owner"),
        Some(command().to_string()),
        &["application/json"],
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(workflow.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn path_identifiers_and_revision_zero_are_not_coerced() {
    for path in [
        "/api/v1/cases/not-a-uuid/deadlines".to_string(),
        format!("{BASE}/invalid"),
        format!("{BASE}/{ID}/revisions/0"),
        format!("{BASE}/{ID}/revisions/4294967296"),
    ] {
        let workflow = Arc::new(Workflow::default());
        let (status, body) =
            request(workflow.clone(), "GET", &path, Some("owner"), None, &[]).await;
        assert_eq!(status, 400, "{path}: {body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn json_content_type_and_one_mib_limit_have_specific_errors() {
    for types in [
        vec![],
        vec!["text/plain"],
        vec!["application/json", "application/json"],
    ] {
        let workflow = Arc::new(Workflow::default());
        let (status, body) = request(
            workflow.clone(),
            "POST",
            &format!("{BASE}/prepare"),
            Some("owner"),
            Some(command().to_string()),
            &types,
        )
        .await;
        assert_eq!(status, 400, "{body}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
    let workflow = Arc::new(Workflow::default());
    let (status, body) = request(
        workflow.clone(),
        "POST",
        &format!("{BASE}/prepare"),
        Some("owner"),
        Some(" ".repeat(1024 * 1024 + 1)),
        &["application/json"],
    )
    .await;
    assert_eq!(status, 413, "{body}");
    assert_eq!(body["error"]["code"], "deadline_body_too_large");
    assert!(workflow.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn authorization_and_revision_failures_preserve_their_status() {
    for (token, failure, status) in [
        ("client", None, 403),
        ("owner", Some("case"), 404),
        ("owner", Some("session"), 401),
    ] {
        let workflow = Arc::new(Workflow::default());
        *workflow.failure.lock().unwrap() = failure;
        let (actual, body) = request(workflow, "GET", BASE, Some(token), None, &[]).await;
        assert_eq!(actual, status, "{body}");
    }
    for (token, failure, status) in [
        ("paralegal", None, 403),
        ("owner", Some("closed"), 409),
        ("owner", Some("conflict"), 409),
        ("owner", Some("mismatch"), 409),
        ("owner", Some("retired"), 409),
        ("owner", Some("responsible"), 409),
        ("owner", Some("profile"), 409),
    ] {
        let workflow = Arc::new(Workflow::default());
        *workflow.failure.lock().unwrap() = failure;
        let (actual, body) = request(
            workflow,
            "POST",
            &format!("{BASE}/prepare"),
            Some(token),
            Some(command().to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(actual, status, "{body}");
    }
}
