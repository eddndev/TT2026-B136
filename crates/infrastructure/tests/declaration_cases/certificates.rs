use der::asn1::{Any, ObjectIdentifier, OctetString};
use der::oid::AssociatedOid;
use der::{Decode, Encode};
use domain::crypto::{CredentialFailure, DocumentHasher, InternalDeclarationVerifier, Signature};
use infrastructure::certificates::InternalRsaDeclarationVerifier;
use infrastructure::RingSha256Hasher;
use x509_cert::ext::pkix::{
    AuthorityKeyIdentifier, BasicConstraints, KeyUsage, SubjectKeyIdentifier,
};
use x509_cert::{Certificate, Version};

use crate::declaration_fixture::{fixture, leaf, openssl, root, signed_certificate, time};

fn rejected(certificate: Certificate, expected: CredentialFailure) {
    assert_eq!(
        InternalRsaDeclarationVerifier::new()
            .inspect_certificate(&signed_certificate(certificate))
            .unwrap_err(),
        expected
    );
}

fn change_extension<T: AssociatedOid + Encode>(certificate: &mut Certificate, value: T) {
    certificate
        .tbs_certificate
        .extensions
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|ext| ext.extn_id == T::OID)
        .unwrap()
        .extn_value = OctetString::new(value.to_der().unwrap()).unwrap();
}

#[test]
fn certificate_extension_inventory_and_criticality_are_mandatory() {
    for oid in [
        BasicConstraints::OID,
        KeyUsage::OID,
        SubjectKeyIdentifier::OID,
        AuthorityKeyIdentifier::OID,
    ] {
        let mut cert = leaf();
        cert.tbs_certificate
            .extensions
            .as_mut()
            .unwrap()
            .retain(|ext| ext.extn_id != oid);
        rejected(cert, CredentialFailure::UnsupportedCertificate);
        let mut cert = leaf();
        let extensions = cert.tbs_certificate.extensions.as_mut().unwrap();
        extensions.push(
            extensions
                .iter()
                .find(|ext| ext.extn_id == oid)
                .unwrap()
                .clone(),
        );
        rejected(cert, CredentialFailure::UnsupportedCertificate);
        let mut cert = leaf();
        let ext = cert
            .tbs_certificate
            .extensions
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|ext| ext.extn_id == oid)
            .unwrap();
        ext.critical = !ext.critical;
        rejected(cert, CredentialFailure::UnsupportedCertificate);
    }
}

#[test]
fn additional_extensions_are_rejected_even_when_not_critical() {
    for critical in [false, true] {
        for oid in ["2.5.29.37", "1.2.3.4"] {
            let mut cert = leaf();
            cert.tbs_certificate
                .extensions
                .as_mut()
                .unwrap()
                .push(x509_cert::ext::Extension {
                    extn_id: ObjectIdentifier::new(oid).unwrap(),
                    critical,
                    extn_value: OctetString::new([5, 0]).unwrap(),
                });
            rejected(cert, CredentialFailure::UnsupportedCertificate);
        }
    }
}

#[test]
fn leaf_cannot_be_ca_or_use_ca_key_usage() {
    let mut cert = leaf();
    change_extension(
        &mut cert,
        BasicConstraints {
            ca: true,
            path_len_constraint: None,
        },
    );
    rejected(cert, CredentialFailure::UnsupportedCertificate);
    let mut cert = leaf();
    change_extension(
        &mut cert,
        BasicConstraints {
            ca: false,
            path_len_constraint: Some(0),
        },
    );
    rejected(cert, CredentialFailure::UnsupportedCertificate);
    let mut cert = leaf();
    let usage: KeyUsage = root().tbs_certificate.get().unwrap().unwrap().1;
    change_extension(&mut cert, usage);
    rejected(cert, CredentialFailure::UnsupportedCertificate);
}

#[test]
fn key_identifiers_must_match_the_encoded_public_keys_and_issuer() {
    let mut cert = leaf();
    change_extension(
        &mut cert,
        SubjectKeyIdentifier(OctetString::new([0; 20]).unwrap()),
    );
    rejected(cert, CredentialFailure::UnsupportedCertificate);
    let mut cert = leaf();
    change_extension(
        &mut cert,
        AuthorityKeyIdentifier {
            key_identifier: Some(OctetString::new([0; 20]).unwrap()),
            ..Default::default()
        },
    );
    let fx = fixture();
    assert_eq!(
        InternalRsaDeclarationVerifier::new()
            .verify(
                &RingSha256Hasher.hash_bytes(&fx.statement),
                &signed_certificate(cert),
                &fx.signature,
                &fx.root,
                &fx.crl,
                fx.at
            )
            .unwrap_err(),
        CredentialFailure::UntrustedIssuer
    );
}

#[test]
fn algorithm_parameters_versions_and_serials_are_restricted() {
    let mut cert = leaf();
    cert.tbs_certificate.version = Version::V2;
    rejected(cert, CredentialFailure::UnsupportedCertificate);
    let mut cert = leaf();
    cert.tbs_certificate.serial_number = 0_u32.into();
    rejected(cert, CredentialFailure::UnsupportedCertificate);
    let mut cert = leaf();
    cert.tbs_certificate.signature.parameters = None;
    rejected(cert, CredentialFailure::UnsupportedCertificate);
    let mut cert = leaf();
    cert.tbs_certificate.signature.parameters = None;
    cert.signature_algorithm.parameters = None;
    rejected(cert, CredentialFailure::UnsupportedCertificate);
    let mut cert = leaf();
    cert.tbs_certificate
        .subject_public_key_info
        .algorithm
        .parameters = Some(Any::from_der(&[2, 1, 0]).unwrap());
    rejected(cert, CredentialFailure::UnsupportedCertificate);
}

