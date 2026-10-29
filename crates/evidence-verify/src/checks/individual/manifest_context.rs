//! The context rules individual signing v2 adds: the order of the document
//! token, the manifest token and the acceptance, the registration instant
//! the manifest covers, and the acceptance receipt.
//!
//! The document token's `genTime` must not be after the manifest token's,
//! since the server requests the manifest token later and rejects an
//! authority that answers out of order; the manifest token's `genTime` must
//! not be after `accepted_at` beyond its accuracy; the manifest's
//! `binding_registered_at` must equal the record in
//! `individual-signing.json`; and the receipt must state the same
//! acceptance second, the digest of the manifest entry and the operation.
//! Rationale: the sections "Instants and consistency" and "The evidence
//! manifest" of `docs/adr/0085-in-browser-evidence-validation.md`.

use crate::individual_json::IndividualSigning;
use crate::manifest::{Manifest, TOKEN_ENTRY};
use crate::receipt::{self, Receipt};
use crate::report::{Finding, Outcome};
use crate::timestamp::TokenFacts;

pub(in crate::checks) struct Input<'a> {
    pub record: Option<&'a IndividualSigning>,
    pub document_token: Option<&'a TokenFacts>,
    pub manifest_token: Option<&'a TokenFacts>,
    pub manifest: Option<&'a Manifest>,
    /// The operation the receipt must name: the manifest's, or the consent
    /// statement's when the manifest did not parse.
    pub operation_id: Option<String>,
    /// SHA-256 of the exact manifest entry bytes.
    pub manifest_sha256: [u8; 32],
    /// The receipt entry bytes.
    pub receipt: &'a [u8],
}

/// The findings of these rules and the receipt, when it parsed.
pub(in crate::checks) fn check(input: &Input<'_>) -> (Vec<Finding>, Option<Receipt>) {
    let mut findings = Vec::new();
    match input.manifest_token {
        None => findings.push(Finding::indeterminate(
            "context.token_time_unknown",
            format!("{TOKEN_ENTRY} did not yield a verified genTime"),
        )),
        Some(manifest) => {
            if let Some(document) = input.document_token {
                findings.extend(token_order(document, manifest));
            }
            if let Some(record) = input.record {
                findings.push(manifest_before_acceptance(manifest, record));
            }
        }
    }
    if let (Some(manifest), Some(record)) = (input.manifest, input.record) {
        findings.extend(registration(manifest, record));
    }
    let receipt = match receipt::parse(input.receipt) {
        Ok(receipt) => {
            findings.extend(receipt_matches(input, &receipt));
            Some(receipt)
        }
        Err(finding) => {
            findings.push(crate::checks::in_entry(receipt::ENTRY, finding));
            None
        }
    };
    (findings, receipt)
}

fn token_order(document: &TokenFacts, manifest: &TokenFacts) -> Option<Finding> {
    (document.gen_time > manifest.gen_time).then(|| {
        Finding::failed(
            "context.document_after_manifest",
            format!(
                "document token genTime {} is after manifest token genTime {}",
                document.gen_time, manifest.gen_time
            ),
        )
    })
}

fn manifest_before_acceptance(manifest: &TokenFacts, record: &IndividualSigning) -> Finding {
    if manifest.gen_time.saturating_sub(manifest.accuracy_seconds) <= record.accepted_at {
        Finding::new(
            Outcome::Checked,
            "context.token_before_acceptance",
            TOKEN_ENTRY,
        )
    } else {
        Finding::failed(
            "context.manifest_after_acceptance",
            format!(
                "manifest token genTime {} with accuracy {} s is after accepted_at {}",
                manifest.gen_time, manifest.accuracy_seconds, record.accepted_at
            ),
        )
    }
}

fn registration(manifest: &Manifest, record: &IndividualSigning) -> Option<Finding> {
    let covered = manifest.signers.first()?.binding_registered_at;
    (covered != record.binding_registered_at).then(|| {
        Finding::failed(
            "context.binding_registration_mismatch",
            format!(
                "the manifest states {covered}, individual-signing.json {}",
                record.binding_registered_at
            ),
        )
    })
}

fn receipt_matches(input: &Input<'_>, receipt: &Receipt) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let Some(record) = input.record {
        if receipt.accepted_at_seconds != record.accepted_at {
            findings.push(Finding::failed(
                "receipt.seconds_mismatch",
                format!(
                    "the receipt states {}, individual-signing.json {}",
                    receipt.accepted_at_seconds, record.accepted_at
                ),
            ));
        }
    }
    if receipt.manifest_sha256 != input.manifest_sha256 {
        findings.push(Finding::failed(
            "receipt.manifest_mismatch",
            "manifest_sha256 is not the digest of the manifest entry",
        ));
    }
    if let Some(operation) = &input.operation_id {
        if &receipt.operation_id != operation {
            findings.push(Finding::failed(
                "receipt.operation_mismatch",
                "operation_id is not the operation of the manifest",
            ));
        }
    }
    findings
}
