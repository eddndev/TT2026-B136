//! Issuer subject policy is checked before invoking a mutable CA script.

use std::{fs, str::FromStr};

use der::{asn1::BitString, DecodePem, Encode, EncodePem};
use domain::crypto::{
    certificate::{CertificateRequestAuthority, CertificateRequestProfile},
    DocumentHasher, DocumentSigner,
};
use infrastructure::{
    certificates::inspect_ca_request, OpensslCaAdapter, RingSha256Hasher, RsaPkcs1Signer,
};
use x509_cert::{name::Name, request::CertReq};
use zeroize::Zeroizing;

use crate::declaration_fixture::{fixture, root, signed_certificate};

fn request_with_subject(subject: &str) -> Vec<u8> {
    let fx = fixture();
    let original = fs::read(fx.ca.join("csr/synthetic-declarant.csr.pem")).unwrap();
    let mut request = CertReq::from_pem(original).unwrap();
    request.info.subject = Name::from_str(subject).unwrap();
    let key = Zeroizing::new(fs::read(fx.ca.join("private/synthetic-declarant.key.pem")).unwrap());
    let signature = RsaPkcs1Signer::new(key)
        .unwrap()
        .sign(&RingSha256Hasher.hash_bytes(&request.info.to_der().unwrap()))
        .unwrap();
    request.signature = BitString::from_bytes(signature.as_bytes()).unwrap();
    request
        .to_pem(der::pem::LineEnding::LF)
        .unwrap()
        .into_bytes()
}

#[test]
fn issuer_policy_rejects_invalid_subject_before_calling_the_signing_script() {
    for subject in [
        "CN=External Holder",
        "CN=External Holder,O=Despacho Juridico Demo",
        "CN=External Holder,C=MX",
        "CN=External Holder,O=Despacho Juridico Demo,C=US",
        "CN=External Holder,O=Another Office,C=MX",
        "O=Despacho Juridico Demo,C=MX",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let ca = directory.path().join("ca");
        let scripts = directory.path().join("scripts");
        fs::create_dir(&ca).unwrap();
        fs::create_dir(&scripts).unwrap();
        fs::write(ca.join("ca.crt.pem"), &fixture().root).unwrap();
        fs::write(ca.join("serial"), b"1002\n").unwrap();
        fs::write(ca.join("index.txt"), b"").unwrap();
        fs::write(
            scripts.join("issue-request.sh"),
            b"printf entered > \"$PKI_CA_DIR/script-entered\"\nexit 1\n",
        )
        .unwrap();
        let adapter = OpensslCaAdapter::new(&scripts, &ca);
        for purpose in [
            CertificateRequestProfile::Partner,
            CertificateRequestProfile::DocumentSigning,
        ] {
            assert!(adapter
                .issue_request("invalid-subject", &request_with_subject(subject), purpose)
                .is_err());
            assert!(
                !ca.join("script-entered").exists(),
                "issuer policy must reject before mutable script execution: {subject}"
            );
        }
        assert_eq!(fs::read(ca.join("serial")).unwrap(), b"1002\n");
        assert!(fs::read(ca.join("index.txt")).unwrap().is_empty());
        assert!(!ca.join("certs").exists());
        assert!(!ca.join("newcerts").exists());
    }
}

#[test]
fn issuer_policy_reports_signed_subject_mismatch_without_losing_public_identity() {
    for subject in [
        "CN=External Holder",
        "CN=External Holder,O=Despacho Juridico Demo",
        "CN=External Holder,C=MX",
        "CN=External Holder,O=Despacho Juridico Demo,C=US",
        "CN=External Holder,O=Another Office,C=MX",
        "O=Despacho Juridico Demo,C=MX",
    ] {
        let csr = request_with_subject(subject);
        let found = inspect_ca_request(&csr, &fixture().root).unwrap();
        assert!(!found.issuer_policy_matches, "subject: {subject}");
        assert_eq!(found.csr_sha256, RingSha256Hasher.hash_bytes(&csr));
        assert_eq!(
            found.subject,
            CertReq::from_pem(&csr).unwrap().info.subject.to_string()
        );
    }
}

#[test]
fn issuer_policy_uses_the_adopted_root_and_every_repeated_subject_attribute() {
    for subject in [
        "CN=External Holder,O=Despacho Juridico Demo,C=MX",
        "C=MX,CN=External Holder,O=Despacho Juridico Demo",
        "CN=External Holder+O=Despacho Juridico Demo,C=MX",
        "CN=External Holder,O=Despacho Juridico Demo,O=Despacho Juridico Demo,C=MX",
    ] {
        assert!(
            inspect_ca_request(&request_with_subject(subject), &fixture().root)
                .unwrap()
                .issuer_policy_matches
        );
    }
    let mut authority = root();
    authority.tbs_certificate.subject =
        Name::from_str("CN=Alternate CA,O=Alternate Office,O=Second Office,C=US").unwrap();
    authority.tbs_certificate.issuer = authority.tbs_certificate.subject.clone();
    let authority = signed_certificate(authority);
    for (subject, expected) in [
        ("CN=External Holder,O=Alternate Office,C=US", true),
        ("CN=External Holder,O=Second Office,C=US", true),
        ("CN=External Holder,O=Despacho Juridico Demo,C=MX", false),
        (
            "CN=External Holder,O=Alternate Office,O=Another Office,C=US",
            false,
        ),
        ("CN=External Holder,O=Alternate Office,C=US,C=MX", false),
    ] {
        assert_eq!(
            inspect_ca_request(&request_with_subject(subject), &authority)
                .unwrap()
                .issuer_policy_matches,
            expected,
            "subject: {subject}"
        );
    }
}

#[test]
fn issuer_policy_compares_asn1_types_and_does_not_turn_bad_proofs_into_rejections() {
    let printable = der::asn1::PrintableStringRef::new("Despacho Juridico Demo")
        .unwrap()
        .to_der()
        .unwrap()
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>();
    let csr = request_with_subject(&format!("CN=External Holder,O=#{printable},C=MX"));
    assert!(
        !inspect_ca_request(&csr, &fixture().root)
            .unwrap()
            .issuer_policy_matches
    );
    let mut damaged = CertReq::from_pem(&csr).unwrap();
    let mut signature = damaged.signature.raw_bytes().to_vec();
    signature[0] ^= 1;
    damaged.signature = BitString::from_bytes(&signature).unwrap();
    let damaged = damaged.to_pem(der::pem::LineEnding::LF).unwrap();
    assert!(inspect_ca_request(damaged.as_bytes(), &fixture().root).is_err());
    assert!(inspect_ca_request(&csr, b"invalid root").is_err());
}
