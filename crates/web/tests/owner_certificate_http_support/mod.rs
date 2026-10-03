use std::{sync::Arc, time::Duration};

use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, Request, StatusCode},
    Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use tower::ServiceExt;
use web::{owner_certificate_router, HttpLimits};

use crate::support::*;

pub fn router(harness: Harness) -> Router {
    owner_certificate_router(Arc::new(harness.service()), HttpLimits::default())
}

pub fn path(operation: &str) -> String {
    format!("/api/v1/auth/certificate-bindings/{}{operation}", binding())
}

pub fn prepare_body() -> Value {
    json!({"certificate_base64":STANDARD.encode(RAW_CERTIFICATE)})
}

pub fn registration_body() -> Value {
    json!({"statement_base64":STANDARD.encode(statement().canonical_bytes()),
        "certificate_der_base64":STANDARD.encode(certificate().der),
        "signature_base64":STANDARD.encode(signature().as_bytes())})
}

pub struct Reply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub bytes: Vec<u8>,
}

impl Reply {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.bytes).unwrap()
    }

    pub fn public(&self) {
        assert_eq!(self.headers["cache-control"], "no-store");
        assert!(!self.headers.contains_key("set-cookie"));
        assert!(!self.headers.contains_key("authorization"));
        let text = String::from_utf8_lossy(&self.bytes);
        for secret in [
            "Bearer session",
            "private-port-detail",
            "password_hash",
            "totp_secret",
            "access_token",
            "private_key",
            "recovery_codes",
        ] {
            assert!(
                !text.contains(secret),
                "response included a secret or diagnostic field"
            );
        }
    }

    pub fn error(&self, status: StatusCode, code: &str) {
        self.public();
        assert_eq!(self.status, status);
        let value = self.json();
        assert_eq!(value["error"]["code"], code);
        assert!(value["error"]["message"]
            .as_str()
            .is_some_and(|s| !s.is_empty()));
        assert_eq!(value.as_object().unwrap().len(), 1);
    }
}

pub async fn raw(
    router: &Router,
    method: &str,
    path: &str,
    bytes: Vec<u8>,
    types: &[&str],
    auth: &[&str],
) -> Reply {
    let mut request = Request::builder().method(method).uri(path);
    for value in types {
        request = request.header("content-type", *value);
    }
    for value in auth {
        request = request.header("authorization", *value);
    }
    let response = tokio::time::timeout(
        Duration::from_secs(2),
        router
            .clone()
            .oneshot(request.body(Body::from(bytes)).unwrap()),
    )
    .await
    .expect("bounded Owner HTTP request did not finish")
    .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    Reply {
        status,
        headers,
        bytes,
    }
}

pub async fn request(router: &Router, method: &str, operation: &str, body: Option<Value>) -> Reply {
    let types = if body.is_some() {
        vec!["application/json"]
    } else {
        vec![]
    };
    raw(
        router,
        method,
        &path(operation),
        body.map(|v| v.to_string().into_bytes()).unwrap_or_default(),
        &types,
        &["Bearer session"],
    )
    .await
}

pub fn decoded(value: &Value) -> Vec<u8> {
    STANDARD.decode(value.as_str().unwrap()).unwrap()
}

pub fn assert_evidence(
    value: &Value,
    expected: &application::identity::owner_certificates::OwnerBindingReceipt,
) {
    let original = expected.record.registration();
    assert_eq!(
        value["binding_id"],
        original.material().binding().to_string()
    );
    assert_eq!(value["owner_id"], expected.owner.to_string());
    assert_eq!(value["revision"], expected.record.revision());
    assert_eq!(value["policy"], "internal_partner_binding_v1");
    let registration = &value["registration"];
    assert_eq!(
        decoded(&registration["statement_base64"]),
        original.canonical_bytes()
    );
    assert_eq!(
        registration["statement_digest"],
        expected.check.statement_digest.to_hex()
    );
    assert_eq!(
        decoded(&registration["signature_base64"]),
        expected.check.signature.as_bytes()
    );
    assert_eq!(
        decoded(&registration["certificate"]["der_base64"]),
        expected.check.certificate.der
    );
    assert_eq!(
        registration["certificate"]["fingerprint"],
        expected.check.certificate.fingerprint.to_hex()
    );
    let summary = &expected.check.certificate.summary;
    assert_eq!(
        registration["certificate"]["summary"],
        json!({"subject":summary.subject,
        "issuer":summary.issuer,"serial_hex":summary.serial_hex,
        "not_before_unix":summary.not_before_unix,"not_after_unix":summary.not_after_unix})
    );
    assert_eq!(
        registration["account_revision"],
        original.owner().revision().to_string()
    );
    assert_eq!(
        registration["auth_generation"],
        original.owner().generation().to_string()
    );
    assert_eq!(registration["checked_at_unix"], expected.check.checked_at);
    assert_eq!(registration["valid_from_unix"], expected.check.valid_from);
    assert_eq!(registration["valid_until_unix"], expected.check.valid_until);
    assert_eq!(
        registration["registered_at"],
        expected
            .registered_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap()
    );
    let trust = &registration["trust"];
    assert_eq!(
        trust["deployment_id"],
        expected.trust.deployment_id.to_string()
    );
    assert_eq!(trust["revision"], expected.trust.revision.get());
    assert_eq!(
        decoded(&trust["root_der_base64"]),
        expected.trust.inspection.root_der
    );
    assert_eq!(
        decoded(&trust["crl_der_base64"]),
        expected.trust.inspection.crl_der
    );
    assert_eq!(
        trust["root_fingerprint"],
        expected.trust.inspection.root_fingerprint.to_hex()
    );
    assert_eq!(
        trust["crl_digest"],
        expected.trust.inspection.crl_digest.to_hex()
    );
    assert_eq!(
        trust["crl_number"],
        expected.trust.inspection.crl_number.to_string()
    );
    assert_eq!(
        trust["crl_this_update_unix"],
        expected.trust.inspection.crl_this_update
    );
    assert_eq!(
        trust["crl_next_update_unix"],
        expected.trust.inspection.crl_next_update
    );
    assert_eq!(
        trust["valid_from_unix"],
        expected.trust.inspection.valid_from
    );
    assert_eq!(
        trust["valid_until_unix"],
        expected.trust.inspection.valid_until
    );
    assert_eq!(
        trust["published_at"],
        expected
            .trust
            .published_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap()
    );
    assert_eq!(trust["published_by"], expected.trust.published_by);
    match expected.record.withdrawal() {
        None => assert!(value["withdrawal"].is_null()),
        Some(withdrawal) => {
            assert_eq!(
                decoded(&value["withdrawal"]["statement_base64"]),
                withdrawal.canonical_bytes()
            );
            assert_eq!(
                value["withdrawal"]["account_revision"],
                withdrawal.owner().revision().to_string()
            );
            assert_eq!(
                value["withdrawal"]["auth_generation"],
                withdrawal.owner().generation().to_string()
            );
            assert_eq!(
                value["withdrawal"]["withdrawn_at"],
                expected
                    .withdrawn_at
                    .unwrap()
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap()
            );
        }
    }
}
