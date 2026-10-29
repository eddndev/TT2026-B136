//! Inserts the sealed evidence manifest in the acceptance transaction.

use super::{inconsistent, port};
use application::{document_signing::SigningReceipt, ApplicationError};
use postgres::Transaction;

/// Appends the manifest row of `receipt`, after its receipt row; the
/// deferred `individual_signature_manifest_pair` trigger rejects the commit
/// without it.
pub(super) fn insert(
    tx: &mut Transaction<'_>,
    receipt: &SigningReceipt,
) -> Result<(), ApplicationError> {
    let manifest = receipt.manifest.as_ref().ok_or_else(inconsistent)?;
    let consent = receipt.consent;
    let digest = manifest.digest();
    tx.execute(
        "INSERT INTO individual_document_manifests(operation_id,document_id,document_version,
            manifest,manifest_sha256,document_sha256,manifest_token,tsa_crl_pem)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8)",
        &[
            &consent.operation(),
            &consent.document().id.as_uuid(),
            &i64::from(consent.document().version.get()),
            &manifest.manifest,
            &&digest.as_bytes()[..],
            &&consent.content_digest().as_bytes()[..],
            &manifest.token,
            &manifest.tsa_crl_pem,
        ],
    )
    .map_err(port)?;
    Ok(())
}
