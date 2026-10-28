use application::{credential_trust::CredentialTrustSnapshot, evidence_trust::EvidenceTrust};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;

use crate::error::ApiError;

/// Purpose names match the pinned root purposes of the validator.
pub(super) fn trust(value: &EvidenceTrust) -> Result<Value, ApiError> {
    let material = &value.material;
    let roots: Vec<Value> = material
        .time_stamping
        .iter()
        .map(|(der, fingerprint)| {
            json!({"root_der_base64":STANDARD.encode(der),"root_fingerprint":fingerprint})
        })
        .collect();
    let individual = match &value.individual_signing {
        Some(snapshot) => credential_trust(snapshot)?,
        None => Value::Null,
    };
    Ok(json!({
        "server-seal":{"root_der_base64":STANDARD.encode(&material.seal_root_der),
            "root_fingerprint":material.seal_root_fingerprint,
            "crl_der_base64":STANDARD.encode(&material.seal_crl_der),
            "crl_digest":material.seal_crl_digest},
        "time-stamping":{"roots":roots},
        "individual-signing":individual,
    }))
}

/// The credential trust snapshot with the field names of the Owner
/// certificate trust responses, without `published_by`: that is the database
/// role that published it, which is not trust material and is not shown to
/// every authenticated principal.
fn credential_trust(value: &CredentialTrustSnapshot) -> Result<Value, ApiError> {
    let inspection = &value.inspection;
    let published_at = value
        .published_at
        .format(&Rfc3339)
        .map_err(|_| ApiError::internal())?;
    Ok(
        json!({"deployment_id":value.deployment_id.to_string(),"revision":value.revision.get(),
        "root_der_base64":STANDARD.encode(&inspection.root_der),"crl_der_base64":STANDARD.encode(&inspection.crl_der),
        "root_fingerprint":inspection.root_fingerprint.to_hex(),"crl_digest":inspection.crl_digest.to_hex(),
        "crl_number":inspection.crl_number.to_string(),"crl_this_update_unix":inspection.crl_this_update,
        "crl_next_update_unix":inspection.crl_next_update,"valid_from_unix":inspection.valid_from,
        "valid_until_unix":inspection.valid_until,"published_at":published_at}),
    )
}
