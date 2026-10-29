//! Registration and acceptance consistency over the fixture statements and
//! CRLs, for trust revisions the fixture packages cannot express because
//! the statements are signed.

use super::{check, Input};
use crate::certificate::{self, Certificate};
use crate::crl::{self, Crl};
use crate::report::{Finding, Outcome};
use crate::statements::{decode_binding, decode_consent, BindingStatement, ConsentStatement};

macro_rules! fixture {
    ($name:literal) => {
        include_bytes!(concat!("../../../../tests/fixtures/", $name)).as_slice()
    };
}

fn statements(
    binding_revision: u32,
    consent_revision: u32,
) -> (BindingStatement, ConsentStatement) {
    let mut binding = decode_binding(fixture!("certificate-binding.bin")).expect("binding");
    let mut consent = decode_consent(fixture!("document-consent.bin")).expect("consent");
    binding.trust_revision = binding_revision;
    consent.trust_revision = consent_revision;
    (binding, consent)
}

fn root() -> Certificate {
    certificate::parse(fixture!("root.der")).expect("root")
}

fn later() -> Crl {
    crl::parse(fixture!("root.crl.der")).expect("crl")
}

fn earlier() -> Crl {
    crl::parse(fixture!("root-empty.crl.der")).expect("crl")
}

fn progression(
    pair: &(BindingStatement, ConsentStatement),
    registration: &Crl,
    acceptance: &Crl,
) -> Finding {
    let root = root();
    let findings = check(&Input {
        record: None,
        token: None,
        statements: Some(pair),
        root: Some(&root),
        binding_root_der: fixture!("root.der"),
        crl: Some(acceptance),
        binding_crl: Some(registration),
        binding_crl_der: &registration.der,
        manifest: false,
    });
    findings
        .into_iter()
        .find(|finding| {
            finding.code.starts_with("context.trust") || finding.code == "context.same_trust"
        })
        .expect("trust finding")
}

#[test]
fn a_later_revision_with_a_larger_crl_number_is_checked() {
    assert!(later().number_value() > earlier().number_value());
    let finding = progression(&statements(1, 2), &earlier(), &later());
    assert_eq!(finding.code, "context.trust_progressed");
    assert_eq!(finding.outcome, Outcome::Checked);
}

#[test]
fn a_later_revision_with_an_older_crl_fails() {
    let finding = progression(&statements(1, 2), &later(), &earlier());
    assert_eq!(finding.code, "context.trust_order");
    assert_eq!(finding.outcome, Outcome::Failed);
}

#[test]
fn a_later_revision_with_the_same_crl_fails() {
    let finding = progression(&statements(1, 2), &later(), &later());
    assert_eq!(finding.code, "context.trust_order");
}

#[test]
fn the_same_revision_requires_the_same_crl() {
    assert_eq!(
        progression(&statements(2, 2), &later(), &later()).code,
        "context.same_trust"
    );
    assert_eq!(
        progression(&statements(2, 2), &earlier(), &later()).code,
        "context.trust_mismatch"
    );
}
