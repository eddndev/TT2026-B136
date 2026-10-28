//! Parity of the strict certificate and CRL profiles of this module with the
//! independent implementation in `evidence_verify::certificate` and
//! `evidence_verify::crl`, which the browser validator runs, over the shared
//! fixtures in `crates/evidence-verify/tests/fixtures`. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use evidence_verify::certificate::{self as verifier, Role};
use evidence_verify::crl as verifier_crl;
use serde_json::Value;

use super::profile::certificate;
use super::revocation::inspect;
use super::Profile;

macro_rules! fixture {
    ($name:literal) => {
        (
            $name,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../evidence-verify/tests/fixtures/",
                $name
            ))
            .as_slice(),
        )
    };
}

/// Every DER certificate of the fixtures, valid and deliberately defective.
const CERTIFICATES: [(&str, &[u8]); 11] = [
    fixture!("root.der"),
    fixture!("foreign-root.der"),
    fixture!("server-signer.der"),
    fixture!("holder.der"),
    fixture!("revoked-holder.der"),
    fixture!("tsa.der"),
    fixture!("foreign-tsa.der"),
    fixture!("tsa-noncritical-eku.der"),
    fixture!("signer-ca-true.der"),
    fixture!("signer-rsa2048.der"),
    fixture!("holder-with-eku.der"),
];

const ROOTS: [(&str, &[u8]); 2] = [fixture!("root.der"), fixture!("foreign-root.der")];

const CRLS: [(&str, &[u8]); 3] = [
    fixture!("root.crl.der"),
    fixture!("root-empty.crl.der"),
    fixture!("foreign.crl.der"),
];

/// The server profile of each verifier role.
const ROLES: [(Role, Profile); 4] = [
    (Role::Root, Profile::Root),
    (Role::ServerSealSigner, Profile::Partner),
    (Role::IndividualSigner, Profile::Declaration),
    (Role::TimeStamping, Profile::TimeStamping),
];

fn facts() -> Value {
    serde_json::from_slice(fixture!("fixtures.json").1).expect("fixtures.json")
}

fn instant(facts: &Value, pointer: &str) -> i64 {
    facts
        .pointer(pointer)
        .and_then(Value::as_i64)
        .unwrap_or_else(|| panic!("fixture fact {pointer}"))
}

fn verifier_profile(der: &[u8], role: Role) -> bool {
    verifier::parse(der)
        .and_then(|parsed| verifier::check_profile(&parsed, role))
        .is_ok()
}

/// Inputs derived from `der` that either side may reject differently if
/// their strictness diverged: a changed signature octet and a truncation.
fn variants(der: &[u8]) -> Vec<Vec<u8>> {
    let mut changed = der.to_vec();
    let last = changed.len() - 1;
    changed[last] ^= 0x01;
    vec![der.to_vec(), changed, der[..der.len() - 1].to_vec()]
}

#[test]
fn every_fixture_certificate_gets_the_same_profile_verdict_for_every_role() {
    let mut admitted = 0;
    for (name, der) in CERTIFICATES {
        for input in variants(der) {
            for (role, profile) in ROLES {
                let server = certificate(&input, profile).is_ok();
                assert_eq!(
                    verifier_profile(&input, role),
                    server,
                    "{name} ({} bytes) as {role:?}",
                    input.len()
                );
                admitted += usize::from(server);
            }
        }
    }
    // The seven valid certificates fit exactly their own role; with a
    // changed signature octet only the five non-root ones still do, because
    // the profile checks no issuer signature except the root's own.
    assert_eq!(admitted, 7 + 5);
}

#[test]
fn issuer_links_and_validity_windows_agree() {
    let facts = facts();
    for (name, der) in CERTIFICATES {
        let (Ok(server), Ok(parsed)) = (
            ROLES
                .iter()
                .find_map(|(_, profile)| certificate(der, *profile).ok())
                .ok_or(()),
            verifier::parse(der),
        ) else {
            continue;
        };
        for (root_name, root_der) in ROOTS {
            let server_root = certificate(root_der, Profile::Root).expect("server root");
            let root = verifier::parse(root_der).expect("verifier root");
            if !name.ends_with("root.der") {
                assert_eq!(
                    verifier::check_issued_by(&parsed, &root).is_ok(),
                    server.check_issuer(&server_root).is_ok(),
                    "{name} issued by {root_name}"
                );
            }
        }
        let not_before = instant(&facts, &format!("/certificates/{name}/not_before"));
        let not_after = instant(&facts, &format!("/certificates/{name}/not_after"));
        for at in [not_before - 1, not_before, not_after, not_after + 1] {
            assert_eq!(
                verifier::valid_over(&parsed, at, at),
                server.check_time(at).is_ok(),
                "{name} at {at}"
            );
        }
    }
}

