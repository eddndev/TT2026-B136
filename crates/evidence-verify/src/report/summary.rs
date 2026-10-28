//! The structured result of validating one package.
//!
//! The components and the overall precedence follow the section "Component
//! results and the overall result" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use super::{combine, Finding, Outcome};
use crate::policy::Purpose;

/// The package profile selected by the inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    ServerSealV1,
    IndividualSigningV1,
    IndividualSigningV2,
}

/// One part of the evidence with its own result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    /// Admission of the archive and recognition of its profile.
    Package,
    /// The document digest and its time-stamp token, including the
    /// time-stamping certificate and its anchor.
    DocumentToken,
    /// The detached signature over the document bytes.
    DocumentSignature,
    /// The binding and consent statements of individual signing.
    BindingConsent,
    /// The signer certificate, its root and the trust of that root.
    Certificates,
    /// Revocation of every certificate role, including the time-stamping one.
    Revocation,
    /// The evidence manifest and its token.
    Manifest,
    /// Instants, application records and their consistency.
    Context,
}

impl ComponentKind {
    /// Every component, in the order a report lists them.
    pub const ALL: [ComponentKind; 8] = [
        ComponentKind::Package,
        ComponentKind::DocumentToken,
        ComponentKind::DocumentSignature,
        ComponentKind::BindingConsent,
        ComponentKind::Certificates,
        ComponentKind::Revocation,
        ComponentKind::Manifest,
        ComponentKind::Context,
    ];
}

/// The result of one component and the findings it combines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub kind: ComponentKind,
    /// The findings combined with [`combine`]; not applicable when there is
    /// no finding.
    pub outcome: Outcome,
    pub findings: Vec<Finding>,
}

impl Component {
    pub fn new(kind: ComponentKind, findings: Vec<Finding>) -> Self {
        Self {
            kind,
            outcome: combine(findings.iter().map(|finding| finding.outcome)),
            findings,
        }
    }

    /// A component the profile does not include.
    pub fn not_applicable(kind: ComponentKind) -> Self {
        Self::new(kind, Vec::new())
    }
}

/// Where an instant comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstantSource {
    /// The `genTime` of a verified time-stamp token.
    Token,
    /// A record of the application that no time-stamp covers.
    ApplicationRecord,
    /// The caller's instant, read from the client clock.
    Caller,
}

/// One instant the validation used, in Unix seconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instant {
    pub label: String,
    pub unix: i64,
    pub source: InstantSource,
}

/// A difference between the pinned set and the served trust, or another
/// fact about trust anchors that does not change the historical result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustNote {
    pub code: &'static str,
    pub purpose: Option<Purpose>,
    /// Lowercase hexadecimal SHA-256 of the root concerned, or empty.
    pub fingerprint: String,
    pub detail: String,
}

/// The result of one validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The recognized profile; `None` when admission or recognition failed.
    pub profile: Option<Profile>,
    /// Failed, not supported, indeterminate (partial) or checked (complete).
    pub overall: Outcome,
    /// Every component in [`ComponentKind::ALL`] order for a recognized
    /// profile; only the package component otherwise.
    pub components: Vec<Component>,
    pub instants: Vec<Instant>,
    /// Each admitted entry name with the lowercase hexadecimal SHA-256 of
    /// its content, in archive order.
    pub digests: Vec<(String, String)>,
    pub trust: Vec<TrustNote>,
    /// Validity and revocation at the caller's instant. Informational: it
    /// never changes `overall`.
    pub current_status: Vec<Finding>,
}

impl Report {
    /// A report whose components were all evaluated; the overall result
    /// combines them with the contract precedence.
    pub fn from_components(profile: Option<Profile>, components: Vec<Component>) -> Self {
        let overall = combine(components.iter().map(|component| component.outcome));
        Self {
            profile,
            overall,
            components,
            instants: Vec::new(),
            digests: Vec::new(),
            trust: Vec::new(),
            current_status: Vec::new(),
        }
    }

    pub fn component(&self, kind: ComponentKind) -> Option<&Component> {
        self.components
            .iter()
            .find(|component| component.kind == kind)
    }

    /// The findings whose outcome equals the overall result, which state
    /// why the package has that result.
    pub fn reasons(&self) -> Vec<&Finding> {
        self.components
            .iter()
            .flat_map(|component| component.findings.iter())
            .filter(|finding| finding.outcome == self.overall)
            .collect()
    }
}

/// Lowercase hexadecimal of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}
