use std::sync::Arc;

use application::documents::{DocumentSummary, DocumentWorkflow, EvidenceExport};
use application::identity::{
    EnrollmentResult, IdentityWorkflow, LoginChallenge, Principal, SessionResult,
};
use application::verification::{ComponentReport, ComponentStatus, Verdict, VerificationReport};
use application::ApplicationError;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use domain::audit::ChainVerification;
use domain::crypto::{DocumentId, DocumentVersion};
use domain::identity::{Permission, Role, UserId};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;
use web::application_router;

const DOCUMENT_UUID: Uuid = Uuid::from_u128(0x00112233_4455_6677_8899_aabbccddeeff);

struct StubWorkflow;

impl StubWorkflow {
    fn id() -> DocumentId {
        DocumentId::from_uuid(DOCUMENT_UUID)
    }

    fn ensure_id(id: DocumentId) -> Result<(), ApplicationError> {
        if id == Self::id() {
            Ok(())
        } else {
            Err(ApplicationError::DocumentNotFound(id.to_string()))
        }
    }

    fn summary(sealed: bool) -> DocumentSummary {
        DocumentSummary {
            id: Self::id(),
            version: DocumentVersion::initial(),
            name: "acta.txt".to_string(),
            digest_hex: "ab".repeat(32),
            sealed,
        }
    }
}

impl DocumentWorkflow for StubWorkflow {
    fn upload(
        &self,
        actor: &str,
        name: &str,
        document: &[u8],
    ) -> Result<DocumentSummary, ApplicationError> {
        assert_eq!(actor, "ana@example.com");
        assert_eq!(name, "acta.txt");
        assert_eq!(document, b"case document");
        Ok(Self::summary(false))
    }

    fn seal(&self, actor: &str, id: DocumentId) -> Result<DocumentSummary, ApplicationError> {
        assert_eq!(actor, "ana@example.com");
        Self::ensure_id(id)?;
        Ok(Self::summary(true))
    }

    fn verify(&self, actor: &str, id: DocumentId) -> Result<VerificationReport, ApplicationError> {
        assert_eq!(actor, "ana@example.com");
        Self::ensure_id(id)?;
        let passed = || ComponentReport {
            status: ComponentStatus::Passed,
            detail: "accepted".to_string(),
        };
        Ok(VerificationReport {
            document_digest_hex: "ab".repeat(32),
            integrity: passed(),
            signature: passed(),
            certificate: passed(),
            timestamp: passed(),
            verdict: Verdict::Valid,
        })
    }

    fn export_evidence(
        &self,
        actor: &str,
        id: DocumentId,
    ) -> Result<EvidenceExport, ApplicationError> {
        assert_eq!(actor, "ana@example.com");
        Self::ensure_id(id)?;
        Ok(EvidenceExport {
            archive: b"zip bytes".to_vec(),
            file_name: "acta.txt-evidence.zip".to_string(),
            document_digest_hex: "ab".repeat(32),
        })
    }

    fn verify_audit(&self) -> Result<ChainVerification, ApplicationError> {
        Ok(ChainVerification::Valid { entries: 4 })
    }
}

struct StubIdentity;

impl StubIdentity {
    fn principal() -> Principal {
        Principal {
            id: UserId::from_uuid(Uuid::from_u128(7)),
            email: "ana@example.com".to_string(),
            role: Role::Owner,
        }
    }
}

impl IdentityWorkflow for StubIdentity {
    fn bootstrap_owner(
        &self,
        _email: &str,
        _password: &str,
    ) -> Result<EnrollmentResult, ApplicationError> {
        Err(ApplicationError::BootstrapClosed)
    }

    fn create_user(
        &self,
        _actor: &Principal,
        _email: &str,
        _password: &str,
        _role: Role,
    ) -> Result<EnrollmentResult, ApplicationError> {
        Err(ApplicationError::InvalidInput("unused".to_string()))
    }

    fn start_login(
        &self,
        _email: &str,
        _password: &str,
    ) -> Result<LoginChallenge, ApplicationError> {
        Err(ApplicationError::InvalidCredentials)
    }

    fn complete_totp(
        &self,
        _challenge_token: &str,
        _code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        Err(ApplicationError::MfaRejected)
    }

    fn complete_recovery(
        &self,
        _challenge_token: &str,
        _code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        Err(ApplicationError::MfaRejected)
    }

    fn authenticate(&self, token: &str) -> Result<Principal, ApplicationError> {
        if token == "owner-token" {
            Ok(Self::principal())
        } else {
            Err(ApplicationError::InvalidSession)
        }
    }

    fn authorize(
        &self,
        token: &str,
        permission: Permission,
    ) -> Result<Principal, ApplicationError> {
        let principal = self.authenticate(token)?;
        if principal.role.allows(permission) {
            Ok(principal)
        } else {
            Err(ApplicationError::PermissionDenied)
        }
    }

    fn logout(&self, _access_token: &str) -> Result<(), ApplicationError> {
        Ok(())
    }
}

fn router() -> axum::Router {
    application_router(Arc::new(StubWorkflow), Arc::new(StubIdentity))
}

async fn json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn the_document_routes_expose_the_complete_workflow() {
    let upload = router()
        .oneshot(
            Request::post("/api/v1/documents")
                .header("authorization", "Bearer owner-token")
                .header("x-document-name", "acta.txt")
                .body(Body::from("case document"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(upload.status(), StatusCode::CREATED);
    let upload_body = json(upload).await;
    assert_eq!(upload_body["id"], DOCUMENT_UUID.to_string());
    assert_eq!(upload_body["sealed"], false);

    let seal = router()
        .oneshot(
            Request::post(format!("/api/v1/documents/{DOCUMENT_UUID}/seal"))
                .header("authorization", "Bearer owner-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(seal.status(), StatusCode::OK);
    assert_eq!(json(seal).await["sealed"], true);

    let verify = router()
        .oneshot(
            Request::post(format!("/api/v1/documents/{DOCUMENT_UUID}/verify"))
                .header("authorization", "Bearer owner-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(verify.status(), StatusCode::OK);
    let verify_body = json(verify).await;
    assert_eq!(verify_body["verdict"], "valid");
    assert_eq!(verify_body["timestamp"]["status"], "passed");

    let evidence = router()
        .oneshot(
            Request::get(format!("/api/v1/documents/{DOCUMENT_UUID}/evidence"))
                .header("authorization", "Bearer owner-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(evidence.status(), StatusCode::OK);
    assert_eq!(evidence.headers()["content-type"], "application/zip");
    assert_eq!(
        evidence.headers()["content-disposition"],
        "attachment; filename=\"acta.txt-evidence.zip\""
    );
    assert_eq!(
        to_bytes(evidence.into_body(), 1024).await.unwrap(),
        "zip bytes"
    );

    let audit = router()
        .oneshot(
            Request::get("/api/v1/audit/verify")
                .header("authorization", "Bearer owner-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(audit.status(), StatusCode::OK);
    assert_eq!(json(audit).await["entries"], 4);
}

#[tokio::test]
async fn upload_requires_bearer_token_and_document_name_header() {
    let response = router()
        .oneshot(
            Request::post("/api/v1/documents")
                .body(Body::from("case document"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json(response).await["error"]["code"], "invalid_session");
}

#[tokio::test]
async fn an_unknown_document_maps_to_not_found() {
    let missing = Uuid::from_u128(1);
    let response = router()
        .oneshot(
            Request::post(format!("/api/v1/documents/{missing}/seal"))
                .header("authorization", "Bearer owner-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(json(response).await["error"]["code"], "document_not_found");
}