#[test]
fn every_fixture_crl_gets_the_same_verdict_for_every_root_and_instant() {
    let facts = facts();
    let mut admitted = 0;
    for (name, der) in CRLS {
        let this_update = instant(&facts, &format!("/crls/{name}/this_update"));
        let next_update = instant(&facts, &format!("/crls/{name}/next_update"));
        for input in variants(der) {
            for (root_name, root_der) in ROOTS {
                let server_root = certificate(root_der, Profile::Root).expect("server root");
                let root = verifier::parse(root_der).expect("verifier root");
                for at in [
                    this_update - 1,
                    this_update,
                    (this_update + next_update) / 2,
                    next_update,
                    next_update + 1,
                ] {
                    let server = inspect(&input, &server_root, at).is_ok();
                    let ours = verifier_crl::parse(&input).is_ok_and(|crl| {
                        verifier_crl::check_issued_by(&crl, &root).is_ok()
                            && verifier_crl::covers(&crl, at, at)
                    });
                    assert_eq!(ours, server, "{name} under {root_name} at {at}");
                    admitted += usize::from(server);
                }
            }
        }
    }
    // Each of the three lists, unchanged, under its own root at the three
    // instants inside its window.
    assert_eq!(admitted, 3 * 3);
}

#[test]
fn revocation_entries_agree_for_every_certificate() {
    let facts = facts();
    let mut revoked = 0;
    for (name, der) in CRLS {
        let at = instant(&facts, &format!("/crls/{name}/this_update"));
        let issuer = issuer_der(&facts, name);
        let server_root = certificate(issuer, Profile::Root).expect("server root");
        let server = inspect(der, &server_root, at).expect("server CRL");
        let ours = verifier_crl::parse(der).expect("verifier CRL");
        for (certificate_name, certificate_der) in CERTIFICATES {
            let parsed = verifier::parse(certificate_der).expect("verifier certificate");
            let listed = server.revoked(&parsed.decoded.tbs_certificate.serial_number);
            assert_eq!(
                verifier_crl::revocation_of(&ours, &parsed.serial).is_some(),
                listed,
                "{certificate_name} in {name}"
            );
            revoked += usize::from(listed);
        }
    }
    // Only `root.crl.der` lists a serial, the revoked holder's.
    assert_eq!(revoked, 1);
}

/// The issuer root DER of the CRL fixture `name`, from `fixtures.json`.
fn issuer_der(facts: &Value, name: &str) -> &'static [u8] {
    root_der(
        facts
            .pointer(&format!("/crls/{name}/issuer_file"))
            .and_then(Value::as_str)
            .expect("CRL issuer"),
    )
}

/// The DER of the root fixture named `name`.
fn root_der(name: &str) -> &'static [u8] {
    ROOTS
        .iter()
        .find(|(root, _)| *root == name)
        .map(|(_, der)| *der)
        .expect("root fixture")
}

/// Every single-bit change of each valid certificate, checked in its own
/// role and linked to its issuer (a root to itself through its profile).
/// The chain verdicts agree for every change. The profile verdicts differ
/// only where the verifier rejects a key usage extension value that is not
/// in DER, such as a set padding bit, which this module's decoder accepts;
/// the issuer signature then fails on both sides, since no issuer signed
/// those bytes.
#[test]
fn single_bit_changes_get_the_same_chain_verdict() {
    let facts = facts();
    let mut stricter = 0;
    for (name, der) in CERTIFICATES {
        let Some((role, profile)) = ROLES
            .into_iter()
            .find(|(_, profile)| certificate(der, *profile).is_ok())
        else {
            continue;
        };
        let issuer = facts
            .pointer(&format!("/certificates/{name}/issuer_file"))
            .and_then(Value::as_str)
            .map_or(der, root_der);
        let server_root = certificate(issuer, Profile::Root).expect("server root");
        let root = verifier::parse(issuer).expect("verifier root");
        for index in 0..der.len() {
            let mut input = der.to_vec();
            input[index] ^= 0x01;
            let server = certificate(&input, profile);
            let server_chain = server.as_ref().is_ok_and(|parsed| {
                profile == Profile::Root || parsed.check_issuer(&server_root).is_ok()
            });
            let parsed = verifier::parse(&input);
            let ours = parsed
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|parsed| verifier::check_profile(parsed, role));
            let ours_chain = parsed.as_ref().is_ok_and(|parsed| {
                ours.is_ok()
                    && (role == Role::Root || verifier::check_issued_by(parsed, &root).is_ok())
            });
            assert_eq!(ours_chain, server_chain, "{name} with byte {index} changed");
            match ours {
                Ok(()) => assert!(server.is_ok(), "{name} with byte {index} changed"),
                Err(finding) if server.is_ok() => {
                    assert_eq!(
                        finding.code, "cert.profile.key_usage",
                        "{name} byte {index}"
                    );
                    stricter += 1;
                }
                Err(_) => {}
            }
        }
    }
    assert!(stricter > 0);
}

/// Every single-bit change of each fixture CRL, under its own root at the
/// start of its window, gets the same verdict on both sides.
#[test]
fn single_bit_changes_of_a_crl_get_the_same_verdict() {
    let facts = facts();
    for (name, der) in CRLS {
        let at = instant(&facts, &format!("/crls/{name}/this_update"));
        let issuer = issuer_der(&facts, name);
        let server_root = certificate(issuer, Profile::Root).expect("server root");
        let root = verifier::parse(issuer).expect("verifier root");
        for index in 0..der.len() {
            let mut input = der.to_vec();
            input[index] ^= 0x01;
            let ours = verifier_crl::parse(&input).is_ok_and(|crl| {
                verifier_crl::check_issued_by(&crl, &root).is_ok()
                    && verifier_crl::covers(&crl, at, at)
            });
            assert_eq!(
                ours,
                inspect(&input, &server_root, at).is_ok(),
                "{name} with byte {index} changed"
            );
        }
    }
}
