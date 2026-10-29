//! The sealed manifest entries of an individual signing package: the
//! manifest, its token, the time-stamping CRL and the acceptance receipt.
//!
//! The receipt is generated here from stored data only, so repeated exports
//! return identical bytes. Rationale: the section "The evidence manifest" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use domain::{
    crypto::{ArchiveEntry, DocumentHasher},
    document_signing::DocumentConsent,
};
use evidence_verify::{
    manifest::{self, role_entry},
    receipt,
};

use super::Assembly;
use crate::{
    documents::{IndividualSigningEvidence, ManifestExport},
    ApplicationError,
};

const MANIFEST_SECTION: &str = include_str!("INDIVIDUAL-MANIFIESTO.template.md");
/// Entry of the individual signing record, which the manifest does not cover.
pub(super) const RECORD_ENTRY: &str = "individual-signing.json";

/// The instruction section for `export`, or nothing for a package without
/// a manifest.
pub(super) fn section(export: Option<&ManifestExport>, anchor: &str) -> String {
    export.map_or_else(String::new, |value| {
        MANIFEST_SECTION
            .replace("{{MANIFIESTO_DIGEST}}", &value.manifest.digest().to_hex())
            .replace("{{ANCLA_SELLO}}", anchor)
            .replace("{{ACEPTACION_AT}}", &value.accepted_at_seconds.to_string())
    })
}

/// Checks that the manifest describes exactly the assembled entries plus
/// `tsa-crl.pem`, then appends the four manifest entries.
pub(super) fn append<H: DocumentHasher + ?Sized>(
    hasher: &H,
    assembly: &mut Assembly,
    document_name: &str,
    consent: &DocumentConsent,
    individual: &IndividualSigningEvidence,
    export: &ManifestExport,
) -> Result<(), ApplicationError> {
    let stored = &export.manifest;
    let parsed = stored.check(
        consent.operation(),
        consent.document(),
        &consent.content_digest(),
    )?;
    if export.operation != consent.operation()
        || export.accepted_at_seconds != individual.accepted_at
        || export.accepted_at_nanoseconds > 999_999_999
    {
        return Err(super::individual::invalid());
    }
    let mut covered: Vec<(&str, &[u8])> = assembly
        .entries
        .iter()
        .filter(|entry| entry.name() != RECORD_ENTRY)
        .map(|entry| (entry.name(), entry.content()))
        .collect();
    covered.push((manifest::TSA_CRL_ENTRY, &stored.tsa_crl_pem));
    let described = parsed.artifacts.len() == covered.len()
        && parsed.artifacts.iter().all(|artifact| {
            artifact.entry == role_entry(artifact.role, document_name)
                && covered.iter().any(|(name, bytes)| {
                    *name == artifact.entry
                        && u64::try_from(bytes.len()).ok() == Some(artifact.size)
                        && hasher.hash_bytes(bytes).as_bytes() == &artifact.sha256
                })
        });
    if !described {
        return Err(super::individual::invalid());
    }
    let receipt = receipt::encode(&receipt::Receipt {
        operation_id: export.operation.to_string(),
        manifest_sha256: *stored.digest().as_bytes(),
        accepted_at_seconds: export.accepted_at_seconds,
        accepted_at_nanoseconds: export.accepted_at_nanoseconds,
    })
    .map_err(|_| super::individual::invalid())?;
    for (name, content) in [
        (manifest::ENTRY, stored.manifest.clone()),
        (manifest::TOKEN_ENTRY, stored.token.clone()),
        (manifest::TSA_CRL_ENTRY, stored.tsa_crl_pem.clone()),
        (receipt::ENTRY, receipt),
    ] {
        assembly.entries.push(ArchiveEntry::new(name, content)?);
    }
    Ok(())
}
