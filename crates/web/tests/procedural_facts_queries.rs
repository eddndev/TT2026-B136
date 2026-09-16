mod procedural_fact_http_support;
use procedural_fact_http_support::*;
use serde_json::{json, Value};
use std::sync::Arc;

#[tokio::test]
async fn list_filters_and_exclusive_nil_cursors_reach_the_matching_family() {
    for family in ["resolution", "notification"] {
        for (filter, expected) in [
            ("all", json!(null)),
            ("recorded", json!("recorded")),
            ("withdrawn", json!("withdrawn")),
        ] {
            let workflow = Arc::new(Workflow::default());
            let path = format!(
                "{}?limit=100&after_id={NIL}&status={filter}",
                base_url(family)
            );
            let (status, body) =
                request(workflow.clone(), "GET", &path, Some("valid"), None, &[]).await;
            assert_eq!(status, 200, "{body}");
            let calls = workflow.calls.lock().unwrap();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0]["limit"], 100);
            assert_eq!(calls[0]["after_id"], NIL);
            assert_eq!(calls[0]["status"], expected);
            assert_eq!(calls[0]["case_id"], CASE);
            if family == "notification" {
                assert_eq!(calls[0]["resolution_id"], PARENT);
            }
        }
    }
}

#[tokio::test]
async fn history_preserves_maximum_revision_and_its_own_page_limit() {
    for family in ["resolution", "notification"] {
        let workflow = Arc::new(Workflow::default());
        let path = format!(
            "{}/{ID}/history?limit=20&before_revision=4294967295",
            base_url(family)
        );
        let (status, body) =
            request(workflow.clone(), "GET", &path, Some("valid"), None, &[]).await;
        assert_eq!(status, 200, "{body}");
        let calls = workflow.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["limit"], 20);
        assert_eq!(calls[0]["before_revision"], u32::MAX);
        assert_eq!(calls[0]["target"]["family"], family);
    }
}

#[tokio::test]
async fn invalid_unknown_and_duplicate_list_and_history_queries_are_rejected() {
    for family in ["resolution", "notification"] {
        for suffix in [
            "?limit=0",
            "?limit=101",
            "?limit=-1",
            "?limit=4294967296",
            "?limit=1&limit=2",
            "?after_id=no",
            "?after_id=",
            "?status=all&status=recorded",
            "?status=active",
            "?status=",
            "?unknown=1",
            "?%6cimit=1&limit=2",
            "?limit=%ZZ",
            "?limit",
        ] {
            reject("GET", &format!("{}{suffix}", base_url(family)), None, 400).await;
        }
        for query in [
            "limit=21",
            "before_revision=0",
            "before_revision=4294967296",
            "before_revision=-1",
            "before_revision=2&before_revision=3",
            "after_id=x",
            "status=recorded",
            "unknown=x",
        ] {
            reject(
                "GET",
                &format!("{}/{ID}/history?{query}", base_url(family)),
                None,
                400,
            )
            .await;
        }
    }
}

#[tokio::test]
async fn detail_exact_revision_and_mutations_never_ignore_extra_queries() {
    for family in ["resolution", "notification"] {
        let base = base_url(family);
        for query in [
            "revision=1",
            "revision=1&revision=2",
            "unknown=1",
            "invalid=%ZZ",
        ] {
            for suffix in [format!("/{ID}"), format!("/{ID}/revisions/1")] {
                reject("GET", &format!("{base}{suffix}?{query}"), None, 400).await;
            }
            for (method, suffix, body) in [
                (
                    "POST",
                    "/prepare".to_owned(),
                    command_json(family, "record"),
                ),
                (
                    "POST",
                    String::new(),
                    submission(command_json(family, "record")),
                ),
                (
                    "PUT",
                    format!("/{ID}"),
                    submission(command_json(family, "correct")),
                ),
                (
                    "POST",
                    format!("/{ID}/withdrawal"),
                    submission(command_json(family, "withdraw")),
                ),
            ] {
                reject(method, &format!("{base}{suffix}?{query}"), Some(body), 400).await;
            }
        }
    }
}

#[tokio::test]
async fn explicitly_empty_query_is_allowed_on_reads_and_mutations() {
    for family in ["resolution", "notification"] {
        let base = base_url(family);
        for (method, path, body, status) in [
            ("GET", format!("{base}/{ID}?"), None, 200),
            ("GET", format!("{base}/{ID}/revisions/1?"), None, 200),
            (
                "POST",
                format!("{base}/prepare?"),
                Some(command_json(family, "record")),
                200,
            ),
            (
                "POST",
                format!("{base}?"),
                Some(submission(command_json(family, "record"))),
                201,
            ),
        ] {
            let workflow = Arc::new(Workflow::default());
            let (actual, response) = request(
                workflow.clone(),
                method,
                &path,
                Some("valid"),
                body.as_ref().map(Value::to_string),
                if body.is_some() {
                    &["application/json"]
                } else {
                    &[]
                },
            )
            .await;
            assert_eq!(actual, status, "{response}");
            assert_eq!(workflow.calls.lock().unwrap().len(), 1);
        }
    }
}

#[tokio::test]
async fn invalid_case_parent_target_and_revision_are_rejected_before_application() {
    for family in ["resolution", "notification"] {
        let base = base_url(family);
        for path in [
            base.replace(CASE, "bad-case"),
            format!("{base}/bad-id"),
            format!("{base}/{ID}/revisions/0"),
            format!("{base}/{ID}/revisions/4294967296"),
            format!("{base}/{ID}/revisions/-1"),
        ] {
            reject("GET", &path, None, 400).await;
        }
        if family == "notification" {
            reject("GET", &base.replace(PARENT, "bad-parent"), None, 400).await;
        }
    }
}

async fn reject(method: &str, path: &str, body: Option<Value>, expected: u16) {
    let workflow = Arc::new(Workflow::default());
    let (status, response) = request(
        workflow.clone(),
        method,
        path,
        Some("valid"),
        body.as_ref().map(Value::to_string),
        if body.is_some() {
            &["application/json"]
        } else {
            &[]
        },
    )
    .await;
    assert_eq!(status, expected, "{method} {path}: {response}");
    assert!(workflow.calls.lock().unwrap().is_empty());
}
