use der::asn1::OctetString;
use der::oid::AssociatedOid;
use der::Encode;
use domain::crypto::{CredentialFailure, DocumentHasher, InternalDeclarationVerifier};
use infrastructure::certificates::InternalRsaDeclarationVerifier;
use infrastructure::RingSha256Hasher;
use x509_cert::crl::{CertificateList, RevokedCert};
use x509_cert::ext::pkix::{AuthorityKeyIdentifier, CrlNumber};

use crate::declaration_fixture::{crl, fixture, leaf, root, signed_certificate, signed_crl, time};

fn rejected(crl: CertificateList, expected: CredentialFailure) {
    let fx = fixture();
    assert_eq!(
        InternalRsaDeclarationVerifier::new()
            .inspect_trust(&fx.root, &signed_crl(crl), fx.at)
            .unwrap_err(),
        expected
    );
}

#[test]
fn crl_time_window_is_inclusive_and_future_or_stale_lists_fail() {
    let fx = fixture();
    let verifier = InternalRsaDeclarationVerifier::new();
    let mut value = crl();
    value.tbs_cert_list.this_update = time(fx.at - 30);
    value.tbs_cert_list.next_update = Some(time(fx.at + 30));
    let bytes = signed_crl(value);
    let result = verifier.inspect_trust(&fx.root, &bytes, fx.at).unwrap();
    for at in [result.valid_from, result.valid_until] {
        assert!(verifier.inspect_trust(&fx.root, &bytes, at).is_ok());
    }
    assert_eq!(
        verifier
            .inspect_trust(&fx.root, &bytes, result.valid_from - 1)
            .unwrap_err(),
        CredentialFailure::CrlNotYetValid
    );
    assert_eq!(
        verifier
            .inspect_trust(&fx.root, &bytes, result.valid_until + 1)
            .unwrap_err(),
        CredentialFailure::CrlExpired
    );
}

#[test]
fn missing_and_empty_crl_are_hard_failures() {
    let fx = fixture();
    assert_eq!(
        InternalRsaDeclarationVerifier::new()
            .inspect_trust(&fx.root, b"", fx.at)
            .unwrap_err(),
        CredentialFailure::MalformedCrl
    );
    let mut value = crl();
    value.tbs_cert_list.next_update = None;
    rejected(value, CredentialFailure::UnsupportedCrl);
    let mut value = crl();
    value.tbs_cert_list.next_update = Some(value.tbs_cert_list.this_update);
    rejected(value, CredentialFailure::UnsupportedCrl);
}

