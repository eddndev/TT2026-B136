//! Inventory recognizes the existing TSA profile without granting credential use.

use std::{fs, path::Path, process::Command, sync::OnceLock};

use der::{oid::AssociatedOid, DecodePem, Encode};
use domain::crypto::{
    certificate::{CertificateRequestProfile, CertificateRequestProfileValidator},
    CertificateValidator, CredentialFailure, DocumentHasher, DocumentSigner,
};
use infrastructure::{
    certificates::{
        inspect_ca_certificate, inspect_ca_inventory_certificate, CaInventoryPurpose,
        InternalRsaCertificateRequestProfileValidator, InternalRsaDocumentSigningVerifier,
        InternalRsaOwnerBindingVerifier,
    },
    RingSha256Hasher, RsaPkcs1Signer, X509ChainValidator,
};
use x509_cert::{
    ext::pkix::{BasicConstraints, ExtendedKeyUsage, KeyUsage, KeyUsages},
    Certificate,
};
use zeroize::Zeroizing;

use crate::{
    declaration_fixture::{fixture as declaration, script, time},
    owner_binding_fixture::{change_extension, fixture as partner},
};

struct TsaFixture {
    directory: tempfile::TempDir,
    root: Vec<u8>,
    leaf: Vec<u8>,
}

fn fixture() -> &'static TsaFixture {
    static FIXTURE: OnceLock<TsaFixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let directory = tempfile::tempdir().unwrap();
        let ca = directory.path().join("ca");
        let tsa = directory.path().join("tsa");
        script("init-ca.sh", &ca, &[]);
        let result = Command::new("bash")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pki/issue-tsa-cert.sh"))
            .env("PKI_CA_DIR", &ca)
            .env("TSA_DIR", &tsa)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "the isolated TSA fixture could not be issued"
        );
        TsaFixture {
            root: fs::read(ca.join("ca.crt.pem")).unwrap(),
            leaf: fs::read(tsa.join("tsa.crt.pem")).unwrap(),
            directory,
        }
    })
}

fn leaf() -> Certificate {
    Certificate::from_pem(&fixture().leaf).unwrap()
}

fn signed(mut certificate: Certificate) -> Vec<u8> {
    let key =
        Zeroizing::new(fs::read(fixture().directory.path().join("ca/private/ca.key.pem")).unwrap());
    let digest = RingSha256Hasher.hash_bytes(&certificate.tbs_certificate.to_der().unwrap());
    let signature = RsaPkcs1Signer::new(key).unwrap().sign(&digest).unwrap();
    certificate.signature = der::asn1::BitString::from_bytes(signature.as_bytes()).unwrap();
    certificate.to_der().unwrap()
}

#[test]
fn inventory_reads_the_real_tsa_issuance_stored_in_the_same_ca_index() {
    let fx = fixture();
    let parsed = leaf();
    let der = parsed.to_der().unwrap();
    let root_der = Certificate::from_pem(&fx.root).unwrap().to_der().unwrap();
    let summary = X509ChainValidator::new().inspect(&fx.leaf).unwrap();
    let key = parsed
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .unwrap();
    for certificate in [&fx.leaf, &der] {
        for root in [&fx.root, &root_der] {
            let found = inspect_ca_inventory_certificate(certificate, root).unwrap();
            assert_eq!(found.purpose, CaInventoryPurpose::TimeStamping);
            assert_eq!(found.serial, summary.serial_hex);
            assert_eq!(found.subject, summary.subject);
            assert_eq!(found.fingerprint, RingSha256Hasher.hash_bytes(&der));
            assert_eq!(found.spki_sha256, RingSha256Hasher.hash_bytes(&key));
            assert_eq!(
                found.root_fingerprint,
                RingSha256Hasher.hash_bytes(&root_der)
            );
            assert_eq!(found.not_before, summary.not_before_unix);
            assert_eq!(found.not_after, summary.not_after_unix);
        }
    }
    let ca = fx.directory.path().join("ca");
    let index = fs::read_to_string(ca.join("index.txt")).unwrap();
    assert!(index
        .lines()
        .any(|line| line.split('\t').nth(3) == Some(summary.serial_hex.as_str())));
    let archived = fs::read(ca.join(format!("newcerts/{}.pem", summary.serial_hex))).unwrap();
    assert_eq!(
        inspect_ca_inventory_certificate(&archived, &fx.root)
            .unwrap()
            .fingerprint,
        RingSha256Hasher.hash_bytes(&der)
    );
}

#[test]
fn inventory_preserves_partner_and_document_signing_metadata() {
    let base = declaration();
    for (certificate, purpose) in [
        (&base.leaf, CaInventoryPurpose::DocumentSigning),
        (&partner().leaf, CaInventoryPurpose::Partner),
    ] {
        let strict = inspect_ca_certificate(certificate, &base.root).unwrap();
        let found = inspect_ca_inventory_certificate(certificate, &base.root).unwrap();
        assert_eq!(found.purpose, purpose);
        assert_eq!(found.serial, strict.serial);
        assert_eq!(found.fingerprint, strict.fingerprint);
        assert_eq!(found.spki_sha256, strict.spki_sha256);
        assert_eq!(found.root_fingerprint, strict.root_fingerprint);
        assert_eq!(found.subject, strict.subject);
        assert_eq!(found.not_before, strict.not_before);
        assert_eq!(found.not_after, strict.not_after);
    }
}

