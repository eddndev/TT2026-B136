use application::{
    credential_trust::CredentialTrustSnapshot,
    identity::owner_certificates::{OwnerBindingReceipt, PreparedOwnerRegistration},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::{
    clock::OffsetDateTime,
    crypto::{CertificateSummary, CredentialCertificate},
};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;

use crate::error::ApiError;

const POLICY: &str = "internal_partner_binding_v1";

pub(super) fn preparation(row: PreparedOwnerRegistration) -> Value {
    let statement = row.statement();
    let material = statement.material();
    json!({"binding_id":material.binding().to_string(),"owner_id":statement.owner().id().to_string(),
        "policy":POLICY,"statement_base64":STANDARD.encode(statement.canonical_bytes()),
        "account_revision":statement.owner().revision().to_string(),
        "auth_generation":statement.owner().generation().to_string(),
        "certificate":certificate(row.certificate()),"deployment_id":material.deployment().to_string(),
        "trust_revision":material.trust_revision(),"root_fingerprint":material.root().to_hex()})
}

pub(super) fn receipt(row: OwnerBindingReceipt) -> Result<Value, ApiError> {
    let statement = row.record.registration();
    let check = &row.check;
    let withdrawal = match row.record.withdrawal() {
        Some(value) => Some(
            json!({"statement_base64":STANDARD.encode(value.canonical_bytes()),
            "account_revision":value.owner().revision().to_string(),
            "auth_generation":value.owner().generation().to_string(),
            "withdrawn_at":timestamp(row.withdrawn_at.ok_or_else(ApiError::internal)?)?}),
        ),
        None => None,
    };
    Ok(
        json!({"binding_id":statement.material().binding().to_string(),"owner_id":row.owner.to_string(),
        "revision":row.record.revision(),"policy":POLICY,
        "registration":{"statement_base64":STANDARD.encode(statement.canonical_bytes()),
            "statement_digest":check.statement_digest.to_hex(),"certificate":certificate(&check.certificate),
            "signature_base64":STANDARD.encode(check.signature.as_bytes()),
            "account_revision":statement.owner().revision().to_string(),
            "auth_generation":statement.owner().generation().to_string(),
            "checked_at_unix":check.checked_at,"valid_from_unix":check.valid_from,
            "valid_until_unix":check.valid_until,"registered_at":timestamp(row.registered_at)?,
            "trust":trust(&row.trust)?},"withdrawal":withdrawal}),
    )
}

fn certificate(value: &CredentialCertificate) -> Value {
    json!({"der_base64":STANDARD.encode(&value.der),"fingerprint":value.fingerprint.to_hex(),
        "summary":summary(&value.summary)})
}

fn summary(value: &CertificateSummary) -> Value {
    json!({"subject":value.subject,"issuer":value.issuer,"serial_hex":value.serial_hex,
        "not_before_unix":value.not_before_unix,"not_after_unix":value.not_after_unix})
}

fn trust(value: &CredentialTrustSnapshot) -> Result<Value, ApiError> {
    let inspection = &value.inspection;
    Ok(
        json!({"deployment_id":value.deployment_id.to_string(),"revision":value.revision.get(),
        "root_der_base64":STANDARD.encode(&inspection.root_der),"crl_der_base64":STANDARD.encode(&inspection.crl_der),
        "root_fingerprint":inspection.root_fingerprint.to_hex(),"crl_digest":inspection.crl_digest.to_hex(),
        "crl_number":inspection.crl_number.to_string(),"crl_this_update_unix":inspection.crl_this_update,
        "crl_next_update_unix":inspection.crl_next_update,"valid_from_unix":inspection.valid_from,
        "valid_until_unix":inspection.valid_until,"published_at":timestamp(value.published_at)?,
        "published_by":value.published_by}),
    )
}

fn timestamp(value: OffsetDateTime) -> Result<String, ApiError> {
    value.format(&Rfc3339).map_err(|_| ApiError::internal())
}
