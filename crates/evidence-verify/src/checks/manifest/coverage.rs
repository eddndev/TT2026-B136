//! The manifest against the package: every covered entry listed once with
//! its digest, size and role, the document digest, the signer's entries and
//! the holder's consent statement.
//!
//! The covered entries are all package entries except the manifest, its
//! token, `individual-signing.json`, the instructions and the receipt. A
//! listed entry the package lacks, a covered entry the manifest omits, a
//! digest or size that differs, and a role that names another entry than
//! the one the server writes for it each fail. An entry name the manifest
//! states, unlike a package entry name, is not restricted by admission, so
//! details show it escaped. Rationale: the section "The evidence manifest"
//! of `docs/adr/0085-in-browser-evidence-validation.md`.

use crate::certificate;
use crate::checks::Evidence;
use crate::manifest::schema::shown;
use crate::manifest::{role_entry, Artifact, Manifest, Role, ENTRY, TOKEN_ENTRY};
use crate::profile::INSTRUCTIONS;
use crate::receipt;
use crate::report::Finding;
use crate::statements::{uuid_text, ConsentStatement};

/// Entries of a v2 package that the manifest does not cover.
const UNCOVERED: [&str; 5] = [
    ENTRY,
    TOKEN_ENTRY,
    "individual-signing.json",
    INSTRUCTIONS,
    receipt::ENTRY,
];

/// The failures of `manifest` against the package of `evidence` and the
/// decoded consent statement, when it decoded.
pub(super) fn check(
    evidence: &Evidence<'_>,
    manifest: &Manifest,
    consent: Option<&ConsentStatement>,
) -> Vec<Finding> {
    let mut findings = artifacts(evidence, manifest);
    findings.extend(document(evidence, manifest));
    if let Some(consent) = consent {
        findings.extend(consent_matches(manifest, consent));
    }
    findings.extend(signer_entries(manifest));
    findings
}

fn listed<'a>(manifest: &'a Manifest, entry: &str) -> Option<&'a Artifact> {
    manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.entry == entry)
}

fn artifacts(evidence: &Evidence<'_>, manifest: &Manifest) -> Vec<Finding> {
    let mut findings = Vec::new();
    let covered: Vec<&str> = evidence
        .archive
        .names()
        .into_iter()
        .filter(|name| !UNCOVERED.contains(name))
        .collect();
    for name in &covered {
        if listed(manifest, name).is_none() {
            findings.push(Finding::failed(
                "manifest.artifact_missing",
                format!("{name} is not listed"),
            ));
        }
    }
    for artifact in &manifest.artifacts {
        let entry = artifact.entry.as_str();
        if !covered.contains(&entry) {
            findings.push(Finding::failed(
                "manifest.artifact_unexpected",
                format!("{} is not a covered entry of the package", shown(entry)),
            ));
        } else {
            let bytes = evidence.entry(entry);
            let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
            if artifact.sha256 != certificate::sha256(bytes) || artifact.size != size {
                findings.push(Finding::failed(
                    "manifest.artifact_mismatch",
                    format!("{entry} differs from its manifest digest or size"),
                ));
            }
        }
        let expected = role_entry(artifact.role, &evidence.layout.document);
        if entry != expected {
            findings.push(Finding::failed(
                "manifest.role_mismatch",
                format!(
                    "{} names {}, not {expected}",
                    artifact.role.as_str(),
                    shown(entry)
                ),
            ));
        }
    }
    findings
}

fn document(evidence: &Evidence<'_>, manifest: &Manifest) -> Option<Finding> {
    let actual = certificate::sha256(evidence.entry(&evidence.layout.document));
    let listed = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.role == Role::Document)
        .map(|artifact| artifact.sha256);
    (manifest.document_sha256 != actual || listed != Some(actual)).then(|| {
        Finding::failed(
            "manifest.document_mismatch",
            "document_sha256 or the document artifact is not the digest of the document",
        )
    })
}

/// The consent statement must name the manifest's operation, case,
/// document, version and digest, the digest of the signer's document
/// signature, and the signer's deployment, account and binding.
fn consent_matches(manifest: &Manifest, consent: &ConsentStatement) -> Vec<Finding> {
    let mut findings = Vec::new();
    let document_fields = [
        (
            "operation_id",
            uuid_text(consent.operation) == manifest.operation_id,
        ),
        ("case_id", uuid_text(consent.case) == manifest.case_id),
        (
            "document_id",
            uuid_text(consent.document) == manifest.document_id,
        ),
        (
            "document_version",
            u64::from(consent.document_version) == manifest.document_version,
        ),
        (
            "document_sha256",
            consent.document_sha256 == manifest.document_sha256,
        ),
    ];
    let differing = names(&document_fields);
    if !differing.is_empty() {
        findings.push(Finding::failed(
            "manifest.document_mismatch",
            format!("the consent statement differs in {differing}"),
        ));
    }
    for signer in &manifest.signers {
        let signature =
            listed(manifest, &signer.entries.document_signature).map(|artifact| artifact.sha256);
        let signer_fields = [
            (
                "document_signature",
                signature == Some(consent.signature_sha256),
            ),
            (
                "deployment_id",
                uuid_text(consent.deployment) == manifest.deployment_id,
            ),
            (
                "account_id",
                uuid_text(consent.account) == signer.account_id,
            ),
            (
                "binding_id",
                uuid_text(consent.binding) == signer.binding_id,
            ),
        ];
        let differing = names(&signer_fields);
        if !differing.is_empty() {
            findings.push(Finding::failed(
                "manifest.signer_mismatch",
                format!(
                    "{}: the consent statement differs in {differing}",
                    signer.id
                ),
            ));
        }
    }
    findings
}

fn names(fields: &[(&str, bool)]) -> String {
    fields
        .iter()
        .filter(|(_, holds)| !holds)
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Each member of a signer's `entries` must name a listed artifact with the
/// matching role.
fn signer_entries(manifest: &Manifest) -> Vec<Finding> {
    let mut findings = Vec::new();
    for signer in &manifest.signers {
        for (key, role, entry) in signer.entries.members() {
            let holds = listed(manifest, entry).is_some_and(|artifact| artifact.role == role);
            if !holds {
                findings.push(Finding::failed(
                    "manifest.signer_mismatch",
                    format!(
                        "{}: entries {key} does not name the {} artifact",
                        signer.id,
                        role.as_str()
                    ),
                ));
            }
        }
    }
    findings
}