#[test]
fn tsa_inventory_does_not_admit_recovery_owner_or_document_signing_profiles() {
    let fx = fixture();
    assert_eq!(
        inspect_ca_certificate(&fx.leaf, &fx.root).unwrap_err(),
        CredentialFailure::UnsupportedCertificate
    );
    assert_eq!(
        InternalRsaOwnerBindingVerifier::new()
            .inspect_certificate(&fx.leaf)
            .unwrap_err(),
        CredentialFailure::UnsupportedCertificate
    );
    assert_eq!(
        InternalRsaDocumentSigningVerifier::new()
            .inspect_certificate(&fx.leaf)
            .unwrap_err(),
        CredentialFailure::UnsupportedCertificate
    );
    for purpose in [
        CertificateRequestProfile::Partner,
        CertificateRequestProfile::DocumentSigning,
    ] {
        assert_eq!(
            InternalRsaCertificateRequestProfileValidator::new()
                .validate_profile(&fx.leaf, purpose)
                .unwrap_err(),
            CredentialFailure::UnsupportedCertificate
        );
    }
}

#[test]
fn inventory_requires_the_exact_critical_tsa_purpose_and_key_usage() {
    let fx = fixture();
    let mut noncritical = leaf();
    noncritical
        .tbs_certificate
        .extensions
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|extension| extension.extn_id == ExtendedKeyUsage::OID)
        .unwrap()
        .critical = false;
    let mut mixed = leaf();
    change_extension(
        &mut mixed,
        ExtendedKeyUsage(vec![
            der::asn1::ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.8"),
            der::asn1::ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.2"),
        ]),
    );
    let mut signing = leaf();
    change_extension(
        &mut signing,
        KeyUsage(KeyUsages::DigitalSignature | KeyUsages::NonRepudiation),
    );
    let mut authority = leaf();
    change_extension(
        &mut authority,
        BasicConstraints {
            ca: true,
            path_len_constraint: None,
        },
    );
    let mut duplicate = leaf();
    let extensions = duplicate.tbs_certificate.extensions.as_mut().unwrap();
    extensions.push(extensions[0].clone());
    for certificate in [noncritical, mixed, signing, authority, duplicate] {
        assert!(inspect_ca_inventory_certificate(&signed(certificate), &fx.root).is_err());
    }
}

#[test]
fn inventory_rejects_foreign_roots_forged_signatures_and_changed_subjects() {
    let fx = fixture();
    assert!(inspect_ca_inventory_certificate(&fx.leaf, &declaration().root).is_err());
    let mut damaged = leaf().to_der().unwrap();
    *damaged.last_mut().unwrap() ^= 1;
    assert!(inspect_ca_inventory_certificate(&damaged, &fx.root).is_err());
    let mut changed = leaf();
    changed.tbs_certificate.subject = Certificate::from_pem(&declaration().leaf)
        .unwrap()
        .tbs_certificate
        .subject;
    assert!(inspect_ca_inventory_certificate(&changed.to_der().unwrap(), &fx.root).is_err());
    let mut root = Certificate::from_pem(&fx.root).unwrap().to_der().unwrap();
    *root.last_mut().unwrap() ^= 1;
    assert!(inspect_ca_inventory_certificate(&fx.leaf, &root).is_err());
}

#[test]
fn inventory_accepts_only_bounded_single_public_certificates_and_roots() {
    let fx = fixture();
    let private = fs::read(fx.directory.path().join("tsa/private/tsa.key.pem")).unwrap();
    for material in [
        Vec::new(),
        vec![0; 16 * 1024 + 1],
        private.clone(),
        [fx.leaf.as_slice(), private.as_slice()].concat(),
        [fx.leaf.as_slice(), fx.root.as_slice()].concat(),
        fx.root.clone(),
    ] {
        assert!(inspect_ca_inventory_certificate(&material, &fx.root).is_err());
    }
    for root in [
        private,
        fx.leaf.clone(),
        [fx.root.as_slice(), fx.root.as_slice()].concat(),
    ] {
        assert!(inspect_ca_inventory_certificate(&fx.leaf, &root).is_err());
    }
}

#[test]
fn tsa_inventory_returns_historical_dates_without_current_validity_admission() {
    let fx = fixture();
    let mut expired = leaf();
    expired.tbs_certificate.validity.not_before = time(946_684_800);
    expired.tbs_certificate.validity.not_after = time(978_307_200);
    let found = inspect_ca_inventory_certificate(&signed(expired), &fx.root).unwrap();
    assert_eq!(found.purpose, CaInventoryPurpose::TimeStamping);
    assert_eq!(found.not_before, 946_684_800);
    assert_eq!(found.not_after, 978_307_200);
}
