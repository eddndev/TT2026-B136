use der::{
    asn1::{BitString, OctetString},
    Encode,
};
use domain::crypto::{CredentialFailure, DocumentHasher};
use infrastructure::{certificates::OwnerBindingFailure, RingSha256Hasher};
use uuid::Uuid;
use x509_cert::{crl::RevokedCert, ext::pkix::AuthorityKeyIdentifier};

use crate::{
    declaration_fixture::{crl, root, signed_certificate, signed_crl, time},
    owner_binding_fixture::{
        change_extension, fixture, owner, partner, sign, statement, trust_with, verify, verify_leaf,
    },
};

#[test]
fn untrusted_authority_or_crl_and_a_signed_revocation_cannot_admit_a_binding() {
    let fx = fixture();
    let mut leaf = partner();
    leaf.tbs_certificate.issuer = "CN=Another Authority".parse().unwrap();
    assert_eq!(
        verify_leaf(&signed_certificate(leaf), fx.at).unwrap_err(),
        OwnerBindingFailure::Credential(CredentialFailure::UntrustedIssuer)
    );
    let mut leaf = partner();
    change_extension(
        &mut leaf,
        AuthorityKeyIdentifier {
            key_identifier: Some(OctetString::new([0; 20]).unwrap()),
            ..Default::default()
        },
    );
    assert_eq!(
        verify_leaf(&signed_certificate(leaf), fx.at).unwrap_err(),
        OwnerBindingFailure::Credential(CredentialFailure::UntrustedIssuer)
    );
    let mut authority = root();
    authority.signature = BitString::from_bytes(&[0; 384]).unwrap();
    let mut trust = fx.trust.clone();
    trust.inspection.root_der = authority.to_der().unwrap();
    assert_eq!(
        verify(&fx.statement, &fx.leaf, &fx.signature, &trust, fx.at).unwrap_err(),
        OwnerBindingFailure::Credential(CredentialFailure::UntrustedIssuer)
    );
    let mut invalid_signature = crl();
    invalid_signature.signature = BitString::from_bytes(&[0; 384]).unwrap();
    let mut invalid_aki = crl();
    let extensions = invalid_aki.tbs_cert_list.crl_extensions.as_mut().unwrap();
    use der::oid::AssociatedOid;
    extensions
        .iter_mut()
        .find(|ext| ext.extn_id == AuthorityKeyIdentifier::OID)
        .unwrap()
        .extn_value = OctetString::new(
        AuthorityKeyIdentifier {
            key_identifier: Some(OctetString::new([0; 20]).unwrap()),
            ..Default::default()
        }
        .to_der()
        .unwrap(),
    )
    .unwrap();
    for invalid in [invalid_signature.to_der().unwrap(), signed_crl(invalid_aki)] {
        let mut trust = fx.trust.clone();
        trust.inspection.crl_der = invalid;
        assert_eq!(
            verify(&fx.statement, &fx.leaf, &fx.signature, &trust, fx.at).unwrap_err(),
            OwnerBindingFailure::Credential(CredentialFailure::UntrustedCrl)
        );
    }
    let mut revoked = crl();
    revoked.tbs_cert_list.revoked_certificates = Some(vec![RevokedCert {
        serial_number: partner().tbs_certificate.serial_number,
        revocation_date: revoked.tbs_cert_list.this_update,
        crl_entry_extensions: None,
    }]);
    let trust = trust_with(&fx.trust.inspection.root_der, &signed_crl(revoked));
    assert_eq!(
        verify(&fx.statement, &fx.leaf, &fx.signature, &trust, fx.at).unwrap_err(),
        OwnerBindingFailure::Credential(CredentialFailure::Revoked)
    );
}

#[test]
fn leaf_root_and_crl_windows_are_inclusive_and_rechecked_at_explicit_time() {
    let fx = fixture();
    let result = verify(&fx.statement, &fx.leaf, &fx.signature, &fx.trust, fx.at).unwrap();
    for at in [result.valid_from, result.valid_until] {
        assert!(verify(&fx.statement, &fx.leaf, &fx.signature, &fx.trust, at).is_ok());
    }
    for (before, after, expected) in [
        (fx.at + 1, fx.at + 1000, CredentialFailure::NotYetValid),
        (fx.at - 1000, fx.at - 1, CredentialFailure::Expired),
    ] {
        let mut leaf = partner();
        leaf.tbs_certificate.validity.not_before = time(before);
        leaf.tbs_certificate.validity.not_after = time(after);
        assert_eq!(
            verify_leaf(&signed_certificate(leaf), fx.at).unwrap_err(),
            OwnerBindingFailure::Credential(expected)
        );
        let mut authority = root();
        authority.tbs_certificate.validity.not_before = time(before);
        authority.tbs_certificate.validity.not_after = time(after);
        let mut trust = fx.trust.clone();
        trust.inspection.root_der = signed_certificate(authority);
        assert_eq!(
            verify(&fx.statement, &fx.leaf, &fx.signature, &trust, fx.at).unwrap_err(),
            OwnerBindingFailure::Credential(expected)
        );
    }
    for (start, end, expected) in [
        (fx.at + 1, fx.at + 1000, CredentialFailure::CrlNotYetValid),
        (fx.at - 1000, fx.at - 1, CredentialFailure::CrlExpired),
    ] {
        let mut value = crl();
        value.tbs_cert_list.this_update = time(start);
        value.tbs_cert_list.next_update = Some(time(end));
        let mut trust = fx.trust.clone();
        trust.inspection.crl_der = signed_crl(value);
        assert_eq!(
            verify(&fx.statement, &fx.leaf, &fx.signature, &trust, fx.at).unwrap_err(),
            OwnerBindingFailure::Credential(expected)
        );
    }
    let mut leaf = partner();
    leaf.tbs_certificate.validity.not_before = time(fx.at - 30);
    leaf.tbs_certificate.validity.not_after = time(fx.at + 30);
    let leaf = signed_certificate(leaf);
    let statement = statement(
        &fx.trust,
        RingSha256Hasher.hash_bytes(&leaf),
        owner(9, 4),
        Uuid::from_bytes([0x44; 16]),
    );
    let signature = sign(&statement.canonical_bytes());
    let result = verify(&statement, &leaf, &signature, &fx.trust, fx.at).unwrap();
    assert_eq!(
        (result.valid_from, result.valid_until),
        (fx.at - 30, fx.at + 30)
    );
}
