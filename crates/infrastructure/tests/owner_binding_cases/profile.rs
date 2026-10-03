use der::{
    asn1::{Any, ObjectIdentifier, OctetString},
    oid::AssociatedOid,
    Decode,
};
use domain::crypto::{CredentialFailure, InternalDeclarationVerifier, Signature};
use infrastructure::certificates::{
    InternalRsaDeclarationVerifier, InternalRsaOwnerBindingVerifier, OwnerBindingFailure,
};
use rsa::{pkcs8::EncodePublicKey, BigUint, RsaPublicKey};
use x509_cert::{
    ext::{
        pkix::{BasicConstraints, ExtendedKeyUsage, KeyUsage, KeyUsages, SubjectKeyIdentifier},
        Extension,
    },
    Certificate,
};

use crate::{
    declaration_fixture::{fixture as declaration, signed_certificate},
    owner_binding_fixture::{change_extension, fixture, partner, verify},
};

const CLIENT_AUTH: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.2");
const EMAIL: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.4");
const SERVER_AUTH: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.1");

fn rejected(certificate: Certificate) {
    assert_eq!(
        InternalRsaOwnerBindingVerifier::new()
            .inspect_certificate(&signed_certificate(certificate))
            .unwrap_err(),
        CredentialFailure::UnsupportedCertificate
    );
}

#[test]
fn partner_purpose_requires_exact_eku_and_key_usage_without_weakening_declarations() {
    let fx = fixture();
    let verifier = InternalRsaOwnerBindingVerifier::new();
    assert_eq!(
        verifier
            .inspect_certificate(&declaration().leaf)
            .unwrap_err(),
        CredentialFailure::UnsupportedCertificate
    );
    assert_eq!(
        InternalRsaDeclarationVerifier
            .inspect_certificate(&fx.leaf)
            .unwrap_err(),
        CredentialFailure::UnsupportedCertificate
    );
    for usages in [
        vec![],
        vec![CLIENT_AUTH],
        vec![EMAIL],
        vec![CLIENT_AUTH, EMAIL, SERVER_AUTH],
        vec![CLIENT_AUTH, EMAIL, CLIENT_AUTH],
        vec![
            CLIENT_AUTH,
            EMAIL,
            ObjectIdentifier::new_unwrap("2.5.29.37.0"),
        ],
    ] {
        let mut cert = partner();
        change_extension(&mut cert, ExtendedKeyUsage(usages));
        rejected(cert);
    }
    let mut reordered = partner();
    change_extension(&mut reordered, ExtendedKeyUsage(vec![EMAIL, CLIENT_AUTH]));
    assert!(verifier
        .inspect_certificate(&signed_certificate(reordered))
        .is_ok());
    let mut critical = partner();
    critical
        .tbs_certificate
        .extensions
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|ext| ext.extn_id == ExtendedKeyUsage::OID)
        .unwrap()
        .critical = true;
    rejected(critical);
    for usages in [
        KeyUsages::DigitalSignature.into(),
        KeyUsages::NonRepudiation.into(),
        KeyUsages::DigitalSignature | KeyUsages::NonRepudiation | KeyUsages::KeyEncipherment,
        KeyUsages::KeyCertSign | KeyUsages::CRLSign,
    ] {
        let mut cert = partner();
        change_extension(&mut cert, KeyUsage(usages));
        rejected(cert);
    }
    for constraints in [
        BasicConstraints {
            ca: true,
            path_len_constraint: None,
        },
        BasicConstraints {
            ca: false,
            path_len_constraint: Some(0),
        },
    ] {
        let mut cert = partner();
        change_extension(&mut cert, constraints);
        rejected(cert);
    }
}

