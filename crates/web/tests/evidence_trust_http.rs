//! Transport contract of the read-only evidence trust route.
use std::collections::BTreeSet;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use application::credential_trust::{CredentialTrustRevision, CredentialTrustSnapshot};
use application::evidence_trust::{EvidenceTrust, EvidenceTrustWorkflow, ServedEvidenceMaterial};
use application::ApplicationError;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    response::Response,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::clock::OffsetDateTime;
use domain::crypto::{CredentialTrustInspection, Sha256Digest};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

const PATH: &str = "/api/v1/evidence/trust";

struct Workflow {
    calls: AtomicUsize,
    snapshot: Option<CredentialTrustSnapshot>,
}

impl Workflow {
    fn new(snapshot: Option<CredentialTrustSnapshot>) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            snapshot,
        })
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl EvidenceTrustWorkflow for Workflow {
    fn current(&self, access_token: &str) -> Result<EvidenceTrust, ApplicationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if access_token != "session" {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(EvidenceTrust {
            material: Arc::new(material()),
            individual_signing: self.snapshot.clone(),
        })
    }
}

fn material() -> ServedEvidenceMaterial {
    ServedEvidenceMaterial {
        seal_root_der: vec![1, 2, 3],
        seal_root_fingerprint: "ab".repeat(32),
        seal_crl_der: vec![4, 5, 6],
        seal_crl_digest: "cd".repeat(32),
        time_stamping: vec![
            (vec![7, 7], "ef".repeat(32)),
            (vec![8, 8, 8], "01".repeat(32)),
        ],
    }
}

fn snapshot() -> CredentialTrustSnapshot {
    CredentialTrustSnapshot {
        deployment_id: Uuid::from_u128(7),
        revision: CredentialTrustRevision::new(2).unwrap(),
        inspection: CredentialTrustInspection {
            root_der: vec![9, 9],
            crl_der: vec![10],
            root_fingerprint: Sha256Digest::from_array([0x11; 32]),
            crl_digest: Sha256Digest::from_array([0x22; 32]),
            crl_number: 5,
            crl_this_update: 100,
            crl_next_update: 200,
            valid_from: 100,
            valid_until: 200,
        },
        published_at: OffsetDateTime::from_unix_timestamp(150).unwrap(),
        published_by: "trust_admin".into(),
    }
}

async fn send(workflow: &Arc<Workflow>, uri: &str, token: Option<&str>, body: Body) -> Response {
    let mut request = Request::builder().uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    web::evidence_trust_router(workflow.clone())
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap()
}

async fn json(response: Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap()
}

fn keys(value: &Value) -> BTreeSet<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

#[tokio::test]
async fn a_missing_bearer_is_rejected_before_the_workflow() {
    let workflow = Workflow::new(None);
    let response = send(&workflow, PATH, None, Body::empty()).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(workflow.calls(), 0);
}

#[tokio::test]
async fn a_rejected_session_is_unauthorized() {
    let workflow = Workflow::new(None);
    let response = send(&workflow, PATH, Some("expired"), Body::empty()).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(workflow.calls(), 1);
}

#[tokio::test]
async fn a_query_string_is_rejected_before_the_workflow() {
    let workflow = Workflow::new(None);
    for uri in [format!("{PATH}?"), format!("{PATH}?refresh=1")] {
        let response = send(&workflow, &uri, Some("session"), Body::empty()).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{uri}");
        assert_eq!(
            json(response).await["error"]["code"],
            "evidence_trust_invalid_input"
        );
    }
    assert_eq!(workflow.calls(), 0);
}

#[tokio::test]
async fn a_body_is_rejected_before_the_workflow() {
    let workflow = Workflow::new(None);
    let response = send(&workflow, PATH, Some("session"), Body::from("{}")).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json(response).await["error"]["code"],
        "evidence_trust_invalid_input"
    );
    assert_eq!(workflow.calls(), 0);
}

#[tokio::test]
async fn the_material_has_the_exact_keys_and_absent_individual_signing_is_null() {
    let workflow = Workflow::new(None);
    let response = send(&workflow, PATH, Some("session"), Body::empty()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let body = json(response).await;
    assert_eq!(
        keys(&body),
        BTreeSet::from(["individual-signing", "server-seal", "time-stamping"])
    );
    let seal = &body["server-seal"];
    assert_eq!(
        keys(seal),
        BTreeSet::from([
            "crl_der_base64",
            "crl_digest",
            "root_der_base64",
            "root_fingerprint"
        ])
    );
    assert_eq!(seal["root_der_base64"], STANDARD.encode([1, 2, 3]));
    assert_eq!(seal["root_fingerprint"], "ab".repeat(32));
    assert_eq!(seal["crl_der_base64"], STANDARD.encode([4, 5, 6]));
    assert_eq!(seal["crl_digest"], "cd".repeat(32));
    assert_eq!(keys(&body["time-stamping"]), BTreeSet::from(["roots"]));
    let roots = body["time-stamping"]["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 2);
    for (root, (der, fingerprint)) in roots.iter().zip(material().time_stamping) {
        assert_eq!(
            keys(root),
            BTreeSet::from(["root_der_base64", "root_fingerprint"])
        );
        assert_eq!(root["root_der_base64"], STANDARD.encode(der));
        assert_eq!(root["root_fingerprint"], fingerprint);
    }
    assert!(body["individual-signing"].is_null());
    assert_eq!(workflow.calls(), 1);
}

#[tokio::test]
async fn individual_signing_reuses_the_owner_certificate_trust_fields_without_the_publisher() {
    let workflow = Workflow::new(Some(snapshot()));
    let response = send(&workflow, PATH, Some("session"), Body::empty()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert!(!body.to_string().contains("trust_admin"));
    let trust = &body["individual-signing"];
    assert_eq!(
        keys(trust),
        BTreeSet::from([
            "crl_der_base64",
            "crl_digest",
            "crl_next_update_unix",
            "crl_number",
            "crl_this_update_unix",
            "deployment_id",
            "published_at",
            "revision",
            "root_der_base64",
            "root_fingerprint",
            "valid_from_unix",
            "valid_until_unix",
        ])
    );
    assert_eq!(trust["deployment_id"], Uuid::from_u128(7).to_string());
    assert_eq!(trust["revision"], 2);
    assert_eq!(trust["root_der_base64"], STANDARD.encode([9, 9]));
    assert_eq!(trust["root_fingerprint"], "11".repeat(32));
    assert_eq!(trust["crl_der_base64"], STANDARD.encode([10]));
    assert_eq!(trust["crl_digest"], "22".repeat(32));
    assert_eq!(trust["crl_number"], "5");
    assert_eq!(trust["published_at"], "1970-01-01T00:02:30Z");
}
