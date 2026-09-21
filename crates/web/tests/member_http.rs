mod member_http_support;
use axum::http::StatusCode;
use member_http_support::*;
use serde_json::json;

#[tokio::test]
async fn directory_returns_bounded_secret_free_rows_and_exact_string_revisions() {
    let workflow = Workflow::new();
    let path = "/users?limit=1&status=all&role=paralegal&email_prefix=%20STAFF%20";
    let response = request(&workflow, "GET", path, Some("owner"), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let body = json(response).await;
    assert_eq!(
        body["items"],
        json!([{
            "id": user_id().to_string(), "email": "staff@example.com", "role": "paralegal",
            "active": true, "revision": "9007199254740993"
        }])
    );
    assert_eq!(body["has_more"], true);
    assert!(body["next_cursor"].as_str().is_some_and(|s| !s.is_empty()));
    let response = request(
        &workflow,
        "GET",
        &format!("/users/{}", user_id()),
        Some("owner"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json(response).await, body["items"][0]);
}

#[tokio::test]
async fn case_selector_returns_flat_members_and_preserves_inactive_assignments() {
    let workflow = Workflow::new();
    for selection in ["assigned", "available"] {
        let path = format!(
            "/cases/{}/members?limit=1&selection={selection}&role=paralegal&email_prefix=staff",
            case_id()
        );
        let response = request(&workflow, "GET", &path, Some("owner"), None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = json(response).await;
        assert_eq!(body["case_id"], case_id().to_string());
        assert_eq!(
            body["items"][0],
            json!({
                "id": user_id().to_string(), "email": "staff@example.com", "role": "paralegal",
                "active": selection == "available", "revision": "9007199254740993",
                "assigned_at": if selection == "assigned" { Some("1970-01-01T00:00:00Z") } else { None }
            })
        );
        assert_eq!(body["has_more"], false);
        assert!(body["next_cursor"].is_null());
    }
}

#[tokio::test]
async fn access_change_keeps_revision_precision_and_returns_confirmed_projection() {
    let workflow = Workflow::new();
    let response = request(
        &workflow,
        "PUT",
        &format!("/users/{}/access", user_id()),
        Some("owner"),
        Some(json!({"expected_revision":"9007199254740993","role":"litigator","active":false})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json(response).await,
        json!({
            "id": user_id().to_string(), "email":"staff@example.com", "role":"litigator",
            "active":false, "revision":"9007199254740994"
        })
    );
    assert_eq!(*workflow.0.lock().unwrap(), ["change"]);
}

#[tokio::test]
async fn missing_bearer_and_invalid_queries_never_call_member_workflow() {
    let workflow = Workflow::new();
    let response = request(&workflow, "GET", "/users", None, None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    for query in [
        "limit=0",
        "limit=101",
        "limit=1&limit=2",
        "status=archived",
        "role=admin",
        "selection=assigned",
        "cursor=bad",
        "email_prefix=%00",
        "status=all&status=all",
        "unexpected=x",
    ] {
        let response = request(
            &workflow,
            "GET",
            &format!("/users?{query}"),
            Some("owner"),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query}");
    }
    for query in ["selection=all", "status=active", "limit=101", "cursor=bad"] {
        let path = format!("/cases/{}/members?{query}", case_id());
        let response = request(&workflow, "GET", &path, Some("owner"), None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query}");
    }
    assert!(workflow.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn access_body_rejects_ambiguous_revisions_and_unexpected_fields_before_workflow() {
    let workflow = Workflow::new();
    let path = format!("/users/{}/access", user_id());
    for revision in [
        json!(0),
        json!(null),
        json!(""),
        json!("01"),
        json!("+1"),
        json!("-1"),
        json!(" 1"),
        json!("1.0"),
        json!("9223372036854775808"),
    ] {
        let body = json!({"expected_revision":revision,"role":"client","active":true});
        let response = request(&workflow, "PUT", &path, Some("owner"), Some(body)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    for body in [
        json!({"expected_revision":"0","role":"client","active":true,"auth_generation":1}),
        json!({"expected_revision":"0","role":"client"}),
        json!({"expected_revision":"0","role":"admin","active":true}),
    ] {
        assert_eq!(
            request(&workflow, "PUT", &path, Some("owner"), Some(body))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert!(workflow.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn unauthorized_targets_do_not_disclose_account_existence() {
    let workflow = Workflow::new();
    let present = format!("/users/{}", user_id());
    let absent = format!("/users/{}", uuid::Uuid::from_u128(99));
    for token in ["litigator", "paralegal", "client"] {
        let a = request(&workflow, "GET", &present, Some(token), None).await;
        let b = request(&workflow, "GET", &absent, Some(token), None).await;
        assert_eq!(a.status(), StatusCode::FORBIDDEN);
        assert_eq!(b.status(), StatusCode::FORBIDDEN);
        assert_eq!(json(a).await, json(b).await);
    }
    let response = request(&workflow, "GET", &absent, Some("owner"), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(json(response).await["error"]["code"], "user_not_found");
    assert_eq!(
        request(&workflow, "GET", &present, Some("expired"), None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn member_conflicts_are_stable_and_storage_details_are_not_exposed() {
    let workflow = Workflow::new();
    for (revision, code) in [
        ("0", "user_revision_conflict"),
        ("1", "last_active_owner"),
        ("2", "user_access_version_exhausted"),
    ] {
        let body = json!({"expected_revision":revision,"role":"client","active":false});
        let response = request(
            &workflow,
            "PUT",
            &format!("/users/{}/access", user_id()),
            Some("owner"),
            Some(body),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(json(response).await["error"]["code"], code);
    }
    let response = request(
        &workflow,
        "PUT",
        &format!("/users/{}/access", user_id()),
        Some("owner"),
        Some(json!({"expected_revision":"3","role":"client","active":false})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let text = json(response).await.to_string();
    assert!(!text.contains("hidden field"));
    let response = request(
        &workflow,
        "GET",
        &format!("/users/{}", user_id()),
        Some("unavailable"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!json(response).await.to_string().contains("database detail"));
}
