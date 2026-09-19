mod procedural_resource_http_support;
use procedural_resource_http_support::*;
use serde_json::{json, Value};
use std::sync::Arc;
#[tokio::test]
async fn rejects_unknown_duplicate_missing_null_and_positional_command_fields() {
    let valid = command_json("register");
    let mut cases = vec![
        "[]".into(),
        "null".into(),
        format!("{} true", valid),
        valid.to_string().replace(
            "\"expected_revision\":0",
            "\"expected_revision\":0,\"expected_revision\":0",
        ),
    ];
    for key in [
        "mode",
        "resolution_evidence",
        "receiving_authority",
        "notification_at",
        "appellants",
    ] {
        let mut v = valid.clone();
        v["change"]["values"].as_object_mut().unwrap().remove(key);
        cases.push(v.to_string());
    }
    for (pointer, bad) in [
        ("/change/values", json!([])),
        ("/change", Value::Null),
        (
            "/change/values/mode",
            json!({"kind":"known","value":"written","reason":"extra"}),
        ),
        ("/change/values/resolution", json!({"id":ID,"revision":0})),
        (
            "/change/values/resolution_evidence",
            json!([ID, 3, "2a".repeat(32), "locator"]),
        ),
    ] {
        let mut v = valid.clone();
        *v.pointer_mut(pointer).unwrap() = bad;
        cases.push(v.to_string());
    }
    let mut extra = valid.clone();
    extra["legal_effect"] = json!("admitted");
    cases.push(extra.to_string());
    for body in cases {
        let workflow = Arc::new(Workflow::default());
        let (status, error) = request(
            workflow.clone(),
            "POST",
            &format!("{}/prepare", base()),
            "ok",
            Some(body.clone()),
        )
        .await;
        assert!(matches!(status, 400 | 422), "{body}: {status} {error}");
        assert!(workflow.commands.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn route_identity_action_and_act_must_agree_before_workflow() {
    for (method, suffix, action) in [
        ("PUT", ID, "register"),
        ("POST", &format!("{ID}/archive"), "reactivate"),
        ("PUT", &format!("{ID}/acts/{OP}"), "correct_act"),
    ] {
        let workflow = Arc::new(Workflow::default());
        let (status, _) = request(
            workflow.clone(),
            method,
            &format!("{}/{suffix}", base()),
            "ok",
            Some(submission(command_json(action)).to_string()),
        )
        .await;
        assert_eq!(status, 400);
        assert!(workflow.commands.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn query_bounds_duplicates_and_unexpected_parameters_are_rejected() {
    for suffix in [
        "?limit=0",
        "?limit=101",
        "?limit=2&limit=3",
        "?kind=unknown",
        "?status=withdrawn",
        "?extra=1",
        &format!("/{ID}?limit=2"),
        &format!("/{ID}/history?limit=21"),
        &format!("/{ID}/revisions/01"),
    ] {
        let workflow = Arc::new(Workflow::default());
        let (status, error) = request(
            workflow.clone(),
            "GET",
            &format!("{}{suffix}", base()),
            "ok",
            None,
        )
        .await;
        assert_eq!(status, 400, "{suffix}: {error}");
        assert!(workflow.calls.lock().unwrap().is_empty());
    }
}
#[tokio::test]
async fn requires_bearer_and_bounded_complete_json_before_dispatch() {
    let workflow = Arc::new(Workflow::default());
    let (status, _) = request(
        workflow.clone(),
        "POST",
        &format!("{}/prepare", base()),
        "",
        Some(command_json("register").to_string()),
    )
    .await;
    assert_eq!(status, 401);
    let (status, _) = request(
        workflow.clone(),
        "POST",
        &format!("{}/prepare", base()),
        "ok",
        Some(" ".repeat(512 * 1024 + 1)),
    )
    .await;
    assert_eq!(status, 413);
    assert!(workflow.calls.lock().unwrap().is_empty());
}
