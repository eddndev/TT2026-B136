//! Public inspection binds journal evidence without admitting current credentials.

use std::{fs, sync::OnceLock};

use base64::{engine::general_purpose::STANDARD, Engine};
use der::{asn1::ObjectIdentifier, DecodePem, Encode};
use domain::crypto::{
    certificate::CertificateRequestProfile, CertificateValidator, DocumentHasher,
};
use infrastructure::{
    certificates::{inspect_ca_certificate, inspect_ca_request},
    RingSha256Hasher, X509ChainValidator,
};
use x509_cert::{ext::pkix::ExtendedKeyUsage, request::CertReq};

use crate::declaration_fixture::{fixture, leaf, root, script, signed_certificate, time};
use crate::owner_binding_fixture::{change_extension, fixture as partner_fixture, partner};

fn request() -> Vec<u8> {
    fs::read(fixture().ca.join("csr/synthetic-declarant.csr.pem")).unwrap()
}

fn request_pem(der: &[u8]) -> Vec<u8> {
    let encoded = STANDARD.encode(der);
    let lines = encoded
        .as_bytes()
        .chunks(64)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    format!("-----BEGIN CERTIFICATE REQUEST-----\n{lines}\n-----END CERTIFICATE REQUEST-----\n")
        .into_bytes()
}

fn foreign_root() -> &'static [u8] {
    static ROOT: OnceLock<Vec<u8>> = OnceLock::new();
    ROOT.get_or_init(|| {
        let directory = tempfile::tempdir().unwrap();
        let ca = directory.path().join("foreign-ca");
        script("init-ca.sh", &ca, &[]);
        fs::read(ca.join("ca.crt.pem")).unwrap()
    })
}

#[test]
fn request_preview_identifies_exact_public_bytes_and_der_public_key() {
    let fx = fixture();
    let bytes = request();
    let parsed = CertReq::from_pem(&bytes).unwrap();
    let expected_key = RingSha256Hasher.hash_bytes(&parsed.info.public_key.to_der().unwrap());
    let root_der = root().to_der().unwrap();
    for authority in [&fx.root, &root_der] {
        let inspected = inspect_ca_request(&bytes, authority).unwrap();
        assert_eq!(inspected.csr_sha256, RingSha256Hasher.hash_bytes(&bytes));
        assert_eq!(inspected.spki_sha256, expected_key);
        assert_eq!(
            inspected.root_fingerprint,
            RingSha256Hasher.hash_bytes(&root_der)
        );
        assert_eq!(inspected.subject, parsed.info.subject.to_string());
        assert!(inspected.subject.contains("Synthetic Declarant"));
    }
}

#[test]
fn request_fingerprint_keeps_exact_transport_bytes_while_public_identity_is_stable() {
    let fx = fixture();
    let bytes = request();
    let padded = [b" \n".as_slice(), bytes.as_slice(), b"\n \t"].concat();
    let exact = inspect_ca_request(&bytes, &fx.root).unwrap();
    let reformatted = inspect_ca_request(&padded, &fx.root).unwrap();
    assert_ne!(exact.csr_sha256, reformatted.csr_sha256);
    assert_eq!(reformatted.csr_sha256, RingSha256Hasher.hash_bytes(&padded));
    assert_eq!(exact.spki_sha256, reformatted.spki_sha256);
    assert_eq!(exact.subject, reformatted.subject);
    assert_eq!(exact.root_fingerprint, reformatted.root_fingerprint);
}

#[test]
fn request_preview_reports_selected_root_without_claiming_the_csr_signed_it() {
    let fx = fixture();
    let bytes = request();
    let original = inspect_ca_request(&bytes, &fx.root).unwrap();
    let other = inspect_ca_request(&bytes, foreign_root()).unwrap();
    assert_eq!(original.csr_sha256, other.csr_sha256);
    assert_eq!(original.spki_sha256, other.spki_sha256);
    assert_eq!(original.subject, other.subject);
    assert_ne!(original.root_fingerprint, other.root_fingerprint);
}

