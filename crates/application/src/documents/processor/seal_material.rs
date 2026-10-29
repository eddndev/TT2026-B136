//! Check of the signer material a server seal captures.
//!
//! A sealed version is judged at the generation time of its timestamp token
//! with the revocation list captured beside it. Material that cannot verify
//! at that instant, such as a list already past its next update because the
//! process kept it beyond its renewal, would make the new version fail its
//! historical verification permanently, so sealing refuses it. Rationale:
//! docs/adr/0019-immutable-document-versions.md.

use domain::crypto::{CertificateValidation, CertificateValidator};
use domain::DomainError;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::EvidenceMaterial;
use crate::ApplicationError;

/// Requires the signer material to be valid at `generated_at` with the
/// revocation list it will capture.
///
/// # Errors
///
/// `InvalidConfiguration` when the generation time cannot be read or the
/// material is not valid at it, including a list past its next update;
/// other validator failures keep their own error.
pub(super) fn ensure_material_valid_at_token_time(
    validator: &(dyn CertificateValidator + Send + Sync),
    material: &EvidenceMaterial,
    generated_at: &str,
) -> Result<(), ApplicationError> {
    let rejected = |reason: String| {
        ApplicationError::InvalidConfiguration(format!(
            "signer material is not valid at the timestamp generation time \
             {generated_at}: {reason}"
        ))
    };
    let at = OffsetDateTime::parse(generated_at, &Rfc3339)
        .map_err(|_| rejected("the generation time cannot be read".to_string()))?
        .unix_timestamp();
    match validator.validate(
        &material.signer_certificate_pem,
        &material.issuer_certificate_pem,
        Some(&material.crl_pem),
        at,
    ) {
        Ok(CertificateValidation::Valid) => Ok(()),
        Ok(status) => Err(rejected(status.to_string())),
        Err(DomainError::StaleCrl { next_update_unix }) => Err(rejected(format!(
            "the revocation list was due for its next update at unix time \
             {next_update_unix}"
        ))),
        Err(other) => Err(other.into()),
    }
}
