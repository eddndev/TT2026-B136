//! The checks of each package profile and the helpers they share.
//!
//! Each profile module turns an admitted archive with a recognized layout
//! into its components, instants, trust notes and current status. The
//! helpers here decode entries and keep the three kinds of findings apart:
//! a signature over exact bytes, the profile and trust of a certificate, and
//! missing evidence. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

mod current;
mod individual;
mod manifest;
mod revocation;
mod server_seal;
mod token;
mod trust;

use crate::archive::Archive;
use crate::certificate::{self, Certificate};
use crate::crl::{self, Crl};
use crate::pem;
use crate::policy::Policy;
use crate::profile::Layout;
use crate::report::{Component, ComponentKind, Finding, Instant, TrustNote};

pub(crate) use trust::route_notes;

/// The input every profile check reads.
pub(crate) struct Evidence<'a> {
    pub archive: &'a Archive,
    pub layout: &'a Layout,
    pub policy: &'a Policy,
}

impl Evidence<'_> {
    /// The content of `name`, empty when absent; the profile guarantees
    /// that every required entry is present.
    fn entry(&self, name: &str) -> &[u8] {
        self.archive.get(name).unwrap_or_default()
    }
}

/// What a profile check produces besides the package component.
pub(crate) struct Checked {
    pub components: Vec<Component>,
    pub instants: Vec<Instant>,
    pub trust: Vec<TrustNote>,
    pub current: Vec<Finding>,
}

/// The findings of every component after the package, in report order;
/// `None` marks a component the profile does not include.
struct Components {
    document_token: Vec<Finding>,
    document_signature: Vec<Finding>,
    binding_consent: Option<Vec<Finding>>,
    certificates: Vec<Finding>,
    revocation: Vec<Finding>,
    manifest: Option<Vec<Finding>>,
    context: Vec<Finding>,
}

impl Components {
    fn into_vec(self) -> Vec<Component> {
        let optional = |kind, findings: Option<Vec<Finding>>| match findings {
            Some(findings) => Component::new(kind, findings),
            None => Component::not_applicable(kind),
        };
        vec![
            Component::new(ComponentKind::DocumentToken, self.document_token),
            Component::new(ComponentKind::DocumentSignature, self.document_signature),
            optional(ComponentKind::BindingConsent, self.binding_consent),
            Component::new(ComponentKind::Certificates, self.certificates),
            Component::new(ComponentKind::Revocation, self.revocation),
            optional(ComponentKind::Manifest, self.manifest),
            Component::new(ComponentKind::Context, self.context),
        ]
    }
}

/// Run the checks of the recognized profile.
pub(crate) fn run(evidence: &Evidence<'_>) -> Checked {
    use crate::report::Profile;
    match evidence.layout.profile {
        Profile::ServerSealV1 => server_seal::check(evidence),
        Profile::IndividualSigningV1 => individual::check(evidence),
        Profile::IndividualSigningV2 => manifest::check(evidence),
    }
}

/// Prefix the detail of `finding` with the entry it concerns.
fn in_entry(entry: &str, finding: Finding) -> Finding {
    let detail = if finding.detail.is_empty() {
        entry.to_owned()
    } else {
        format!("{entry}: {}", finding.detail)
    };
    Finding { detail, ..finding }
}

fn pem_failure(entry: &str, error: pem::PemError) -> Finding {
    Finding::failed(error.code(), entry)
}

/// The single certificate of the PEM entry `entry`.
fn pem_certificate(entry: &str, bytes: &[u8]) -> Result<Certificate, Finding> {
    let der = pem::decode_one(bytes, "CERTIFICATE").map_err(|error| pem_failure(entry, error))?;
    der_certificate(entry, &der)
}

/// The certificate held as DER in the entry `entry`.
fn der_certificate(entry: &str, der: &[u8]) -> Result<Certificate, Finding> {
    certificate::parse(der).map_err(|finding| in_entry(entry, finding))
}

/// The single CRL of the PEM entry `entry`.
fn pem_crl(entry: &str, bytes: &[u8]) -> Result<Crl, Finding> {
    let der = pem::decode_one(bytes, "X509 CRL").map_err(|error| pem_failure(entry, error))?;
    der_crl(entry, &der)
}

/// The CRL held as DER in the entry `entry`.
fn der_crl(entry: &str, der: &[u8]) -> Result<Crl, Finding> {
    crl::parse(der).map_err(|finding| in_entry(entry, finding))
}

/// The detached signature `signature` over the exact `document` bytes,
/// RSA PKCS#1 v1.5 with SHA-256 under the key of `signer`. Only the
/// cryptographic result is reported here; the signer's profile and trust
/// belong to the certificates component.
fn signature_over(
    code_valid: &'static str,
    code_invalid: &'static str,
    signer: Option<&Certificate>,
    message: &[u8],
    signature: &[u8],
) -> Finding {
    let Some(key) = signer.and_then(|signer| signer.public_key.as_ref()) else {
        return Finding::indeterminate(
            "signature.key_unavailable",
            "the signer certificate does not provide an RSA key",
        );
    };
    if certificate::verify_rsa_sha256(key, message, signature) {
        Finding::checked(code_valid)
    } else {
        Finding::failed(
            code_invalid,
            "the signer key does not verify the signature over these exact bytes",
        )
    }
}

/// The interval `genTime` plus or minus the accuracy of a verified token.
fn token_interval(facts: &crate::timestamp::TokenFacts) -> (i64, i64) {
    (
        facts.gen_time.saturating_sub(facts.accuracy_seconds),
        facts.gen_time.saturating_add(facts.accuracy_seconds),
    )
}
