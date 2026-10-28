//! The binding and consent component: the record in
//! `individual-signing.json` against the statement entries, the holder's
//! signatures over the exact statement bytes, and the statements against
//! the package.
//!
//! The consent must name the package's document digest, document signature
//! digest, holder certificate and root, and the binding and consent must
//! share the deployment, account and binding with non-decreasing account
//! counters and trust revision, as
//! `crates/application/src/documents/individual_evidence.rs` requires.
//! Rationale: `docs/adr/0085-in-browser-evidence-validation.md`.

use crate::certificate::{self, Certificate};
use crate::checks::Evidence;
use crate::individual_json::IndividualSigning;
use crate::report::{Finding, Outcome};
use crate::statements::{self, BindingStatement, ConsentStatement};

pub(super) struct Input<'a> {
    pub evidence: &'a Evidence<'a>,
    pub record: &'a Result<IndividualSigning, Finding>,
    pub holder: Option<&'a Certificate>,
    pub root: Option<&'a Certificate>,
    pub document: &'a [u8],
    pub signature: &'a [u8],
}

/// The findings of the component and the decoded statements, when both
/// decoded.
pub(super) fn check(
    input: &Input<'_>,
) -> (Vec<Finding>, Option<(BindingStatement, ConsentStatement)>) {
    let entry = |name| input.evidence.entry(name);
    let mut findings = record_matches(input);
    let binding_bytes = entry("certificate-binding.bin");
    let consent_bytes = entry("document-consent.bin");
    for (valid, invalid, bytes, proof) in [
        (
            "binding.signature_valid",
            "binding.signature_invalid",
            binding_bytes,
            entry("certificate-binding.sig"),
        ),
        (
            "consent.signature_valid",
            "consent.signature_invalid",
            consent_bytes,
            entry("document-consent.sig"),
        ),
    ] {
        findings.push(crate::checks::signature_over(
            valid,
            invalid,
            input.holder,
            bytes,
            proof,
        ));
    }
    let binding = statements::decode_binding(binding_bytes)
        .map_err(|finding| crate::checks::in_entry("certificate-binding.bin", finding));
    let consent = statements::decode_consent(consent_bytes)
        .map_err(|finding| crate::checks::in_entry("document-consent.bin", finding));
    let (binding, consent) = match (binding, consent) {
        (Ok(binding), Ok(consent)) => (binding, consent),
        (binding, consent) => {
            findings.extend(binding.err());
            findings.extend(consent.err());
            return (findings, None);
        }
    };
    findings.extend(package_matches(input, &binding, &consent));
    findings.push(identity_matches(&binding, &consent));
    (findings, Some((binding, consent)))
}

/// Every byte member of the record must equal its entry.
fn record_matches(input: &Input<'_>) -> Vec<Finding> {
    let record = match input.record {
        Ok(record) => record,
        Err(finding) => {
            return vec![crate::checks::in_entry(
                "individual-signing.json",
                finding.clone(),
            )]
        }
    };
    let pairs: [(&str, &[u8], &str); 6] = [
        (
            "binding_statement",
            &record.binding_statement,
            "certificate-binding.bin",
        ),
        (
            "binding_signature",
            &record.binding_signature,
            "certificate-binding.sig",
        ),
        (
            "binding_root_der",
            &record.binding_root_der,
            "binding-root.der",
        ),
        (
            "binding_crl_der",
            &record.binding_crl_der,
            "binding-crl.der",
        ),
        (
            "consent_statement",
            &record.consent_statement,
            "document-consent.bin",
        ),
        (
            "consent_signature",
            &record.consent_signature,
            "document-consent.sig",
        ),
    ];
    let mismatches: Vec<Finding> = pairs
        .iter()
        .filter(|(_, bytes, name)| *bytes != input.evidence.entry(name))
        .map(|(key, _, name)| {
            Finding::failed(
                "individual.json_mismatch",
                format!("individual-signing.json {key} differs from {name}"),
            )
        })
        .collect();
    if mismatches.is_empty() {
        vec![Finding::checked("individual.json_matches_entries")]
    } else {
        mismatches
    }
}

/// The statements must name the objects of this package.
fn package_matches(
    input: &Input<'_>,
    binding: &BindingStatement,
    consent: &ConsentStatement,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut require = |holds: bool, code: &'static str, detail: &str| {
        if !holds {
            findings.push(Finding::failed(code, detail));
        }
    };
    require(
        consent.document_sha256 == certificate::sha256(input.document),
        "consent.document_mismatch",
        "document_sha256 is not the digest of the document entry",
    );
    require(
        consent.signature_sha256 == certificate::sha256(input.signature),
        "consent.signature_mismatch",
        "signature_sha256 is not the digest of the document signature entry",
    );
    if let Some(holder) = input.holder {
        let leaf = certificate::sha256(&holder.der);
        require(
            binding.certificate_sha256 == leaf,
            "binding.certificate_mismatch",
            "certificate_sha256 is not the digest of certificado.pem",
        );
        require(
            consent.certificate_sha256 == leaf,
            "consent.certificate_mismatch",
            "certificate_sha256 is not the digest of certificado.pem",
        );
    }
    if let Some(root) = input.root {
        require(
            consent.root_sha256 == certificate::sha256(&root.der),
            "consent.root_mismatch",
            "root_sha256 is not the digest of ca.pem",
        );
    }
    require(
        binding.root_sha256 == certificate::sha256(input.evidence.entry("binding-root.der")),
        "binding.root_mismatch",
        "root_sha256 is not the digest of binding-root.der",
    );
    if findings.is_empty() {
        findings.push(Finding::checked("consent.matches_package"));
    }
    findings
}

/// The binding and the consent must describe the same holder binding.
fn identity_matches(binding: &BindingStatement, consent: &ConsentStatement) -> Finding {
    let differing = [
        ("deployment", binding.deployment == consent.deployment),
        ("account", binding.account == consent.account),
        ("binding", binding.binding == consent.binding),
        ("root_sha256", binding.root_sha256 == consent.root_sha256),
        (
            "certificate_sha256",
            binding.certificate_sha256 == consent.certificate_sha256,
        ),
        (
            "account_revision",
            binding.account_revision <= consent.account_revision,
        ),
        (
            "authentication_generation",
            binding.authentication_generation <= consent.authentication_generation,
        ),
        (
            "trust_revision",
            binding.trust_revision <= consent.trust_revision,
        ),
    ]
    .iter()
    .filter(|(_, holds)| !holds)
    .map(|(field, _)| *field)
    .collect::<Vec<_>>();
    if differing.is_empty() {
        Finding::new(
            Outcome::Checked,
            "statements.same_binding",
            "the binding and the consent describe the same holder binding",
        )
    } else {
        Finding::failed("statements.binding_mismatch", differing.join(", "))
    }
}