#[test]
fn request_inspection_rejects_private_combined_malformed_and_oversized_inputs() {
    let fx = fixture();
    let valid = request();
    let private = fs::read(fx.ca.join("private/synthetic-declarant.key.pem")).unwrap();
    let der = CertReq::from_pem(&valid).unwrap().to_der().unwrap();
    for bytes in [
        Vec::new(),
        b"not a certificate request".to_vec(),
        vec![b' '; 16 * 1024 + 1],
        private.clone(),
        [valid.as_slice(), private.as_slice()].concat(),
        [valid.as_slice(), valid.as_slice()].concat(),
        [valid.as_slice(), b"trailing material"].concat(),
        fx.leaf.clone(),
        der,
    ] {
        assert!(inspect_ca_request(&bytes, &fx.root).is_err());
    }
}

#[test]
fn request_inspection_verifies_possession_before_returning_subject_or_key() {
    let fx = fixture();
    let parsed = CertReq::from_pem(request()).unwrap();
    let mut damaged = parsed.to_der().unwrap();
    *damaged.last_mut().unwrap() ^= 1;
    assert!(inspect_ca_request(&request_pem(&damaged), &fx.root).is_err());
    let mut renamed = parsed;
    let other = fs::read(fx.ca.join("csr/second-declarant.csr.pem")).unwrap();
    renamed.info.subject = CertReq::from_pem(&other).unwrap().info.subject;
    assert!(inspect_ca_request(&request_pem(&renamed.to_der().unwrap()), &fx.root).is_err());
}

#[test]
fn both_inspections_require_one_strict_self_signed_public_root() {
    let fx = fixture();
    let bytes = request();
    let mut bad_signature = root().to_der().unwrap();
    *bad_signature.last_mut().unwrap() ^= 1;
    let private = fs::read(fx.ca.join("private/ca.key.pem")).unwrap();
    for authority in [
        Vec::new(),
        b"not a root".to_vec(),
        vec![0; 16 * 1024 + 1],
        private.clone(),
        [fx.root.as_slice(), private.as_slice()].concat(),
        [fx.root.as_slice(), fx.root.as_slice()].concat(),
        fx.leaf.clone(),
        bad_signature,
    ] {
        assert!(inspect_ca_request(&bytes, &authority).is_err());
        assert!(inspect_ca_certificate(&fx.leaf, &authority).is_err());
    }
}

#[test]
fn certificate_inspection_identifies_both_exact_internal_purposes_and_the_issued_key() {
    let fx = fixture();
    let partner = partner_fixture();
    let root_der = root().to_der().unwrap();
    for (pem, der, purpose, csr_name) in [
        (
            &fx.leaf,
            &fx.leaf_der,
            CertificateRequestProfile::DocumentSigning,
            "synthetic-declarant",
        ),
        (
            &partner.leaf,
            &partner.leaf_der,
            CertificateRequestProfile::Partner,
            "synthetic-owner-partner",
        ),
    ] {
        let csr = fs::read(fx.ca.join(format!("csr/{csr_name}.csr.pem"))).unwrap();
        let preview = inspect_ca_request(&csr, &fx.root).unwrap();
        let summary = X509ChainValidator::new().inspect(pem).unwrap();
        let expected_key = RingSha256Hasher.hash_bytes(
            &CertReq::from_pem(&csr)
                .unwrap()
                .info
                .public_key
                .to_der()
                .unwrap(),
        );
        for certificate in [pem, der] {
            for authority in [&fx.root, &root_der] {
                let inspected = inspect_ca_certificate(certificate, authority).unwrap();
                assert_eq!(inspected.serial, summary.serial_hex);
                assert_eq!(inspected.fingerprint, RingSha256Hasher.hash_bytes(der));
                assert_eq!(inspected.spki_sha256, expected_key);
                assert_eq!(inspected.spki_sha256, preview.spki_sha256);
                assert_eq!(inspected.root_fingerprint, preview.root_fingerprint);
                assert_eq!(inspected.purpose, purpose);
                assert_eq!(inspected.subject, summary.subject);
                assert_eq!(inspected.not_before, summary.not_before_unix);
                assert_eq!(inspected.not_after, summary.not_after_unix);
            }
        }
    }
}

