//! Trust anchors: pinned fingerprints per purpose, the served trust route
//! and the time-stamping issuer CRL.

use evidence_verify::policy::{Purposes, ServedRoot, ServedTrust};
use evidence_verify::report::ComponentKind::{Certificates, DocumentToken, Revocation};
use evidence_verify::{validate, Outcome};

use crate::support::{assert_complete, assert_component, facts, fixture, pin, policy, Package};

fn seal() -> Package {
    Package::server_seal(fixture!("token-ess-v1.tsr"))
}

fn only(server_seal: bool, individual_signing: bool, time_stamping: bool) -> Purposes {
    Purposes {
        server_seal,
        individual_signing,
        time_stamping,
    }
}

#[test]
fn a_root_inside_the_zip_that_is_not_pinned_for_the_seal_is_not_supported() {
    let mut unpinned = policy();
    unpinned.pinned = vec![pin(facts().root_sha256, only(false, true, true), "root")];
    let report = validate(&seal().zip(), &unpinned);
    assert_eq!(report.overall, Outcome::NotSupported);
    assert_component(
        &report,
        Certificates,
        Outcome::NotSupported,
        "trust.root_not_pinned",
    );
}

#[test]
fn a_root_pinned_for_another_purpose_does_not_anchor_tokens() {
    let mut unpinned = policy();
    unpinned.pinned = vec![pin(facts().root_sha256, only(true, true, false), "root")];
    let report = validate(&seal().zip(), &unpinned);
    assert_eq!(report.overall, Outcome::NotSupported);
    assert_component(
        &report,
        DocumentToken,
        Outcome::NotSupported,
        "token.anchor_not_pinned",
    );
    assert!(report
        .trust
        .iter()
        .any(|note| note.code == "trust.token_anchor_not_pinned"));
}

#[test]
fn a_root_served_by_the_route_but_not_pinned_is_indeterminate_trust() {
    let mut served = policy();
    served.pinned = vec![pin(facts().root_sha256, only(false, false, true), "root")];
    served.served = Some(ServedTrust {
        roots: vec![ServedRoot {
            der: fixture!("root.der").to_vec(),
            purposes: Purposes::ALL,
        }],
        crls: vec![],
    });
    let report = validate(&seal().zip(), &served);
    assert_eq!(report.overall, Outcome::Indeterminate);
    assert_component(
        &report,
        Certificates,
        Outcome::Indeterminate,
        "trust.root_served_not_pinned",
    );
}

#[test]
fn a_pinned_root_missing_from_the_route_stays_checked_with_a_note() {
    let mut served = policy();
    served.served = Some(ServedTrust {
        roots: vec![],
        crls: vec![],
    });
    let report = validate(&seal().zip(), &served);
    assert_complete(&report);
    assert!(report
        .trust
        .iter()
        .any(|note| note.code == "trust.pinned_not_served"));
}

#[test]
fn an_unavailable_route_changes_only_the_current_status() {
    let report = validate(&seal().zip(), &policy());
    assert_complete(&report);
    assert!(report
        .current_status
        .iter()
        .any(|finding| finding.code == "current.trust_route_unavailable"));
}

#[test]
fn the_current_status_prefers_a_served_crl() {
    let mut served = policy();
    served.served = Some(ServedTrust {
        roots: vec![ServedRoot {
            der: fixture!("root.der").to_vec(),
            purposes: Purposes::ALL,
        }],
        crls: vec![fixture!("root-empty.crl.der").to_vec()],
    });
    let report = validate(&seal().zip(), &served);
    assert_complete(&report);
    assert!(report.trust.is_empty(), "{:#?}", report.trust);
    assert!(report
        .current_status
        .iter()
        .any(|finding| finding.code == "crl.not_revoked" && finding.detail.contains("served")));
}

#[test]
fn a_time_stamping_issuer_without_a_crl_in_the_package_is_indeterminate() {
    let facts = facts();
    let mut both = policy();
    both.pinned = vec![
        pin(facts.root_sha256, only(true, true, false), "root"),
        pin(
            facts.foreign_root_sha256,
            only(false, false, true),
            "foreign",
        ),
    ];
    let mut package = Package::server_seal(fixture!("token-foreign.tsr"));
    package.set("tsa-chain.pem", fixture!("foreign-root.pem"));
    let report = validate(&package.zip(), &both);
    assert_eq!(report.overall, Outcome::Indeterminate, "{report:#?}");
    assert_component(
        &report,
        Revocation,
        Outcome::Indeterminate,
        "revocation.tsa_crl_unavailable",
    );
    assert_eq!(
        crate::support::component(&report, DocumentToken).outcome,
        Outcome::Checked
    );
}

#[test]
fn a_time_stamping_root_served_by_the_route_but_not_pinned_is_indeterminate_trust() {
    let mut served = policy();
    served.pinned = vec![pin(facts().root_sha256, only(true, true, false), "root")];
    served.served = Some(ServedTrust {
        roots: vec![ServedRoot {
            der: fixture!("root.der").to_vec(),
            purposes: Purposes::ALL,
        }],
        crls: vec![],
    });
    let report = validate(&seal().zip(), &served);
    assert_eq!(report.overall, Outcome::Indeterminate, "{report:#?}");
    assert_component(
        &report,
        DocumentToken,
        Outcome::Indeterminate,
        "trust.root_served_not_pinned",
    );
    assert_component(
        &report,
        DocumentToken,
        Outcome::Indeterminate,
        "token.verified",
    );
    assert_component(&report, Revocation, Outcome::Checked, "crl.not_revoked");
}

#[test]
fn a_pinned_root_inside_the_token_does_not_anchor_it_when_tsa_chain_holds_another() {
    let mut package = seal();
    package.set("tsa-chain.pem", fixture!("foreign-root.pem"));
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::NotSupported, "{report:#?}");
    assert_component(
        &report,
        DocumentToken,
        Outcome::NotSupported,
        "token.anchor_not_pinned",
    );
}

#[test]
fn an_individual_package_anchors_the_token_in_tsa_chain_not_in_ca_pem() {
    let mut package = crate::individual::complete();
    package.set("tsa-chain.pem", fixture!("foreign-root.pem"));
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::NotSupported, "{report:#?}");
    assert_component(
        &report,
        DocumentToken,
        Outcome::NotSupported,
        "token.anchor_not_pinned",
    );
}
