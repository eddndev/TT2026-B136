#[allow(dead_code)]
mod judicial_calendar_support;
use judicial_calendar_support::*;
use std::sync::Arc;
#[tokio::test]
async fn queries_validate_known_unique_fields_and_bounded_inclusive_days() {
    let mut paths = vec![];
    for q in [
        "limit=0",
        "limit=101",
        "limit=2&limit=3",
        "limit=+1",
        "status=active",
        "jurisdiction=national",
        "entity_code=1",
        "entity_code=33",
        "entity_code=01&entity_code=02",
        "after_id=oops",
        "unknown=1",
    ] {
        paths.push(format!("{BASE}?{q}"));
    }
    for q in [
        "limit=21",
        "before_revision=0",
        "before_revision=1&before_revision=2",
        "unknown=1",
    ] {
        paths.push(format!("{BASE}/{ID}/history?{q}"));
    }
    for q in [
        "",
        "from=2000-01-01",
        "from=2000-01-01&through=2000-03-03",
        "from=2000-02-30&through=2000-03-01",
        "from=0000-01-01&through=0001-01-01",
        "from=2000-01-02&through=2000-01-01",
        "from=2000-01-01&through=2000-01-01&from=2001-01-01",
        "from=2000-01-01&through=2000-01-01&unknown=1",
    ] {
        paths.push(format!("{BASE}/{ID}/revisions/1/days?{q}"));
    }
    for path in paths {
        let w = Arc::new(Workflow::default());
        let (s, b) = request(w.clone(), "GET", &path, Some("owner"), None, &[]).await;
        assert_eq!(s, 400, "{path}: {b}");
        assert!(w.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn explicit_filters_cursors_and_maximum_limits_reach_workflow() {
    let w = Arc::new(Workflow::default());
    let (s, b) = request(
        w.clone(),
        "GET",
        &format!("{BASE}?limit=100&status=all&jurisdiction=local&entity_code=01&after_id={ID}"),
        Some("paralegal"),
        None,
        &[],
    )
    .await;
    assert_eq!(s, 200, "{b}");
    {
        let calls = w.calls.lock().unwrap();
        assert_eq!(
            calls[0],
            serde_json::json!(["list", 100, ID, null, "local", "01"])
        );
    }
    let (s, b) = request(
        w.clone(),
        "GET",
        &format!("{BASE}/{ID}/history?limit=20&before_revision=4294967295"),
        Some("owner"),
        None,
        &[],
    )
    .await;
    assert_eq!(s, 200, "{b}");
    assert_eq!(w.calls.lock().unwrap()[1][3], u32::MAX);
}
#[tokio::test]
async fn routes_without_options_reject_queries_and_accept_empty_query() {
    for suffix in [format!("/{ID}"), format!("/{ID}/revisions/1")] {
        for q in ["revision=1", "revision=1&revision=2", "unknown=1"] {
            let w = Arc::new(Workflow::default());
            let (s, b) = request(
                w.clone(),
                "GET",
                &format!("{BASE}{suffix}?{q}"),
                Some("owner"),
                None,
                &[],
            )
            .await;
            assert_eq!(s, 400, "{b}");
            assert!(w.calls.lock().unwrap().is_empty());
        }
    }
    for action in ["prepare", "publish", "replace", "retire"] {
        let (method, path, payload) = mutation(action);
        for q in ["unknown=1", "revision=1&revision=2"] {
            let w = Arc::new(Workflow::default());
            let (s, b) = request(
                w.clone(),
                method,
                &format!("{path}?{q}"),
                Some("owner"),
                Some(payload.to_string()),
                &["application/json"],
            )
            .await;
            assert_eq!(s, 400, "{b}");
            assert!(w.calls.lock().unwrap().is_empty());
        }
        let (s, b) = request(
            Arc::new(Workflow::default()),
            method,
            &format!("{path}?"),
            Some("owner"),
            Some(payload.to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(s, if action == "prepare" { 200 } else { 201 }, "{b}");
    }
}
#[tokio::test]
async fn missing_bearer_invalid_uuid_and_revision_never_call_workflow() {
    for (path, token, status) in [
        (BASE.to_string(), None, 401),
        (format!("{BASE}/not-a-uuid"), Some("owner"), 400),
        (format!("{BASE}/{ID}/revisions/0"), Some("owner"), 422),
        (format!("{BASE}/{ID}/revisions/+1"), Some("owner"), 400),
    ] {
        let w = Arc::new(Workflow::default());
        let (s, b) = request(w.clone(), "GET", &path, token, None, &[]).await;
        assert_eq!(s, status, "{b}");
        assert!(w.calls.lock().unwrap().is_empty());
    }
}