#[test]
fn crl_extension_inventory_numbers_and_signatures_are_checked() {
    for oid in [AuthorityKeyIdentifier::OID, CrlNumber::OID] {
        let mut value = crl();
        value
            .tbs_cert_list
            .crl_extensions
            .as_mut()
            .unwrap()
            .retain(|ext| ext.extn_id != oid);
        rejected(value, CredentialFailure::UnsupportedCrl);
        let mut value = crl();
        let extensions = value.tbs_cert_list.crl_extensions.as_mut().unwrap();
        extensions.push(
            extensions
                .iter()
                .find(|ext| ext.extn_id == oid)
                .unwrap()
                .clone(),
        );
        rejected(value, CredentialFailure::UnsupportedCrl);
    }
    let mut value = crl();
    let ext = value
        .tbs_cert_list
        .crl_extensions
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|ext| ext.extn_id == CrlNumber::OID)
        .unwrap();
    ext.extn_value = OctetString::new([2, 9, 1, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
    rejected(value, CredentialFailure::UnsupportedCrl);
    let mut value = crl();
    value.signature = der::asn1::BitString::from_bytes(&[0; 384]).unwrap();
    let fx = fixture();
    assert_eq!(
        InternalRsaDeclarationVerifier::new()
            .inspect_trust(&fx.root, &value.to_der().unwrap(), fx.at)
            .unwrap_err(),
        CredentialFailure::UntrustedCrl
    );
}

#[test]
fn revoked_certificate_is_rejected_from_the_signed_list() {
    let fx = fixture();
    let mut value = crl();
    value.tbs_cert_list.revoked_certificates = Some(vec![RevokedCert {
        serial_number: leaf().tbs_certificate.serial_number,
        revocation_date: value.tbs_cert_list.this_update,
        crl_entry_extensions: None,
    }]);
    assert_eq!(
        InternalRsaDeclarationVerifier::new()
            .verify(
                &RingSha256Hasher.hash_bytes(&fx.statement),
                &fx.leaf,
                &fx.signature,
                &fx.root,
                &signed_crl(value),
                fx.at
            )
            .unwrap_err(),
        CredentialFailure::Revoked
    );
}

#[test]
fn serial_duplicates_future_revocation_and_entry_extensions_are_rejected() {
    let original = crl();
    let entry = RevokedCert {
        serial_number: 42_u32.into(),
        revocation_date: original.tbs_cert_list.this_update,
        crl_entry_extensions: None,
    };
    let mut value = original.clone();
    value.tbs_cert_list.revoked_certificates = Some(vec![entry.clone(), entry.clone()]);
    rejected(value, CredentialFailure::UnsupportedCrl);
    let mut value = original.clone();
    let mut future = entry.clone();
    future.revocation_date = time(fixture().at + 1);
    value.tbs_cert_list.revoked_certificates = Some(vec![future]);
    rejected(value, CredentialFailure::UnsupportedCrl);
    let mut value = original;
    let mut extended = entry;
    extended.crl_entry_extensions = Some(vec![]);
    value.tbs_cert_list.revoked_certificates = Some(vec![extended]);
    rejected(value, CredentialFailure::UnsupportedCrl);
}

#[test]
fn revoked_entry_count_is_bounded_independently_of_raw_size() {
    let mut value = crl();
    let date = value.tbs_cert_list.this_update;
    value.tbs_cert_list.revoked_certificates = Some(
        (1_u32..=10_001)
            .map(|serial| RevokedCert {
                serial_number: serial.into(),
                revocation_date: date,
                crl_entry_extensions: None,
            })
            .collect(),
    );
    let bytes = signed_crl(value);
    assert!(bytes.len() < 1024 * 1024);
    let fx = fixture();
    assert_eq!(
        InternalRsaDeclarationVerifier::new()
            .inspect_trust(&fx.root, &bytes, fx.at)
            .unwrap_err(),
        CredentialFailure::LimitExceeded
    );
}

#[test]
fn crl_issuer_key_identifier_algorithms_and_extra_extensions_are_rejected() {
    let mut value = crl();
    let ext = value
        .tbs_cert_list
        .crl_extensions
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|ext| ext.extn_id == AuthorityKeyIdentifier::OID)
        .unwrap();
    let aki = AuthorityKeyIdentifier {
        key_identifier: Some(OctetString::new([0; 20]).unwrap()),
        ..Default::default()
    };
    ext.extn_value = OctetString::new(aki.to_der().unwrap()).unwrap();
    rejected(value, CredentialFailure::UntrustedCrl);
    let mut value = crl();
    value.tbs_cert_list.signature.parameters = None;
    rejected(value, CredentialFailure::UnsupportedCrl);
    let mut value = crl();
    value
        .tbs_cert_list
        .crl_extensions
        .as_mut()
        .unwrap()
        .push(x509_cert::ext::Extension {
            extn_id: der::asn1::ObjectIdentifier::new_unwrap("2.5.29.27"),
            critical: true,
            extn_value: OctetString::new([2, 1, 0]).unwrap(),
        });
    rejected(value, CredentialFailure::UnsupportedCrl);
}

#[test]
fn original_crl_dates_are_distinct_from_the_trust_validity_intersection() {
    let fx = fixture();
    let mut root = root();
    root.tbs_certificate.validity.not_before = time(fx.at - 10);
    root.tbs_certificate.validity.not_after = time(fx.at + 10);
    let mut list = crl();
    list.tbs_cert_list.this_update = time(fx.at - 20);
    list.tbs_cert_list.next_update = Some(time(fx.at + 20));
    let inspected = InternalRsaDeclarationVerifier::new()
        .inspect_trust(&signed_certificate(root), &signed_crl(list), fx.at)
        .unwrap();
    assert_eq!(inspected.valid_from, fx.at - 10);
    assert_eq!(inspected.valid_until, fx.at + 10);
    assert_eq!(inspected.crl_this_update, fx.at - 20);
    assert_eq!(inspected.crl_next_update, fx.at + 20);
}
