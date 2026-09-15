mod document_support;

use std::sync::{atomic::Ordering, Arc};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use document_support::{StubIdentity, StubWorkflow, CASE_UUID, DOCUMENT_UUID};
use serde_json::Value;
use tower::ServiceExt;

fn collection() -> String {
    format!("/api/v1/cases/{CASE_UUID}/documents")
}

async fn get(
    workflow: &Arc<StubWorkflow>,
    path: &str,
    token: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::get(path);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    web::application_router(workflow.clone(), Arc::new(StubIdentity))
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}

#[tokio::test]
async fn document_listing_returns_authorized_metadata_and_pagination() {
    let workflow = Arc::new(StubWorkflow::default());
    let response = get(&workflow, &collection(), Some("owner-token")).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["documents"][0]["case_id"], CASE_UUID.to_string());
    assert_eq!(body["documents"][0]["id"], DOCUMENT_UUID.to_string());
    assert_eq!(body["has_more"], false);
    assert!(body["documents"][0].get("vault").is_none());
    assert!(body["documents"][0].get("evidence").is_none());
}

#[tokio::test]
async fn document_detail_returns_the_persisted_summary() {
    let workflow = Arc::new(StubWorkflow::default());
    let response = get(
        &workflow,
        &format!("{}/{DOCUMENT_UUID}", collection()),
        Some("owner-token"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["name"], "acta.txt");
    assert_eq!(body["case_id"], CASE_UUID.to_string());
    assert_eq!(body["version"], 1);
    assert_eq!(body["current_metadata"]["metadata_revision"], 3);
}

#[tokio::test]
async fn classification_filters_are_normalized_individual_values_combined_with_content_filters() {
    let workflow = Arc::new(StubWorkflow::default());
    for (query, count) in [
        (
            "document_type=%20Escrito%20&classification=Penal&tag=a%2Cb&name=ACTA&sealed=false",
            1,
        ),
        ("document_type=escrito", 0),
        ("classification=penal", 0),
        ("tag=acci%C3%B3n", 1),
        ("tag=a", 0),
        ("tag=%25", 0),
    ] {
        let response = get(
            &workflow,
            &format!("{}?{query}", collection()),
            Some("owner-token"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            json(response).await["documents"].as_array().unwrap().len(),
            count,
            "{query}"
        );
    }
}

#[tokio::test]
async fn invalid_classification_filters_are_rejected_before_querying_documents() {
    let workflow = Arc::new(StubWorkflow::default());
    for query in [
        "tag=".to_owned(),
        "classification=%0APenal".to_owned(),
        format!("document_type={}", "x".repeat(81)),
        format!("tag={}", "x".repeat(41)),
    ] {
        let response = get(
            &workflow,
            &format!("{}?{query}", collection()),
            Some("owner-token"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn list_passes_search_status_and_page_to_the_workflow() {
    let workflow = Arc::new(StubWorkflow::default());
    for (query, count) in [
        ("name=ACTA&limit=1&offset=0&sealed=false", 1),
        ("name=missing", 0),
        ("sealed=true", 0),
        ("offset=1", 0),
    ] {
        let response = get(
            &workflow,
            &format!("{}?{query}", collection()),
            Some("owner-token"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            json(response).await["documents"].as_array().unwrap().len(),
            count
        );
    }
}

#[tokio::test]
async fn document_queries_require_sessions_and_preserve_role_denials() {
    let workflow = Arc::new(StubWorkflow::default());
    for path in [collection(), format!("{}/{DOCUMENT_UUID}", collection())] {
        for (token, expected) in [
            (None, StatusCode::UNAUTHORIZED),
            (Some("expired-token"), StatusCode::UNAUTHORIZED),
            (Some("client-token"), StatusCode::FORBIDDEN),
        ] {
            assert_eq!(get(&workflow, &path, token).await.status(), expected);
        }
    }
}

#[tokio::test]
async fn invalid_filters_are_rejected_before_the_workflow() {
    let workflow = Arc::new(StubWorkflow::default());
    for query in ["limit=0", "limit=101", "name=line%0Abreak"] {
        assert_eq!(
            get(
                &workflow,
                &format!("{}?{query}", collection()),
                Some("owner-token")
            )
            .await
            .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    for query in ["offset=-1", "limit=abc", "sealed=yes", "unknown=value"] {
        assert_eq!(
            get(
                &workflow,
                &format!("{}?{query}", collection()),
                Some("owner-token")
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(workflow.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn detail_hides_foreign_and_nonexistent_document_associations() {
    let workflow = Arc::new(StubWorkflow::default());
    let mut errors = Vec::new();
    for path in [
        format!(
            "/api/v1/cases/{}/documents/{DOCUMENT_UUID}",
            uuid::Uuid::from_u128(1)
        ),
        format!("{}/{}", collection(), uuid::Uuid::from_u128(1)),
    ] {
        let response = get(&workflow, &path, Some("owner-token")).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        errors.push(json(response).await);
    }
    assert_eq!(errors[0], errors[1]);
    assert_eq!(errors[0]["error"]["code"], "document_not_found");
}
