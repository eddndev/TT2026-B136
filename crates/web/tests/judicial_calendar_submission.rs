#[allow(dead_code)]
mod judicial_calendar_support;
use judicial_calendar_support::*;
use serde_json::json;
use std::sync::Arc;
#[tokio::test]
async fn submit_envelopes_obey_byte_limit_and_named_unique_fields() {
    for action in ["publish", "replace", "retire"] {
        let (method, path, payload) = mutation(action);
        let text = payload.to_string();
        let exact = format!("{}{}", text, " ".repeat(1024 * 1024 - text.len()));
        let (s, b) = request(
            Arc::new(Workflow::default()),
            method,
            &path,
            Some("owner"),
            Some(exact.clone()),
            &["application/json"],
        )
        .await;
        assert_eq!(s, 201, "{b}");
        let w = Arc::new(Workflow::default());
        let (s, b) = request(
            w.clone(),
            method,
            &path,
            Some("owner"),
            Some(format!("{exact} ")),
            &["application/json"],
        )
        .await;
        assert_eq!(s, 413, "{b}");
        assert!(w.calls.lock().unwrap().is_empty());
        for bad in [
            json!([payload["command"], payload["expected_submission_digest"]]).to_string(),
            text.replacen("\"command\":", "\"unknown\":true,\"command\":", 1),
            text.replacen(
                "\"expected_submission_digest\":",
                "\"expected_submission_digest\":null,\"expected_submission_digest\":",
                1,
            ),
        ] {
            let w = Arc::new(Workflow::default());
            let (s, b) = request(
                w.clone(),
                method,
                &path,
                Some("owner"),
                Some(bad),
                &["application/json"],
            )
            .await;
            assert_eq!(s, 400, "{b}");
            assert!(w.calls.lock().unwrap().is_empty());
        }
    }
}
#[tokio::test]
async fn wrong_calendar_body_variant_or_digest_never_reaches_workflow() {
    for action in ["replace", "retire"] {
        let (method, path, payload) = mutation(action);
        let mut cases = vec![];
        let mut v = payload.clone();
        v["command"]["calendar_id"] = json!(uuid::Uuid::from_u128(1));
        cases.push(v);
        let mut v = payload.clone();
        v["expected_submission_digest"] = json!("ff");
        cases.push(v);
        let mut v = payload.clone();
        v["command"]["change"]["extra"] = json!(true);
        cases.push(v);
        if action == "retire" {
            let mut v = payload.clone();
            v["command"]["change"]["values"] = command_json()["change"]["values"].clone();
            cases.push(v);
        }
        for v in cases {
            let w = Arc::new(Workflow::default());
            let (s, b) = request(
                w.clone(),
                method,
                &path,
                Some("owner"),
                Some(v.to_string()),
                &["application/json"],
            )
            .await;
            assert_eq!(s, 400, "{b}");
            assert!(w.calls.lock().unwrap().is_empty());
        }
    }
}
#[tokio::test]
async fn all_routes_require_bearer_and_propagate_current_permission_denial() {
    let mut routes = vec![
        ("GET", BASE.to_string(), None),
        ("GET", format!("{BASE}/{ID}"), None),
        ("GET", format!("{BASE}/{ID}/history"), None),
        ("GET", format!("{BASE}/{ID}/revisions/1"), None),
        (
            "GET",
            format!("{BASE}/{ID}/revisions/1/days?from=1970-01-01&through=1970-01-01"),
            None,
        ),
    ];
    routes.extend(["prepare", "publish", "replace", "retire"].map(|action| {
        let (m, p, v) = mutation(action);
        (m, p, Some(v.to_string()))
    }));
    for (method, path, body) in routes {
        let types = if body.is_some() {
            vec!["application/json"]
        } else {
            vec![]
        };
        for token in [None, Some("client"), Some("expired")] {
            let w = Arc::new(Workflow::default());
            let (s, b) = request(w.clone(), method, &path, token, body.clone(), &types).await;
            assert_eq!(
                s,
                if token == Some("client") { 403 } else { 401 },
                "{method} {path}: {b}"
            );
            assert_eq!(w.calls.lock().unwrap().len(), usize::from(token.is_some()));
        }
    }
}
