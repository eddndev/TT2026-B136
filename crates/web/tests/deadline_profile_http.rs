mod deadline_profile_http_support;
use application::deadline_profiles::*;
use deadline_profile_http_support::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn collection_routes_preserve_scope_filters_and_lightweight_pages() {
    for (base, c) in [
        (BASE.to_owned(), DeadlineProfileCollection::Global),
        (private_base(), DeadlineProfileCollection::ForCase(case())),
    ] {
        let w = Arc::new(Workflow::default());
        let (status, body) = request(
            w.clone(),
            "GET",
            &format!("{base}?limit=1&status=all"),
            Some("paralegal"),
            None,
            &[],
        )
        .await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["profiles"][0]["id"], ID);
        assert_eq!(body["profiles"][0]["algorithm"], "v1");
        assert!(body["profiles"][0].get("definition").is_none());
        assert_eq!(body["next_after_id"], serde_json::Value::Null);
        assert_eq!(
            w.calls.lock().unwrap()[0],
            json!(["paralegal", collection_json(c), ["list", 1, null, null]])
        );
    }
}
#[tokio::test]
async fn prepare_returns_editable_normalized_definition_and_v1_receipt_context() {
    for (base, case_id) in [(BASE.to_owned(), None), (private_base(), Some(CASE))] {
        let w = Arc::new(Workflow::default());
        let value = command_json(case_id);
        let (status, body) = request(
            w.clone(),
            "POST",
            &format!("{base}/prepare"),
            Some("owner"),
            Some(value.to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["command"], value);
        assert_eq!(body["definition"], value["change"]["definition"]);
        assert_eq!(body["algorithm"], "v1");
        assert_eq!(body["result_revision"], 1);
        assert_eq!(body["definition_digest"], digest().to_hex());
        assert_eq!(body["submission_digest"], digest().to_hex());
        let command = w.command.lock().unwrap();
        let DeadlineProfileChange::Publish { definition } = &command.as_ref().unwrap().change
        else {
            panic!("expected publish")
        };
        assert_eq!(definition.examples()[0].anchor.offset(), None);
        assert_eq!(definition.examples()[0].anchor.local_hour(), None);
    }
}
#[tokio::test]
async fn publish_replace_retire_and_exact_history_have_distinct_routes() {
    for (action, method, suffix) in [
        ("publish", "POST", String::new()),
        ("replace", "PUT", format!("/{ID}")),
        ("retire", "POST", format!("/{ID}/retirement")),
    ] {
        let w = Arc::new(Workflow::default());
        let (status, body) = request(
            w,
            method,
            &format!("{BASE}{suffix}"),
            Some("owner"),
            Some(submission(action, None).to_string()),
            &["application/json"],
        )
        .await;
        assert_eq!(status, 201, "{body}");
        assert_eq!(body["receipt"]["action"], action);
        assert_eq!(body["id"], ID);
        assert_eq!(body["revision"], if action == "publish" { 1 } else { 2 });
        if action != "publish" {
            assert_eq!(body["reason"], "Explicit reason\nMore");
        }
    }
    for path in [
        format!("{BASE}/{ID}"),
        format!("{BASE}/{ID}/revisions/1"),
        format!("{BASE}/{ID}/history?limit=10"),
    ] {
        let w = Arc::new(Workflow::default());
        let (status, body) = request(w, "GET", &path, Some("litigator"), None, &[]).await;
        assert_eq!(status, 200, "{body}");
        if path.contains("history") {
            assert!(body["revisions"][0].get("definition").is_none());
        } else {
            assert_eq!(body["definition"], definition_json(None));
        }
    }
}
