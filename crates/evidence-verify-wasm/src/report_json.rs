//! Encoding of a validation report as the JSON the Web Worker reads.
//!
//! Every enumeration is written as a lowercase, hyphenated name; findings
//! keep the engine's stable codes so that the interface translates them.

use evidence_verify::report::{
    Component, ComponentKind, Instant, InstantSource, Profile, Report, TrustNote,
};
use evidence_verify::{Finding, Outcome};
use serde_json::{json, Value};

/// The report as a JSON object without the engine identification, which
/// [`crate::validate_json`] adds.
pub fn report_value(report: &Report) -> Value {
    json!({
        "profile": report.profile.map(profile),
        "overall": outcome(report.overall),
        "components": report.components.iter().map(component).collect::<Vec<_>>(),
        "instants": report.instants.iter().map(instant).collect::<Vec<_>>(),
        "digests": report
            .digests
            .iter()
            .map(|(name, sha256)| json!({"name": name, "sha256": sha256}))
            .collect::<Vec<_>>(),
        "trust": report.trust.iter().map(trust_note).collect::<Vec<_>>(),
        "current_status": report.current_status.iter().map(finding).collect::<Vec<_>>(),
    })
}

fn profile(profile: Profile) -> &'static str {
    match profile {
        Profile::ServerSealV1 => "server-seal-v1",
        Profile::IndividualSigningV1 => "individual-signing-v1",
        Profile::IndividualSigningV2 => "individual-signing-v2",
    }
}

fn outcome(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Checked => "checked",
        Outcome::Failed => "failed",
        Outcome::Indeterminate => "indeterminate",
        Outcome::NotSupported => "not-supported",
        Outcome::NotApplicable => "not-applicable",
    }
}

fn kind(kind: ComponentKind) -> &'static str {
    match kind {
        ComponentKind::Package => "package",
        ComponentKind::DocumentToken => "document-token",
        ComponentKind::DocumentSignature => "document-signature",
        ComponentKind::BindingConsent => "binding-consent",
        ComponentKind::Certificates => "certificates",
        ComponentKind::Revocation => "revocation",
        ComponentKind::Manifest => "manifest",
        ComponentKind::Context => "context",
    }
}

fn source(source: InstantSource) -> &'static str {
    match source {
        InstantSource::Token => "token",
        InstantSource::ApplicationRecord => "application-record",
        InstantSource::Caller => "caller",
    }
}

fn finding(finding: &Finding) -> Value {
    json!({
        "outcome": outcome(finding.outcome),
        "code": finding.code,
        "detail": finding.detail,
    })
}

fn component(component: &Component) -> Value {
    json!({
        "kind": kind(component.kind),
        "outcome": outcome(component.outcome),
        "findings": component.findings.iter().map(finding).collect::<Vec<_>>(),
    })
}

fn instant(instant: &Instant) -> Value {
    json!({"label": instant.label, "unix": instant.unix, "source": source(instant.source)})
}

fn trust_note(note: &TrustNote) -> Value {
    json!({
        "code": note.code,
        "purpose": note.purpose.map(|purpose| purpose.name()),
        "fingerprint": note.fingerprint,
        "detail": note.detail,
    })
}
