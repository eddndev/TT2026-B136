//! The authority checks against the verifier fixtures of
//! `crates/evidence-verify/tests/fixtures`.

use super::{check_authority, facts_of};
use application::{document_signing::TsaAuthority, ApplicationError};

macro_rules! fixture {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../evidence-verify/tests/fixtures/",
            $name
        ))
        .as_slice()
    };
}

fn document_digest() -> [u8; 32] {
    evidence_verify::certificate::sha256(fixture!("document.bin"))
}

fn token_facts() -> evidence_verify::timestamp::TokenFacts {
    evidence_verify::timestamp::verify(
        fixture!("token-ess-v2.tsr"),
        &document_digest(),
        &[evidence_verify::policy::DEFAULT_ADMITTED_POLICY],
    )
    .unwrap()
}

fn authority<'a>(root: &'a [u8], crl: &'a [u8]) -> TsaAuthority<'a> {
    TsaAuthority {
        root_der: root,
        crl_der: crl,
    }
}

#[test]
fn a_root_issued_unrevoked_signer_under_a_covering_crl_is_admitted() {
    let facts = token_facts();
    check_authority(
        &facts.signer_certificate_der,
        facts.gen_time,
        facts.accuracy_seconds,
        &authority(fixture!("root.der"), fixture!("root.crl.der")),
    )
    .unwrap();
}

#[test]
fn foreign_stale_revoking_or_oversized_crls_and_foreign_signers_are_configuration_errors() {
    let facts = token_facts();
    let oversized = vec![0_u8; evidence_verify::crl::CRL_LIMIT + 1];
    let mut forged = fixture!("root.crl.der").to_vec();
    *forged.last_mut().unwrap() ^= 1;
    let cases: [(&[u8], &[u8], &[u8]); 7] = [
        (&facts.signer_certificate_der, fixture!("root.der"), &forged),
        (
            &facts.signer_certificate_der,
            fixture!("root.der"),
            fixture!("foreign.crl.der"),
        ),
        (
            &facts.signer_certificate_der,
            fixture!("root.der"),
            fixture!("root-stale.crl.der"),
        ),
        (
            &facts.signer_certificate_der,
            fixture!("root.der"),
            fixture!("root-tsa-revoked.crl.der"),
        ),
        (
            &facts.signer_certificate_der,
            fixture!("root.der"),
            &oversized,
        ),
        (
            fixture!("foreign-tsa.der"),
            fixture!("root.der"),
            fixture!("root.crl.der"),
        ),
        (
            &facts.signer_certificate_der,
            fixture!("foreign-root.der"),
            fixture!("foreign.crl.der"),
        ),
    ];
    for (signer, root, crl) in cases {
        assert!(matches!(
            check_authority(
                signer,
                facts.gen_time,
                facts.accuracy_seconds,
                &authority(root, crl)
            ),
            Err(ApplicationError::InvalidConfiguration(_))
        ));
    }
}

#[test]
fn a_signer_outside_its_validity_at_gen_time_is_a_configuration_error() {
    let facts = token_facts();
    assert!(matches!(
        check_authority(
            &facts.signer_certificate_der,
            0,
            facts.accuracy_seconds,
            &authority(fixture!("root.der"), fixture!("root.crl.der")),
        ),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
}

#[test]
fn document_token_facts_come_from_the_verified_token_only() {
    let digest = domain::crypto::Sha256Digest::from_array(document_digest());
    let token = fixture!("token-ess-v2.tsr");
    let facts = facts_of(token, &digest).unwrap();
    assert_eq!(
        (
            facts.token.as_slice(),
            facts.gen_time,
            facts.accuracy_seconds
        ),
        (
            token,
            token_facts().gen_time,
            token_facts().accuracy_seconds
        )
    );
    let other = domain::crypto::Sha256Digest::from_array([7; 32]);
    assert!(matches!(
        facts_of(token, &other),
        Err(ApplicationError::TimestampEvidenceRejected(_))
    ));
}
