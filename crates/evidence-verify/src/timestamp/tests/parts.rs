//! Checks of single token parts over constructed encodings, for rules the
//! signed fixtures cannot exercise without a signing key.

use der::asn1::OctetString;
use der::{Any, Encode};
use sha1::Sha1;
use sha2::Digest;
use spki::AlgorithmIdentifierOwned;
use x509_cert::ext::pkix::name::GeneralName;

use super::*;
use crate::certificate::{self, Certificate};
use crate::der_walk;
use crate::timestamp::ess::{
    EssCertId, EssCertIdV2, IssuerSerial, SigningCertificate, SigningCertificateV2,
};
use crate::timestamp::{ess, signer};

fn issuer_serial(of: &Certificate) -> IssuerSerial {
    IssuerSerial {
        issuer: vec![GeneralName::DirectoryName(of.issuer.clone())],
        serial_number: of.decoded.tbs_certificate.serial_number.clone(),
    }
}

fn v1(cert_hash: &[u8], issuer: Option<IssuerSerial>) -> Vec<u8> {
    SigningCertificate {
        certs: vec![EssCertId {
            cert_hash: OctetString::new(cert_hash).unwrap(),
            issuer_serial: issuer,
        }],
        policies: None,
    }
    .to_der()
    .unwrap()
}

fn v2(algorithm: Option<&str>, cert_hash: &[u8]) -> Vec<u8> {
    SigningCertificateV2 {
        certs: vec![EssCertIdV2 {
            hash_algorithm: algorithm.map(|oid| AlgorithmIdentifierOwned {
                oid: oid.parse().unwrap(),
                parameters: None,
            }),
            cert_hash: OctetString::new(cert_hash).unwrap(),
            issuer_serial: None,
        }],
        policies: None,
    }
    .to_der()
    .unwrap()
}

#[test]
fn ess_v1_matches_the_sha1_of_the_signer_certificate() {
    let signer = tsa();
    let hash = Sha1::digest(TSA);
    assert!(ess::check_v1(&v1(&hash, None), &signer).is_ok());
    assert!(ess::check_v1(&v1(&hash, Some(issuer_serial(&signer))), &signer).is_ok());
    let wrong = Sha1::digest(ROOT);
    let result = ess::check_v1(&v1(&wrong, None), &signer);
    assert_finding(result, Outcome::Failed, "token.ess_mismatch");
}

#[test]
fn ess_issuer_serial_must_name_the_signer_certificate() {
    let signer = tsa();
    let hash = Sha1::digest(TSA);
    let mut other_serial = issuer_serial(&signer);
    other_serial.serial_number = root().decoded.tbs_certificate.serial_number;
    let result = ess::check_v1(&v1(&hash, Some(other_serial)), &signer);
    assert_finding(result, Outcome::Failed, "token.ess_mismatch");
    let mut other_issuer = issuer_serial(&signer);
    other_issuer.issuer = vec![GeneralName::DirectoryName(signer.subject.clone())];
    let result = ess::check_v1(&v1(&hash, Some(other_issuer)), &signer);
    assert_finding(result, Outcome::Failed, "token.ess_mismatch");
    let mut two_names = issuer_serial(&signer);
    two_names
        .issuer
        .push(GeneralName::DirectoryName(signer.issuer.clone()));
    let result = ess::check_v1(&v1(&hash, Some(two_names)), &signer);
    assert_finding(result, Outcome::Failed, "token.ess_mismatch");
}

#[test]
fn ess_v2_defaults_to_sha256_and_admits_no_other_hash() {
    let signer = tsa();
    let hash = certificate::sha256(TSA);
    assert!(ess::check_v2(&v2(None, &hash), &signer).is_ok());
    assert!(ess::check_v2(&v2(Some("2.16.840.1.101.3.4.2.1"), &hash), &signer).is_ok());
    let result = ess::check_v2(&v2(None, &certificate::sha256(ROOT)), &signer);
    assert_finding(result, Outcome::Failed, "token.ess_mismatch");
    let result = ess::check_v2(&v2(Some("2.16.840.1.101.3.4.2.2"), &hash), &signer);
    assert_finding(result, Outcome::NotSupported, "token.ess_hash_algorithm");
}

