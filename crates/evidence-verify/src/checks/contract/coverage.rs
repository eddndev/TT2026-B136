//! The contract manifest against the package, the request and the party
//! statements.
//!
//! The covered entries are all package entries except the manifest, its
//! token Tf, `contrato-recibo.json` and the instructions. A listed entry the
//! package lacks, a covered entry the manifest omits and a digest or size
//! that differs each fail; the schema already ties every party and role to
//! its fixed entry. The manifest must repeat the request's identifiers,
//! version, digests and expiry, and each signer the account, binding and
//! operation its party statement names. Rationale: the section "Evidence
//! packages" of `docs/adr/0086-bilateral-contract-signing.md`.

use super::super::Evidence;
use super::differing;
use crate::certificate;
use crate::contract_statements::{ContractRequest, PartyStatement};
use crate::manifest::contract::{ContractManifest, ENTRY, TOKEN_ENTRY};
use crate::manifest::schema::shown;
use crate::profile::INSTRUCTIONS;
use crate::receipt::contract as receipt;
use crate::report::Finding;
use crate::statements::uuid_text;

/// Entries of a full package that the manifest does not cover.
const UNCOVERED: [&str; 4] = [ENTRY, TOKEN_ENTRY, receipt::ENTRY, INSTRUCTIONS];

pub(super) struct Input<'a> {
    pub evidence: &'a Evidence<'a>,
    pub manifest: &'a ContractManifest,
    pub request: Option<&'a ContractRequest>,
    pub request_sha256: [u8; 32],
    pub pdf_sha256: [u8; 32],
    /// The decoded statement of each position, when it decoded.
    pub statements: [Option<&'a PartyStatement>; 2],
}

/// The failures of the manifest; empty when it agrees with everything.
pub(super) fn check(input: &Input<'_>) -> Vec<Finding> {
    let mut findings = artifacts(input);
    findings.extend(request(input));
    findings.extend(signers(input));
    findings
}

fn artifacts(input: &Input<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    let covered: Vec<&str> = input
        .evidence
        .archive
        .names()
        .into_iter()
        .filter(|name| !UNCOVERED.contains(name))
        .collect();
    let listed = |name: &str| {
        input
            .manifest
            .artifacts
            .iter()
            .any(|artifact| artifact.entry == name)
    };
    for name in covered.iter().filter(|name| !listed(name)) {
        findings.push(Finding::failed(
            "manifest.artifact_missing",
            format!("{name} is not listed"),
        ));
    }
    for artifact in &input.manifest.artifacts {
        let entry = artifact.entry.as_str();
        if !covered.contains(&entry) {
            findings.push(Finding::failed(
                "manifest.artifact_unexpected",
                format!("{} is not a covered entry of the package", shown(entry)),
            ));
            continue;
        }
        let bytes = input.evidence.entry(entry);
        let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if artifact.sha256 != certificate::sha256(bytes) || artifact.size != size {
            findings.push(Finding::failed(
                "manifest.artifact_mismatch",
                format!("{entry} differs from its manifest digest or size"),
            ));
        }
    }
    findings
}

fn request(input: &Input<'_>) -> Option<Finding> {
    let manifest = input.manifest;
    let mut fields = vec![
        (
            "request_sha256",
            manifest.request_sha256 == input.request_sha256,
        ),
        (
            "document_sha256",
            manifest.document_sha256 == input.pdf_sha256,
        ),
    ];
    if let Some(request) = input.request {
        fields.extend([
            (
                "deployment_id",
                uuid_text(request.deployment) == manifest.deployment_id,
            ),
            ("case_id", uuid_text(request.case) == manifest.case_id),
            (
                "request_id",
                uuid_text(request.request) == manifest.request_id,
            ),
            (
                "document_id",
                uuid_text(request.document) == manifest.document_id,
            ),
            (
                "document_version",
                u64::from(request.document_version) == manifest.document_version,
            ),
            (
                "request document_sha256",
                request.document_sha256 == manifest.document_sha256,
            ),
            ("expires_at", request.expires_at == manifest.expires_at),
        ]);
    }
    let differing = differing(&fields);
    (!differing.is_empty()).then(|| {
        Finding::failed(
            "manifest.document_mismatch",
            format!("the manifest differs from the request or the package in {differing}"),
        )
    })
}

fn signers(input: &Input<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, (signer, statement)) in input
        .manifest
        .signers
        .iter()
        .zip(input.statements)
        .enumerate()
    {
        let Some(statement) = statement else {
            continue;
        };
        let fields = [
            (
                "account_id",
                uuid_text(statement.account) == signer.account_id,
            ),
            (
                "binding_id",
                uuid_text(statement.binding) == signer.binding_id,
            ),
            (
                "operation_id",
                uuid_text(statement.operation) == signer.operation_id,
            ),
        ];
        let differing = differing(&fields);
        if !differing.is_empty() {
            findings.push(Finding::failed(
                "manifest.signer_mismatch",
                format!(
                    "signer-{}: the party statement differs in {differing}",
                    index + 1
                ),
            ));
        }
    }
    findings
}
