mod resource_activity_http_support;
use resource_activity_http_support::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn strict_commands_reject_ambiguous_fields_and_bind_both_parent_paths() {
    let valid = command_json("link", "hearing");
    let mut cases = vec![
        "[]".into(),
        "null".into(),
        format!("{valid} true"),
        valid.to_string().replace(
            "\"expected_revision\":0",
            "\"expected_revision\":0,\"expected_revision\":0",
        ),
    ];
    for key in ["case_id", "resource_id", "association_id", "operation_id"] {
        let mut value = valid.clone();
        value.as_object_mut().unwrap().remove(key);
        cases.push(value.to_string());
    }
    for (pointer, bad) in [
        ("/case_id", json!(FOREIGN)),
        ("/resource_id", json!(FOREIGN)),
        ("/change/resource/id", json!(FOREIGN)),
        ("/change/resource/revision", json!(0)),
        ("/change/target", json!([])),
        ("/change/target/submission_digest", json!("FF".repeat(32))),
        ("/change/expected_revision", json!(1)),
    ] {
        let mut value = valid.clone();
        *value.pointer_mut(pointer).unwrap() = bad;
        cases.push(value.to_string());
    }
    let mut absent = valid.clone();
    absent["change"].as_object_mut().unwrap().remove("act");
    cases.push(absent.to_string());
    let mut extra = valid.clone();
    extra["change"]["target"]["capture_digest"] = json!("01".repeat(32));
    cases.push(extra.to_string());
    let workflow = Arc::new(Workflow::default());
    for raw in cases {
        let (status, body) = request(
            workflow.clone(),
            "POST",
            &format!("{}/prepare", base()),
            "ok",
            Some(raw.clone()),
        )
        .await;
        assert_eq!(status, 400, "{raw}: {body}");
    }
    let unlink = submission(command_json("unlink", "hearing"));
    let (status, _) = request(
        workflow.clone(),
        "POST",
        &format!("{}/{FOREIGN}/unlink", base()),
        "ok",
        Some(unlink.to_string()),
    )
    .await;
    assert_eq!(status, 400);
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn queries_and_body_have_explicit_limits_before_workflow_calls() {
    let workflow = Arc::new(Workflow::default());
    for suffix in [
        "?limit=0",
        "?limit=101",
        "?limit=2&limit=3",
        "?unknown=true",
        "?kind=other",
        "?status=retired",
        "?after_id=bad",
    ] {
        let (status, body) = request(
            workflow.clone(),
            "GET",
            &format!("{}{suffix}", base()),
            "ok",
            None,
        )
        .await;
        assert_eq!(status, 400, "{suffix}: {body}");
    }
    for suffix in [
        format!("/{ID}?kind=hearing"),
        format!("/{ID}/revisions/01"),
        format!("/{ID}/history?limit=21"),
    ] {
        let (status, body) = request(
            workflow.clone(),
            "GET",
            &format!("{}{suffix}", base()),
            "ok",
            None,
        )
        .await;
        assert_eq!(status, 400, "{suffix}: {body}");
    }
    let (status, _) = request(
        workflow.clone(),
        "POST",
        &format!("{}/prepare", base()),
        "",
        Some("{".into()),
    )
    .await;
    assert_eq!(status, 401);
    let (status, _) = request(
        workflow.clone(),
        "POST",
        &format!("{}/prepare", base()),
        "ok",
        Some(" ".repeat(16 * 1024 + 1)),
    )
    .await;
    assert_eq!(status, 413);
    assert!(workflow.calls.lock().unwrap().is_empty());
}
