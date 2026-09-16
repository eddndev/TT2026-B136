use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use der::{Decode, DecodePem, Encode};
use domain::crypto::{CertificateValidator, DocumentHasher, DocumentSigner, Signature};
use infrastructure::{RingSha256Hasher, RsaPkcs1Signer, X509ChainValidator};
use x509_cert::{crl::CertificateList, Certificate};
use zeroize::Zeroizing;

pub struct Fixture {
    pub _directory: tempfile::TempDir,
    pub ca: PathBuf,
    pub root: Vec<u8>,
    pub leaf: Vec<u8>,
    pub leaf_der: Vec<u8>,
    pub other_leaf: Vec<u8>,
    pub crl: Vec<u8>,
    pub statement: Vec<u8>,
    pub signature: Signature,
    pub at: i64,
}

pub fn script(name: &str, ca: &Path, args: &[&str]) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../pki")
        .join(name);
    let output = Command::new("bash")
        .arg(path)
        .args(args)
        .env("PKI_CA_DIR", ca)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "script failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn openssl(args: &[&str]) -> Vec<u8> {
    let output = Command::new("openssl").args(args).output().unwrap();
    assert!(
        output.status.success(),
        "openssl failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

pub fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let directory = tempfile::tempdir().unwrap();
        let ca = directory.path().join("ca");
        script("init-ca.sh", &ca, &[]);
        script("issue-declaration-cert.sh", &ca, &["Synthetic Declarant"]);
        script("issue-declaration-cert.sh", &ca, &["Second Declarant"]);
        script("gen-crl.sh", &ca, &[]);
        let leaf_path = ca.join("certs/synthetic-declarant.crt.pem");
        let leaf = fs::read(&leaf_path).unwrap();
        let leaf_der = openssl(&[
            "x509",
            "-in",
            leaf_path.to_str().unwrap(),
            "-outform",
            "DER",
        ]);
        let statement = b"Canonical synthetic declaration bytes".to_vec();
        let statement_path = directory.path().join("statement.bin");
        fs::write(&statement_path, &statement).unwrap();
        let signature = Signature::from_bytes(openssl(&[
            "dgst",
            "-sha256",
            "-sign",
            ca.join("private/synthetic-declarant.key.pem")
                .to_str()
                .unwrap(),
            statement_path.to_str().unwrap(),
        ]))
        .unwrap();
        let at = X509ChainValidator::new()
            .inspect(&leaf)
            .unwrap()
            .not_before_unix
            + 3600;
        Fixture {
            root: fs::read(ca.join("ca.crt.pem")).unwrap(),
            crl: fs::read(ca.join("crl/crl.pem")).unwrap(),
            _directory: directory,
            ca,
            leaf,
            other_leaf: fs::read(leaf_path.parent().unwrap().join("second-declarant.crt.pem"))
                .unwrap(),
            leaf_der,
            statement,
            signature,
            at,
        }
    })
}

pub fn leaf() -> Certificate {
    Certificate::from_der(&fixture().leaf_der).unwrap()
}

pub fn root() -> Certificate {
    Certificate::from_pem(&fixture().root).unwrap()
}

pub fn crl() -> CertificateList {
    let (_, der) = der::pem::decode_vec(&fixture().crl).unwrap();
    CertificateList::from_der(&der).unwrap()
}

fn sign(bytes: &[u8]) -> der::asn1::BitString {
    let key = Zeroizing::new(fs::read(fixture().ca.join("private/ca.key.pem")).unwrap());
    let signature = RsaPkcs1Signer::new(key)
        .unwrap()
        .sign(&RingSha256Hasher.hash_bytes(bytes))
        .unwrap();
    der::asn1::BitString::from_bytes(signature.as_bytes()).unwrap()
}

pub fn signed_certificate(mut certificate: Certificate) -> Vec<u8> {
    certificate.signature = sign(&certificate.tbs_certificate.to_der().unwrap());
    certificate.to_der().unwrap()
}

pub fn signed_crl(mut crl: CertificateList) -> Vec<u8> {
    crl.signature = sign(&crl.tbs_cert_list.to_der().unwrap());
    crl.to_der().unwrap()
}

pub fn time(seconds: i64) -> x509_cert::time::Time {
    x509_cert::time::Time::UtcTime(
        der::asn1::UtcTime::from_unix_duration(std::time::Duration::from_secs(
            seconds.try_into().unwrap(),
        ))
        .unwrap(),
    )
}