#[test]
fn ess_values_must_be_strict_and_name_a_certificate() {
    let signer = tsa();
    let empty = SigningCertificate {
        certs: Vec::new(),
        policies: None,
    };
    let result = ess::check_v1(&empty.to_der().unwrap(), &signer);
    assert_finding(result, Outcome::Failed, "token.ess_mismatch");
    let mut trailing = v1(&Sha1::digest(TSA), None);
    trailing.extend_from_slice(&[0x05, 0x00]);
    let result = ess::check_v1(&trailing, &signer);
    assert_finding(result, Outcome::Failed, "token.malformed");
    let result = ess::check_v2(&[0x30, 0x00], &signer);
    assert_finding(result, Outcome::Failed, "token.malformed");
}

fn tlv(bytes: &[u8]) -> der_walk::Tlv<'_> {
    der_walk::read_exact(bytes).unwrap()
}

fn algorithm(oid: &str, parameters: Option<Any>) -> Vec<u8> {
    AlgorithmIdentifierOwned {
        oid: oid.parse().unwrap(),
        parameters,
    }
    .to_der()
    .unwrap()
}

const SHA256: &str = "2.16.840.1.101.3.4.2.1";

#[test]
fn only_sha256_listed_in_digest_algorithms_is_admitted() {
    let absent = algorithm(SHA256, None);
    let null = algorithm(SHA256, Some(Any::null()));
    assert!(signer::check_digest_algorithm(tlv(&absent), &[tlv(&null)]).is_ok());
    assert!(signer::check_digest_algorithm(tlv(&null), &[tlv(&absent)]).is_ok());
    let sha384 = algorithm("2.16.840.1.101.3.4.2.2", None);
    let result = signer::check_digest_algorithm(tlv(&sha384), &[tlv(&sha384)]);
    assert_finding(result, Outcome::NotSupported, "token.digest_algorithm");
    let odd = algorithm(SHA256, Some(Any::new(der::Tag::Integer, [1u8]).unwrap()));
    let result = signer::check_digest_algorithm(tlv(&odd), &[tlv(&odd)]);
    assert_finding(result, Outcome::NotSupported, "token.digest_algorithm");
    let result = signer::check_digest_algorithm(tlv(&null), &[tlv(&sha384)]);
    assert_finding(result, Outcome::Failed, "token.malformed");
}

#[test]
fn only_rsa_signature_algorithms_are_admitted() {
    for oid in ["1.2.840.113549.1.1.1", "1.2.840.113549.1.1.11"] {
        let encoded = algorithm(oid, Some(Any::null()));
        assert!(signer::check_signature_algorithm(tlv(&encoded)).is_ok());
        let encoded = algorithm(oid, None);
        assert!(signer::check_signature_algorithm(tlv(&encoded)).is_ok());
    }
    for oid in [
        "1.2.840.10045.4.3.2",
        "1.2.840.113549.1.1.10",
        "1.2.840.113549.1.1.5",
    ] {
        let result = signer::check_signature_algorithm(tlv(&algorithm(oid, None)));
        assert_finding(result, Outcome::NotSupported, "token.signature_algorithm");
    }
}

#[test]
fn the_signer_identifier_must_match_exactly_one_certificate() {
    let certificates = [tsa(), root()];
    let ski = tsa().subject_key_id.unwrap();
    let mut by_key = vec![0x80, ski.len() as u8];
    by_key.extend_from_slice(&ski);
    assert_eq!(signer::find_signer(3, tlv(&by_key), &certificates), Ok(0));
    let by_name = ::cms::cert::IssuerAndSerialNumber {
        issuer: tsa().issuer,
        serial_number: tsa().decoded.tbs_certificate.serial_number,
    }
    .to_der()
    .unwrap();
    assert_eq!(signer::find_signer(1, tlv(&by_name), &certificates), Ok(0));
    let result = signer::find_signer(1, tlv(&by_key), &certificates).map(|_| ());
    assert_finding(result, Outcome::Failed, "token.malformed");
    let result = signer::find_signer(3, tlv(&by_name), &certificates).map(|_| ());
    assert_finding(result, Outcome::Failed, "token.malformed");
    let result = signer::find_signer(3, tlv(&by_key), &certificates[1..]).map(|_| ());
    assert_finding(result, Outcome::Failed, "token.signer_not_found");
    let twice = [tsa(), root(), tsa()];
    let result = signer::find_signer(3, tlv(&by_key), &twice).map(|_| ());
    assert_finding(result, Outcome::Failed, "token.signer_ambiguous");
}
