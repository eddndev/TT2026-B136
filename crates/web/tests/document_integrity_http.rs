use application::{document_integrity::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use domain::{
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    identity::UserId,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tower::ServiceExt;
use uuid::Uuid;

struct Workflow(AtomicUsize);

fn incident() -> DocumentIntegrityIncident {
    DocumentIntegrityIncident {
        id: DocumentIntegrityIncidentId::from_uuid(Uuid::from_u128(5)),
        observation_id: DocumentIntegrityObservationId::from_uuid(Uuid::from_u128(6)),
        case_id: CaseId::from_uuid(Uuid::from_u128(1)),
        reference: DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(2)),
            version: DocumentVersion::new(3).unwrap(),
        },
        requester: UserId::from_uuid(Uuid::from_u128(4)),
        failure: DocumentIntegrityFailure::DigestMismatch,
        detected_at: time::OffsetDateTime::UNIX_EPOCH,
        recorded_at: time::OffsetDateTime::UNIX_EPOCH,
        expected_digest: Sha256Digest::from_array([0xaa; 32]),
        observed_snapshot_digest: Sha256Digest::from_array([0xbb; 32]),
    }
}

impl DocumentIntegrityWorkflow for Workflow {
    fn list(
        &self,
        token: &str,
        query: DocumentIntegrityQuery,
    ) -> Result<DocumentIntegrityPage, ApplicationError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        if token != "owner-token" {
            return Err(ApplicationError::PermissionDenied);
        }
        assert_eq!(query.limit(), 1);
        assert_eq!(query.after_id().unwrap().as_uuid(), Uuid::from_u128(3));
        Ok(DocumentIntegrityPage {
            incidents: vec![incident()],
            has_more: true,
            next_after_id: Some(incident().id),
        })
    }
    fn get(
        &self,
        token: &str,
        id: DocumentIntegrityIncidentId,
    ) -> Result<DocumentIntegrityIncident, ApplicationError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        if token != "owner-token" {
            return Err(ApplicationError::PermissionDenied);
        }
        if id != incident().id {
            return Err(ApplicationError::DocumentIntegrityIncidentNotFound(
                id.to_string(),
            ));
        }
        Ok(incident())
    }
}

async fn request(
    workflow: &Arc<Workflow>,
    suffix: &str,
    token: Option<&str>,
) -> axum::response::Response {
    let mut request =
        Request::builder().uri(format!("/api/v1/document-integrity-incidents{suffix}"));
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    web::document_integrity_router(workflow.clone())
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn json(response: axum::response::Response) -> serde_json::Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 16384).await.unwrap()).unwrap()
}

#[tokio::test]
async fn owner_inbox_preserves_exact_identity_evidence_and_exclusive_cursor() {
    let workflow = Arc::new(Workflow(AtomicUsize::new(0)));
    let query = format!("?limit=1&after_id={}", Uuid::from_u128(3));
    let response = request(&workflow, &query, Some("owner-token")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let body = json(response).await;
    assert_eq!(body["has_more"], true);
    assert_eq!(body["next_after_id"], incident().id.to_string());
    let row = &body["incidents"][0];
    assert_eq!(row["case_id"], incident().case_id.to_string());
    assert_eq!(row["document_id"], incident().reference.id.to_string());
    assert_eq!(row["document_version"], 3);
    assert_eq!(row["requester_id"], incident().requester.to_string());
    assert_eq!(row["failure"], "digest_mismatch");
    assert_eq!(row["expected_digest"], "aa".repeat(32));
    assert_eq!(row["observed_snapshot_digest"], "bb".repeat(32));
    assert_eq!(row["detected_at"], "1970-01-01T00:00:00Z");
    assert!(row.get("vault").is_none());
    let response = request(
        &workflow,
        &format!("/{}", incident().id),
        Some("owner-token"),
    )
    .await;
    assert_eq!(json(response).await, *row);
}

#[tokio::test]
async fn inbox_authentication_and_strict_queries_precede_workflow_calls() {
    let workflow = Arc::new(Workflow(AtomicUsize::new(0)));
    assert_eq!(
        request(&workflow, "", None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    for query in [
        "?limit=0",
        "?limit=101",
        "?after_id=invalid",
        "?limit=1&limit=2",
        "?read=all",
    ] {
        let response = request(&workflow, query, Some("owner-token")).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query}");
    }
    assert_eq!(workflow.0.load(Ordering::SeqCst), 0);
    let response = request(
        &workflow,
        &format!("/{}", incident().id),
        Some("staff-token"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = json(response).await;
    assert!(body.get("document_id").is_none());
    assert_eq!(body["error"]["code"], "permission_denied");
}

#[tokio::test]
async fn missing_incident_uses_its_stable_error_and_never_returns_document_metadata() {
    let workflow = Arc::new(Workflow(AtomicUsize::new(0)));
    let response = request(
        &workflow,
        &format!("/{}", Uuid::from_u128(90)),
        Some("owner-token"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        json(response).await["error"]["code"],
        "document_integrity_incident_not_found"
    );
}
