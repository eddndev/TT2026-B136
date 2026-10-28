//! Startup composition of the read-only evidence trust route.
//!
//! The configured seal and time-stamping material is decoded once; malformed
//! material stops startup instead of being served. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use std::sync::Arc;

use anyhow::Context;
use application::documents::{DocumentProcessor, EvidenceMaterial};
use application::evidence_trust::{
    EvidenceTrustService, EvidenceTrustWorkflow, ServedEvidenceMaterial,
};
use application::identity::IdentityWorkflow;
use domain::crypto::Sha256Digest;
use evidence_verify::{certificate::sha256, pem};
use infrastructure::{PostgresConnectionSource, PostgresCredentialTrustReader};

/// Builds the route workflow over the processor material and the runtime trust reader.
pub(super) fn open(
    database: &(impl PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
    processor: &DocumentProcessor,
) -> anyhow::Result<Arc<dyn EvidenceTrustWorkflow>> {
    let material = material(processor.evidence_material())?;
    let reader = PostgresCredentialTrustReader::open(database)
        .context("cannot open PostgreSQL credential trust reader")?;
    Ok(Arc::new(EvidenceTrustService::new(
        identity,
        Some(Arc::new(reader)),
        Arc::new(material),
    )))
}

/// Most certificates admitted in the configured time-stamping chain.
const MAX_TIME_STAMPING_CHAIN: usize = 8;

/// Decodes the seal root, its CRL and every time-stamping chain certificate.
///
/// Each PEM input must be exactly what the exported packages carry, so the
/// served bytes are the ones the validator compares with its pinned roots.
pub(super) fn material(evidence: &EvidenceMaterial) -> anyhow::Result<ServedEvidenceMaterial> {
    let root = pem::decode_one(&evidence.issuer_certificate_pem, "CERTIFICATE")
        .context("issuer certificate must hold exactly one strict PEM certificate")?;
    let crl = pem::decode_one(&evidence.crl_pem, "X509 CRL")
        .context("certificate revocation list must hold exactly one strict PEM CRL")?;
    let chain = evidence
        .tsa_chain_pem
        .as_deref()
        .context("timestamp authority chain is required to serve evidence trust")?;
    let time_stamping = pem::decode_all(chain, "CERTIFICATE", MAX_TIME_STAMPING_CHAIN)
        .context("timestamp authority chain must hold strict PEM certificates")?
        .into_iter()
        .map(|der| {
            let fingerprint = sha256_hex(&der);
            (der, fingerprint)
        })
        .collect();
    Ok(ServedEvidenceMaterial {
        seal_root_fingerprint: sha256_hex(&root),
        seal_root_der: root,
        seal_crl_digest: sha256_hex(&crl),
        seal_crl_der: crl,
        time_stamping,
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256Digest::from_array(sha256(bytes)).to_hex()
}

#[cfg(test)]
mod tests;
