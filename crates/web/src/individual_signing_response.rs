use crate::{dto::VerificationResponse, error::ApiError};
use application::document_signing::{
    CurrentSigningStatus, PreparedDocumentSigning, PreparedSigningBinding, SigningBindingReceipt,
    SigningReceipt, SigningVerification,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::{
    clock::OffsetDateTime,
    crypto::{CredentialCertificate, CredentialFailure},
    document_signing::DocumentConsent,
};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;

const POLICY: &str = "internal_individual_signing_v1";
pub(super) fn binding_preparation(row: PreparedSigningBinding) -> Value {
    let statement = row.statement();
    let material = statement.material();
    json!({"binding_id":material.binding().to_string(),"account_id":statement.account().id().to_string(),
        "account_label":row.context().principal.email,"policy":POLICY,
        "statement_base64":STANDARD.encode(statement.canonical_bytes()),
        "account_revision":statement.account().revision().to_string(),
        "auth_generation":statement.account().generation().to_string(),
        "certificate":certificate(row.certificate()),"deployment_id":material.deployment().to_string(),
        "trust_revision":material.trust_revision(),"root_fingerprint":material.root().to_hex()})
}
pub(super) fn binding_receipt(row: SigningBindingReceipt) -> Result<Value, ApiError> {
    let statement = &row.statement;
    let material = statement.material();
    Ok(
        json!({"binding_id":material.binding().to_string(),"account_id":statement.account().id().to_string(),
        "policy":POLICY,"statement_base64":STANDARD.encode(statement.canonical_bytes()),
        "signature_base64":STANDARD.encode(row.check.signature.as_bytes()),
        "account_revision":statement.account().revision().to_string(),"auth_generation":statement.account().generation().to_string(),
        "certificate":certificate(&row.check.certificate),"deployment_id":material.deployment().to_string(),
        "trust_revision":material.trust_revision(),"root_fingerprint":material.root().to_hex(),
        "registered_at":timestamp(row.registered_at)?,"withdrawn_at":row.withdrawn_at.map(timestamp).transpose()?}),
    )
}
fn certificate(value: &CredentialCertificate) -> Value {
    let summary = &value.summary;
    json!({"der_base64":STANDARD.encode(&value.der),"fingerprint":value.fingerprint.to_hex(),
        "summary":{"subject":summary.subject,"issuer":summary.issuer,"serial_hex":summary.serial_hex,
        "not_before_unix":summary.not_before_unix,"not_after_unix":summary.not_after_unix}})
}
fn consent(value: &DocumentConsent) -> Value {
    let material = value.material();
    let account = value.account();
    json!({"operation_id":value.operation().to_string(),"case_id":value.case().to_string(),
        "document_id":value.document().id.to_string(),"version":value.document().version.get(),
        "document_digest":value.content_digest().to_hex(),"pdf_signature_digest":value.pdf_signature_digest().to_hex(),
        "account_id":account.id().to_string(),"binding_id":material.binding().to_string(),
        "certificate_fingerprint":material.leaf().to_hex(),"trust_revision":material.trust_revision(),
        "account_revision":account.revision().to_string(),"auth_generation":account.generation().to_string(),
        "consent_base64":STANDARD.encode(value.canonical_bytes()),"policy":POLICY})
}
pub(super) fn document_preparation(row: PreparedDocumentSigning) -> Value {
    let mut value = consent(row.consent());
    value["name"] = json!(row.name());
    value
}
pub(super) fn receipt(row: SigningReceipt) -> Result<Value, ApiError> {
    let mut value = consent(&row.consent);
    value["status"] = json!("accepted");
    value["accepted_at"] = json!(timestamp(row.accepted_at)?);
    Ok(value)
}
pub(super) fn verification(row: SigningVerification) -> Result<Value, ApiError> {
    let (status, reason) = match row.certificate_current {
        CurrentSigningStatus::NotIndividual => ("not_individual", None),
        CurrentSigningStatus::TrustUnavailable => ("trust_unavailable", None),
        CurrentSigningStatus::Valid => ("valid", None),
        CurrentSigningStatus::Rejected(failure) => ("rejected", Some(failure_code(failure))),
    };
    Ok(
        json!({"kind":if row.signing.is_some() { "individual" } else { "legacy_server" },
        "historical":VerificationResponse::from(row.historical),
        "certificate_current":{"status":status,"reason":reason,"checked_at_unix":row.checked_at.unix_timestamp()},
        "binding_withdrawn_at":row.binding_withdrawn_at.map(timestamp).transpose()?,
        "signing":row.signing.map(receipt).transpose()?}),
    )
}
fn failure_code(value: CredentialFailure) -> &'static str {
    match value {
        CredentialFailure::LimitExceeded => "limit_exceeded",
        CredentialFailure::MalformedCertificate => "malformed_certificate",
        CredentialFailure::UnsupportedCertificate => "unsupported_certificate",
        CredentialFailure::UntrustedIssuer => "untrusted_issuer",
        CredentialFailure::NotYetValid => "not_yet_valid",
        CredentialFailure::Expired => "expired",
        CredentialFailure::MalformedCrl => "malformed_crl",
        CredentialFailure::UnsupportedCrl => "unsupported_crl",
        CredentialFailure::UntrustedCrl => "untrusted_crl",
        CredentialFailure::CrlNotYetValid => "crl_not_yet_valid",
        CredentialFailure::CrlExpired => "crl_expired",
        CredentialFailure::Revoked => "revoked",
        CredentialFailure::InvalidSignature => "invalid_signature",
    }
}
fn timestamp(value: OffsetDateTime) -> Result<String, ApiError> {
    value.format(&Rfc3339).map_err(|_| ApiError::internal())
}