#[test]
fn certificate_inspection_rejects_another_root_and_tampered_issuance() {
    let fx = fixture();
    for pem in [&fx.leaf, &partner_fixture().leaf] {
        assert!(inspect_ca_certificate(pem, foreign_root()).is_err());
        let (_, mut der) = der::pem::decode_vec(pem).unwrap();
        *der.last_mut().unwrap() ^= 1;
        assert!(inspect_ca_certificate(&der, &fx.root).is_err());
    }
    let mut renamed = leaf();
    renamed.tbs_certificate.subject = partner().tbs_certificate.subject;
    assert!(inspect_ca_certificate(&renamed.to_der().unwrap(), &fx.root).is_err());
}

#[test]
fn another_issuance_of_the_same_public_key_has_a_distinct_serial_and_fingerprint() {
    let fx = fixture();
    let original = inspect_ca_certificate(&fx.leaf, &fx.root).unwrap();
    let mut another = leaf();
    another.tbs_certificate.serial_number = 9_999_u32.into();
    let inspected = inspect_ca_certificate(&signed_certificate(another), &fx.root).unwrap();
    assert_ne!(inspected.serial, original.serial);
    assert_ne!(inspected.fingerprint, original.fingerprint);
    assert_eq!(inspected.spki_sha256, original.spki_sha256);
    assert_eq!(inspected.root_fingerprint, original.root_fingerprint);
    assert_eq!(inspected.purpose, original.purpose);
    assert_eq!(inspected.subject, original.subject);
}

#[test]
fn certificate_inspection_rejects_private_combined_malformed_and_oversized_inputs() {
    let fx = fixture();
    let private = fs::read(fx.ca.join("private/synthetic-declarant.key.pem")).unwrap();
    for certificate in [
        Vec::new(),
        b"not a certificate".to_vec(),
        vec![0; 16 * 1024 + 1],
        private.clone(),
        [fx.leaf.as_slice(), private.as_slice()].concat(),
        [fx.leaf.as_slice(), fx.root.as_slice()].concat(),
        [fx.leaf.as_slice(), fx.leaf.as_slice()].concat(),
        [fx.leaf.as_slice(), b"trailing material"].concat(),
        request(),
        fx.root.clone(),
    ] {
        assert!(inspect_ca_certificate(&certificate, &fx.root).is_err());
    }
}

#[test]
fn certificate_inspection_enforces_exact_profiles_even_with_valid_authority_signatures() {
    let fx = fixture();
    let mut unsupported_eku = partner();
    change_extension(
        &mut unsupported_eku,
        ExtendedKeyUsage(vec![ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.2")]),
    );
    assert!(inspect_ca_certificate(&signed_certificate(unsupported_eku), &fx.root).is_err());
    let mut missing_constraint = leaf();
    missing_constraint
        .tbs_certificate
        .extensions
        .as_mut()
        .unwrap()
        .remove(0);
    assert!(inspect_ca_certificate(&signed_certificate(missing_constraint), &fx.root).is_err());
    let mut extra_extension = leaf();
    let extension = extra_extension.tbs_certificate.extensions.as_ref().unwrap()[0].clone();
    extra_extension
        .tbs_certificate
        .extensions
        .as_mut()
        .unwrap()
        .push(extension);
    assert!(inspect_ca_certificate(&signed_certificate(extra_extension), &fx.root).is_err());
}

#[test]
fn certificate_inspection_preserves_historical_dates_without_wall_clock_admission() {
    let fx = fixture();
    let mut historical = leaf();
    historical.tbs_certificate.validity.not_before = time(946_684_800);
    historical.tbs_certificate.validity.not_after = time(978_307_200);
    let inspected = inspect_ca_certificate(&signed_certificate(historical), &fx.root).unwrap();
    assert_eq!(inspected.not_before, 946_684_800);
    assert_eq!(inspected.not_after, 978_307_200);
    assert_eq!(
        inspected.purpose,
        CertificateRequestProfile::DocumentSigning
    );
}
