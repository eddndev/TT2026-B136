mod participant_support;

use axum::{body::Body, http::StatusCode};
use participant_support::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn malformed_and_spoofed_json_never_reaches_participant_commands() {
    let workflow = Arc::new(Workflow::default());
    for raw in ["null", "[]", "{}", "{", "{\"display_name\":3,\"procedural_role\":\"Witness\"}",
        "{\"display_name\":\"Ana\",\"display_name\":\"Other\",\"procedural_role\":\"Witness\"}",
        "{\"display_name\":\"Ana\",\"procedural_role\":\"Witness\",\"directory_status\":\"archived\"}"] {
        assert_eq!(request(&workflow, "POST", &base(), Some("owner"), raw).await.status(), StatusCode::BAD_REQUEST);
    }
    for field in [
        "id",
        "case_id",
        "changed_by",
        "changed_at",
        "values_digest",
        "user_id",
    ] {
        let mut input = replacement(3);
        input[field] = json!("spoofed");
        assert_eq!(
            request(&workflow, "PUT", &item(), Some("owner"), input.to_string())
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    for value in [
        json!(-1),
        json!(4294967296_u64),
        json!(1.5),
        json!("3"),
        json!(null),
    ] {
        let mut input = replacement(3);
        input["expected_revision"] = value;
        assert_eq!(
            request(&workflow, "PUT", &item(), Some("owner"), input.to_string())
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    for status in ["Active", "active ", "all", "deleted"] {
        let mut input = replacement(3);
        input["directory_status"] = json!(status);
        assert_eq!(
            request(&workflow, "PUT", &item(), Some("owner"), input.to_string())
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    for raw in [
        "{\"expected_revision\":3,\"directory_status\":\"active\",\"display_name\":\"stale name\"}",
        "{\"expected_revision\":3,\"directory_status\":\"active\",\"expected_revision\":2}",
    ] {
        assert_eq!(
            request(
                &workflow,
                "PUT",
                &format!("{}/directory-status", item()),
                Some("owner"),
                raw
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn participant_semantic_errors_are_422_and_zero_revision_is_not_json_failure() {
    let workflow = Arc::new(Workflow::default());
    for (field, value) in [
        ("display_name", " ".into()),
        ("display_name", "x".repeat(201)),
        ("procedural_role", "x".repeat(81)),
        ("organization", "x".repeat(201)),
        ("legal_status", "x".repeat(161)),
        ("display_name", "\nAna".into()),
        ("legal_status", "\u{0085}status".into()),
    ] {
        let mut input = replacement(3);
        input[field] = json!(value);
        let response = request(&workflow, "PUT", &item(), Some("owner"), input.to_string()).await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{field}"
        );
        assert_eq!(
            body(response).await["error"]["code"],
            "invalid_participant_values"
        );
    }
    for (path, input) in [
        (item(), replacement(0)),
        (
            format!("{}/directory-status", item()),
            json!({"expected_revision":0,"directory_status":"active"}),
        ),
    ] {
        let response = request(&workflow, "PUT", &path, Some("owner"), input.to_string()).await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            body(response).await["error"]["code"],
            "invalid_participant_revision"
        );
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn query_types_are_strict_and_semantic_bounds_have_separate_errors() {
    let workflow = Arc::new(Workflow::default());
    for (suffix, query, status) in [
        ("", "limit=0", 422),
        ("", "limit=101", 422),
        ("", "limit=-1", 400),
        ("", "limit=1&limit=2", 400),
        ("", "offset=1", 400),
        ("", "status=Active", 400),
        ("", "after_id=bad", 400),
        ("", "name=%00Ana", 422),
        ("", "procedural_role=Witness%0A", 422),
        ("/history", "before_revision=0", 422),
        ("/history", "before_revision=-1", 400),
        ("/history", "before_revision=4294967296", 400),
        ("/history", "limit=101", 422),
        ("/history", "status=all", 400),
    ] {
        let path = if suffix.is_empty() {
            base()
        } else {
            format!("{}{suffix}", item())
        };
        let response = request(
            &workflow,
            "GET",
            &format!("{path}?{query}"),
            Some("owner"),
            Body::empty(),
        )
        .await;
        assert_eq!(response.status().as_u16(), status, "{path}?{query}");
    }
    for (field, limit) in [("name", 200), ("procedural_role", 80)] {
        let response = request(
            &workflow,
            "GET",
            &format!("{}?{field}={}", base(), "x".repeat(limit + 1)),
            Some("owner"),
            Body::empty(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn malformed_resource_ids_are_rejected_without_workflow_calls() {
    let workflow = Arc::new(Workflow::default());
    for (path, code) in [
        (base().replace(CASE, "invalid"), "invalid_case_id"),
        (format!("{}/bad", base()), "invalid_participant_id"),
        (format!("{}/bad/history", base()), "invalid_participant_id"),
    ] {
        let response = request(&workflow, "GET", &path, Some("owner"), Body::empty()).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body(response).await["error"]["code"], code);
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn the_eight_kibibyte_body_limit_includes_trailing_transport_data() {
    let workflow = Arc::new(Workflow::default());
    let mut raw = replacement(3).to_string();
    raw.extend(std::iter::repeat_n(' ', 8192 - raw.len()));
    assert_eq!(
        request(&workflow, "PUT", &item(), Some("owner"), raw.clone())
            .await
            .status(),
        StatusCode::OK
    );
    raw.push(' ');
    let response = request(&workflow, "PUT", &item(), Some("owner"), raw).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        body(response).await["error"]["code"],
        "participant_body_too_large"
    );
    let chunks = futures_util::stream::iter([
        Ok::<_, std::io::Error>(replacement(3).to_string()),
        Ok(" ".repeat(8192)),
    ]);
    assert_eq!(
        request(
            &workflow,
            "PUT",
            &item(),
            Some("owner"),
            Body::from_stream(chunks)
        )
        .await
        .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(workflow.calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn largest_escaped_unicode_values_fit_the_declared_json_limit() {
    let workflow = Arc::new(Workflow::default());
    let escaped = |length: usize| "\\ud800\\udc00".repeat(length);
    let input = format!(
        concat!(
            "{{\"expected_revision\":4294967295,\"display_name\":\"{}\",",
            "\"procedural_role\":\"{}\",\"organization\":\"{}\",\"legal_status\":\"{}\",",
            "\"directory_status\":\"archived\"}}"
        ),
        escaped(200),
        escaped(80),
        escaped(200),
        escaped(160)
    );
    assert_eq!(input.len(), 7817);
    assert_eq!(
        request(&workflow, "PUT", &item(), Some("owner"), input)
            .await
            .status(),
        StatusCode::OK
    );
    let calls = workflow.calls.lock().unwrap();
    assert_eq!(
        calls[0][5]["display_name"]
            .as_str()
            .unwrap()
            .chars()
            .count(),
        200
    );
}

#[tokio::test]
async fn manual_commands_reject_trailing_entities_before_any_mutation() {
    let workflow = Arc::new(Workflow::default());
    for (method, path, input) in [
        (
            "POST",
            base(),
            json!({"display_name":"Ana","procedural_role":"Witness"}),
        ),
        ("PUT", item(), replacement(3)),
        (
            "PUT",
            format!("{}/directory-status", item()),
            json!({"expected_revision":3,"directory_status":"active"}),
        ),
    ] {
        let response = request(
            &workflow,
            method,
            &path,
            Some("owner"),
            format!("{input}{{}}"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    assert!(workflow.calls.lock().unwrap().is_empty());
}