#[test]
fn rsa_modulus_and_exponent_have_exact_bounds() {
    for (bits, exponent) in [(2048, 65537), (4096, 65537), (3072, 3), (3072, 65537)] {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("key.pem");
        openssl(&[
            "genpkey",
            "-quiet",
            "-algorithm",
            "RSA",
            "-pkeyopt",
            &format!("rsa_keygen_bits:{bits}"),
            "-pkeyopt",
            &format!("rsa_keygen_pubexp:{exponent}"),
            "-out",
            key_path.to_str().unwrap(),
        ]);
        let der = openssl(&[
            "pkey",
            "-in",
            key_path.to_str().unwrap(),
            "-pubout",
            "-outform",
            "DER",
        ]);
        let mut cert = leaf();
        cert.tbs_certificate.subject_public_key_info =
            x509_cert::spki::SubjectPublicKeyInfoOwned::from_der(&der).unwrap();
        let key_bits = cert
            .tbs_certificate
            .subject_public_key_info
            .subject_public_key
            .as_bytes()
            .unwrap();
        let identifier = ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, key_bits);
        change_extension(
            &mut cert,
            SubjectKeyIdentifier(OctetString::new(identifier.as_ref()).unwrap()),
        );
        if bits == 3072 && exponent == 65537 {
            assert!(InternalRsaDeclarationVerifier::new()
                .inspect_certificate(&signed_certificate(cert))
                .is_ok());
        } else {
            rejected(cert, CredentialFailure::UnsupportedCertificate);
        }
    }
}

#[test]
fn certificate_window_is_checked_at_the_explicit_time() {
    let fx = fixture();
    for (before, after, expected) in [
        (fx.at + 1, fx.at + 1000, CredentialFailure::NotYetValid),
        (fx.at - 1000, fx.at - 1, CredentialFailure::Expired),
    ] {
        let mut cert = leaf();
        cert.tbs_certificate.validity.not_before = time(before);
        cert.tbs_certificate.validity.not_after = time(after);
        assert_eq!(
            InternalRsaDeclarationVerifier::new()
                .verify(
                    &RingSha256Hasher.hash_bytes(&fx.statement),
                    &signed_certificate(cert),
                    &fx.signature,
                    &fx.root,
                    &fx.crl,
                    fx.at
                )
                .unwrap_err(),
            expected
        );
    }
}

#[test]
fn wrong_signature_size_and_untrusted_root_are_rejected() {
    let fx = fixture();
    for length in [1, 383, 385, 16 * 1024] {
        assert_eq!(
            InternalRsaDeclarationVerifier::new()
                .verify(
                    &RingSha256Hasher.hash_bytes(&fx.statement),
                    &fx.leaf,
                    &Signature::from_bytes(vec![1; length]).unwrap(),
                    &fx.root,
                    &fx.crl,
                    fx.at
                )
                .unwrap_err(),
            CredentialFailure::InvalidSignature
        );
    }
    let mut root = root();
    root.signature = der::asn1::BitString::from_bytes(&[0; 384]).unwrap();
    assert_eq!(
        InternalRsaDeclarationVerifier::new()
            .inspect_trust(&root.to_der().unwrap(), &fx.crl, fx.at)
            .unwrap_err(),
        CredentialFailure::UntrustedIssuer
    );
}

#[test]
fn valid_signature_cannot_be_transferred_to_another_trusted_leaf() {
    let fx = fixture();
    assert_eq!(
        InternalRsaDeclarationVerifier::new()
            .verify(
                &RingSha256Hasher.hash_bytes(&fx.statement),
                &fx.other_leaf,
                &fx.signature,
                &fx.root,
                &fx.crl,
                fx.at
            )
            .unwrap_err(),
        CredentialFailure::InvalidSignature
    );
}

#[test]
fn root_constraints_and_root_validity_are_also_mandatory() {
    let fx = fixture();
    let verifier = InternalRsaDeclarationVerifier::new();
    let mut cert = root();
    change_extension(
        &mut cert,
        BasicConstraints {
            ca: false,
            path_len_constraint: None,
        },
    );
    assert_eq!(
        verifier
            .inspect_trust(&signed_certificate(cert), &fx.crl, fx.at)
            .unwrap_err(),
        CredentialFailure::UnsupportedCertificate
    );
    let mut cert = root();
    let leaf_usage: KeyUsage = leaf().tbs_certificate.get().unwrap().unwrap().1;
    change_extension(&mut cert, leaf_usage);
    assert_eq!(
        verifier
            .inspect_trust(&signed_certificate(cert), &fx.crl, fx.at)
            .unwrap_err(),
        CredentialFailure::UnsupportedCertificate
    );
    for (before, after, expected) in [
        (fx.at + 1, fx.at + 1000, CredentialFailure::NotYetValid),
        (fx.at - 1000, fx.at - 1, CredentialFailure::Expired),
    ] {
        let mut cert = root();
        cert.tbs_certificate.validity.not_before = time(before);
        cert.tbs_certificate.validity.not_after = time(after);
        assert_eq!(
            verifier
                .inspect_trust(&signed_certificate(cert), &fx.crl, fx.at)
                .unwrap_err(),
            expected
        );
    }
}

#[test]
fn leaf_certificate_signature_itself_must_be_trusted() {
    let fx = fixture();
    let mut value = leaf();
    value.signature = der::asn1::BitString::from_bytes(&[0; 384]).unwrap();
    assert_eq!(
        InternalRsaDeclarationVerifier::new()
            .verify(
                &RingSha256Hasher.hash_bytes(&fx.statement),
                &value.to_der().unwrap(),
                &fx.signature,
                &fx.root,
                &fx.crl,
                fx.at
            )
            .unwrap_err(),
        CredentialFailure::UntrustedIssuer
    );
}
