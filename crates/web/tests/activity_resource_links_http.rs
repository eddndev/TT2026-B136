mod resource_activity_http_support;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use resource_activity_http_support::*;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn target_path(kind: &str) -> String {
    let (collection, id) = if kind == "hearing" {
        ("hearings", HEARING)
    } else {
        ("deadlines", BEFORE)
    };
    format!("/api/v1/cases/{CASE}/{collection}/{id}/resource-associations")
}
async fn read(workflow: Arc<Workflow>, path: &str, token: &str) -> (u16, Value) {
    let mut request = Request::builder().uri(path);
    if !token.is_empty() {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = web::resource_activity_router(workflow)
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn both_activity_kinds_keep_exact_captures_separate_from_current_heads() {
    for (kind, id) in [("hearing", HEARING), ("deadline", BEFORE)] {
        let workflow = Arc::new(Workflow::default());
        let (status, body) = read(workflow.clone(), &target_path(kind), "owner").await;
        assert_eq!(status, 200, "{kind}: {body}");
        assert_eq!(body["case_id"], CASE);
        assert_eq!(body["target"], json!({"kind":kind,"id":id}));
        assert_eq!(
            body["checked_at"],
            json!({"unix_seconds":now().unix_timestamp(),
            "nanosecond":0,"offset_seconds":0})
        );
        assert_eq!(body["has_more"], false);
        assert!(body["next_after_id"].is_null());
        let row = &body["associations"][0];
        assert_eq!(row["checked_at"], body["checked_at"]);
        assert_eq!(row["association"]["resource_id"], RESOURCE);
        assert_eq!(row["association"]["selection"]["resource"]["revision"], 2);
        assert_eq!(row["association"]["recorded_resource_head"]["revision"], 5);
        assert_eq!(row["association"]["selection"]["target"]["revision"], 1);
        assert_eq!(
            row["association"]["sources"]["target"]["record"]["revision"],
            1
        );
        assert_eq!(row["current_target"]["kind"], kind);
        assert_eq!(row["current_target"]["record"]["revision"], 2);
        if kind == "deadline" {
            assert_eq!(row["current_target"]["record"]["status"], "retired");
            assert!(row["current_target"]["record"]["operational"]["due_at"].is_null());
        }
        assert_eq!(
            workflow.calls.lock().unwrap().as_slice(),
            &[json!({
                "method":"list_for_target","case_id":CASE,"target":{"kind":kind,"id":id},
                "limit":20,"after_id":null,"status":"linked"
            })]
        );
        assert!(workflow.commands.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn pagination_and_status_are_forwarded_without_replacing_captured_acts() {
    for (status_filter, expected) in [
        ("linked", json!("linked")),
        ("unlinked", json!("unlinked")),
        ("all", Value::Null),
    ] {
        let workflow = Arc::new(Workflow::default());
        let path = format!(
            "{}?limit=2&after_id={BEFORE}&status={status_filter}",
            target_path("hearing")
        );
        let (status, body) = read(workflow.clone(), &path, "pagination").await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["associations"].as_array().unwrap().len(), 2);
        assert_eq!(body["has_more"], true);
        assert_eq!(
            body["next_after_id"],
            "00000000-0000-0000-0000-000000000009"
        );
        let calls = workflow.calls.lock().unwrap();
        assert_eq!(calls[0]["status"], expected);
        assert_eq!(calls[0]["after_id"], BEFORE);
        assert_eq!(calls[0]["limit"], 2);
    }
    let (status, body) = read(
        Arc::new(Workflow::default()),
        &target_path("hearing"),
        "act",
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let row = &body["associations"][0]["association"];
    assert_eq!(row["selection"]["act"]["resource_revision"], 3);
    assert_eq!(row["sources"]["act"]["act"]["id"], ACT);
    assert_eq!(row["sources"]["act"]["act"]["supports"][0]["version"], 3);
}

#[tokio::test]
async fn empty_authorized_page_has_its_observation_and_no_invented_cursor() {
    for kind in ["hearing", "deadline"] {
        let (status, body) = read(Arc::new(Workflow::default()), &target_path(kind), "empty").await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["associations"], json!([]));
        assert_eq!(body["checked_at"]["unix_seconds"], now().unix_timestamp());
        assert_eq!(body["has_more"], false);
        assert!(body["next_after_id"].is_null());
    }
}

#[tokio::test]
async fn invalid_queries_and_noncanonical_paths_never_call_the_workflow() {
    let workflow = Arc::new(Workflow::default());
    for kind in ["hearing", "deadline"] {
        for suffix in [
            "?limit=0",
            "?limit=101",
            "?limit=01",
            "?limit=+2",
            "?limit=-1",
            "?limit=2&limit=3",
            "?after_id=bad",
            "?after_id=",
            "?after_id=00000000000000000000000000000000",
            "?status=other",
            "?status=",
            "?status=linked&status=all",
            "?kind=hearing",
            "?unknown=1",
        ] {
            let (status, body) = read(
                workflow.clone(),
                &format!("{}{suffix}", target_path(kind)),
                "owner",
            )
            .await;
            assert_eq!(status, 400, "{kind}{suffix}: {body}");
        }
        let path = target_path(kind);
        for invalid in [
            path.replace(CASE, "bad"),
            path.replace("/resource-associations", "x/resource-associations"),
        ] {
            let (status, body) = read(workflow.clone(), &invalid, "owner").await;
            assert_eq!(status, 400, "{invalid}: {body}");
        }
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn protected_reads_keep_authentication_denial_and_missing_target_distinct() {
    let workflow = Arc::new(Workflow::default());
    let (status, body) = read(workflow.clone(), &target_path("hearing"), "").await;
    assert_eq!(status, 401, "{body}");
    assert!(workflow.calls.lock().unwrap().is_empty());
    for (kind, token, expected_status, code) in [
        ("hearing", "client", 403, "permission_denied"),
        ("deadline", "revoked", 403, "permission_denied"),
        ("hearing", "expired", 401, "invalid_session"),
        ("hearing", "missing_hearing", 404, "hearing_not_found"),
        ("deadline", "missing_deadline", 404, "deadline_not_found"),
        ("hearing", "internal", 500, "internal_error"),
    ] {
        let (status, body) = read(workflow.clone(), &target_path(kind), token).await;
        assert_eq!(status, expected_status, "{token}: {body}");
        assert_eq!(body["error"]["code"], code);
        assert!(body.get("associations").is_none());
        assert!(!body.to_string().contains("private database"));
    }
    for token in ["owner", "litigator", "paralegal"] {
        let (status, body) = read(workflow.clone(), &target_path("deadline"), token).await;
        assert_eq!(status, 200, "{token}: {body}");
    }
    assert!(workflow.commands.lock().unwrap().is_empty());
}

#[tokio::test]
async fn inconsistent_scope_captures_observation_and_page_shapes_are_not_serialized() {
    for (token, suffix) in [
        ("foreign_case", ""),
        ("foreign_resource", ""),
        ("wrong_target", ""),
        ("wrong_kind", ""),
        ("wrong_capture", ""),
        ("wrong_current", ""),
        ("wrong_checked_at", ""),
        ("empty_bad_time", ""),
        ("duplicate", ""),
        ("descending", ""),
        ("oversized", "?limit=1"),
        ("short_page", "?limit=2"),
        ("cursor_without_more", ""),
        ("wrong_cursor", "?limit=1"),
        ("wrong_status", "?status=unlinked"),
        ("owner", "?after_id=00000000-0000-0000-0000-000000000099"),
    ] {
        let (status, body) = read(
            Arc::new(Workflow::default()),
            &format!("{}{suffix}", target_path("hearing")),
            token,
        )
        .await;
        assert_eq!(status, 500, "{token}{suffix}: {body}");
        assert_eq!(body["error"]["code"], "internal_error");
        assert!(body.get("associations").is_none());
    }
}