#[test]
fn rsa_algorithm_parameters_and_extension_inventory_are_exact() {
    for (bits, exponent) in [
        (2048_usize, 65537_u32),
        (3071, 65537),
        (3073, 65537),
        (4096, 65537),
        (3072, 3),
    ] {
        let modulus = (BigUint::from(1_u8) << (bits - 1)) + BigUint::from(1_u8);
        let key = RsaPublicKey::new(modulus, BigUint::from(exponent)).unwrap();
        let public = key.to_public_key_der().unwrap();
        let mut cert = partner();
        cert.tbs_certificate.subject_public_key_info =
            x509_cert::spki::SubjectPublicKeyInfoOwned::from_der(public.as_bytes()).unwrap();
        let key_bits = cert
            .tbs_certificate
            .subject_public_key_info
            .subject_public_key
            .as_bytes()
            .unwrap();
        let ski = ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, key_bits);
        change_extension(
            &mut cert,
            SubjectKeyIdentifier(OctetString::new(ski.as_ref()).unwrap()),
        );
        rejected(cert);
    }
    let mut cert = partner();
    cert.tbs_certificate.signature.parameters = None;
    cert.signature_algorithm.parameters = None;
    rejected(cert);
    let mut cert = partner();
    cert.tbs_certificate.signature.oid = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.13");
    cert.signature_algorithm = cert.tbs_certificate.signature.clone();
    rejected(cert);
    let mut cert = partner();
    cert.tbs_certificate
        .subject_public_key_info
        .algorithm
        .parameters = None;
    rejected(cert);
    let mut cert = partner();
    cert.tbs_certificate
        .subject_public_key_info
        .algorithm
        .parameters = Some(Any::from_der(&[2, 1, 0]).unwrap());
    rejected(cert);
    let mut cert = partner();
    cert.tbs_certificate.subject_public_key_info.algorithm.oid =
        ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.10");
    rejected(cert);
    let extensions = partner().tbs_certificate.extensions.unwrap();
    for original in extensions {
        let mut missing = partner();
        missing
            .tbs_certificate
            .extensions
            .as_mut()
            .unwrap()
            .retain(|ext| ext.extn_id != original.extn_id);
        rejected(missing);
        let mut duplicate = partner();
        duplicate
            .tbs_certificate
            .extensions
            .as_mut()
            .unwrap()
            .push(original.clone());
        rejected(duplicate);
        let mut critical = partner();
        critical
            .tbs_certificate
            .extensions
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|ext| ext.extn_id == original.extn_id)
            .unwrap()
            .critical = !original.critical;
        rejected(critical);
    }
    for critical in [false, true] {
        let mut extra = partner();
        extra
            .tbs_certificate
            .extensions
            .as_mut()
            .unwrap()
            .push(Extension {
                extn_id: ObjectIdentifier::new_unwrap("1.2.3.4"),
                critical,
                extn_value: OctetString::new([5, 0]).unwrap(),
            });
        rejected(extra);
    }
}

#[test]
fn raw_limits_single_material_encoding_and_signature_length_are_enforced() {
    let fx = fixture();
    let verifier = InternalRsaOwnerBindingVerifier::new();
    let mut padded = fx.leaf.clone();
    padded.resize(16 * 1024, b' ');
    assert_eq!(
        verifier.inspect_certificate(&padded).unwrap().der,
        fx.leaf_der
    );
    padded.push(b' ');
    assert_eq!(
        verifier.inspect_certificate(&padded).unwrap_err(),
        CredentialFailure::LimitExceeded
    );
    for bad in [
        [fx.leaf.as_slice(), &fx.leaf].concat(),
        [fx.leaf.as_slice(), b"trailing"].concat(),
        [fx.leaf_der.as_slice(), &[0]].concat(),
        b"-----BEGIN PRIVATE KEY-----\nYQ==\n-----END PRIVATE KEY-----\n".to_vec(),
    ] {
        assert_eq!(
            verifier.inspect_certificate(&bad).unwrap_err(),
            CredentialFailure::MalformedCertificate
        );
    }
    for (root, limit) in [(true, 16 * 1024), (false, 1024 * 1024)] {
        let mut trust = fx.trust.clone();
        if root {
            trust.inspection.root_der = vec![0; limit + 1];
        } else {
            trust.inspection.crl_der = vec![0; limit + 1];
        }
        assert_eq!(
            verify(&fx.statement, &fx.leaf, &fx.signature, &trust, fx.at).unwrap_err(),
            OwnerBindingFailure::Credential(CredentialFailure::LimitExceeded)
        );
    }
    for length in [1, 383, 385, 16 * 1024] {
        let signature = Signature::from_bytes(vec![1; length]).unwrap();
        let error = verify(&fx.statement, &fx.leaf, &signature, &fx.trust, fx.at).unwrap_err();
        assert_eq!(
            error,
            OwnerBindingFailure::Credential(CredentialFailure::InvalidSignature)
        );
        assert!(!format!("{error:?} {error}").contains("Synthetic Owner Partner"));
    }
}
