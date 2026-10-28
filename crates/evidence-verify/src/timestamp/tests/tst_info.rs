//! Checks of the `TSTInfo` content over constructed encodings.

use der::asn1::{GeneralizedTime, Int, OctetString};
use der::{Any, Encode};
use x509_cert::ext::pkix::name::GeneralName;
use x509_cert::ext::Extension;
use x509_tsp::{Accuracy, MessageImprint, TspVersion, TstInfo};

use super::shape::encode;
use super::*;
use crate::certificate;
use crate::der_walk;
use crate::timestamp::tstinfo::{self, TstFacts};

const SHA256: &str = "2.16.840.1.101.3.4.2.1";
const NULL: &[u8] = &[0x05, 0x00];
/// Index of `genTime` among the `TSTInfo` fields.
const GEN_TIME: usize = 4;

/// Parse `encoded` and check its imprint against `covered`, in the order
/// [`Token::verify`] applies them.
fn check(encoded: &[u8], covered: &[u8; 32]) -> Result<TstFacts, Finding> {
    let facts = tstinfo::parse(encoded)?;
    facts.check_imprint(covered)?;
    Ok(facts)
}

fn tst_info(covered: &[u8; 32]) -> TstInfo {
    TstInfo {
        version: TspVersion::V1,
        policy: INTERNAL_POLICY.parse().unwrap(),
        message_imprint: MessageImprint {
            hash_algorithm: spki::AlgorithmIdentifier {
                oid: SHA256.parse().unwrap(),
                parameters: Some(Any::null()),
            },
            hashed_message: OctetString::new(&covered[..]).unwrap(),
        },
        serial_number: Int::new(&[0x10, 0x01]).unwrap(),
        gen_time: GeneralizedTime::from_unix_duration(core::time::Duration::from_secs(
            1_793_165_512,
        ))
        .unwrap(),
        accuracy: None,
        ordering: false,
        nonce: None,
        tsa: None,
        extensions: None,
    }
}

fn accuracy(seconds: Option<u64>, millis: Option<i16>, micros: Option<i16>) -> Option<Accuracy> {
    Some(Accuracy {
        seconds,
        millis,
        micros,
    })
}

#[test]
fn tst_info_facts_round_accuracy_up_to_whole_seconds() {
    let covered = document_digest();
    let cases = [
        (None, 0),
        (accuracy(None, None, None), 0),
        (accuracy(Some(1), None, None), 1),
        (accuracy(Some(1), Some(500), None), 2),
        (accuracy(None, None, Some(1)), 1),
        (accuracy(Some(3), Some(999), Some(999)), 4),
    ];
    for (value, expected) in cases {
        let mut info = tst_info(&covered);
        info.accuracy = value;
        let facts = check(&info.to_der().unwrap(), &covered).unwrap();
        assert_eq!(facts.accuracy_seconds, expected);
        assert_eq!(facts.gen_time, 1_793_165_512);
        assert_eq!(facts.serial, [0x10, 0x01]);
        assert_eq!(facts.nonce, None);
    }
    let mut info = tst_info(&covered);
    info.accuracy = accuracy(None, Some(-1), None);
    let result = check(&info.to_der().unwrap(), &covered).map(|_| ());
    assert_finding(result, Outcome::Failed, "token.malformed");
}

#[test]
fn tst_info_admits_only_version_one_without_critical_extensions() {
    let covered = document_digest();
    let mut encoded = tst_info(&covered).to_der().unwrap();
    let (outer, _) = der_walk::read(&encoded).unwrap();
    let version_at = outer.header.len() + 2;
    encoded[version_at] = 2;
    let result = check(&encoded, &covered).map(|_| ());
    assert_finding(result, Outcome::NotSupported, "token.version");
    let mut info = tst_info(&covered);
    info.extensions = Some(vec![Extension {
        extn_id: "1.2.3.4".parse().unwrap(),
        critical: true,
        extn_value: OctetString::new(NULL).unwrap(),
    }]);
    let result = check(&info.to_der().unwrap(), &covered).map(|_| ());
    assert_finding(result, Outcome::NotSupported, "token.critical_extension");
    if let Some(extensions) = info.extensions.as_mut() {
        extensions[0].critical = false;
    }
    assert!(check(&info.to_der().unwrap(), &covered).is_ok());
}

#[test]
fn tst_info_imprint_must_be_sha256_of_the_covered_object() {
    let covered = document_digest();
    let mut info = tst_info(&covered);
    info.message_imprint.hash_algorithm.oid = "1.3.14.3.2.26".parse().unwrap();
    let result = check(&info.to_der().unwrap(), &covered).map(|_| ());
    assert_finding(result, Outcome::Failed, "token.imprint_mismatch");
    let mut info = tst_info(&covered);
    info.message_imprint.hash_algorithm.parameters = None;
    assert!(check(&info.to_der().unwrap(), &covered).is_ok());
    let other = certificate::sha256(b"another object");
    let result = check(&tst_info(&covered).to_der().unwrap(), &other).map(|_| ());
    assert_finding(result, Outcome::Failed, "token.imprint_mismatch");
}

#[test]
fn tst_info_must_be_strict_der() {
    let covered = document_digest();
    let mut encoded = tst_info(&covered).to_der().unwrap();
    encoded.push(0);
    let result = check(&encoded, &covered).map(|_| ());
    assert_finding(result, Outcome::Failed, "token.malformed");
}
/// `info` encoded with its `genTime` element replaced by `gen_time`.
fn with_gen_time(info: &TstInfo, gen_time: &[u8]) -> Vec<u8> {
    let encoded = info.to_der().unwrap();
    let outer = der_walk::read_exact(&encoded).unwrap();
    let fields = der_walk::children(outer.value, 16).unwrap();
    assert_eq!(fields[GEN_TIME].tag, 0x18);
    let mut value = Vec::new();
    for (at, field) in fields.iter().enumerate() {
        if at == GEN_TIME {
            value.extend(encode(0x18, gen_time));
        } else {
            value.extend_from_slice(field.raw);
        }
    }
    encode(0x30, &value)
}

#[test]
fn a_gen_time_with_fractional_seconds_is_not_supported() {
    let covered = document_digest();
    let info = tst_info(&covered);
    assert_eq!(
        with_gen_time(&info, b"20261028053152Z"),
        info.to_der().unwrap()
    );
    for fraction in [&b"20261028053152.5Z"[..], b"20261028053152.123Z"] {
        let result = check(&with_gen_time(&info, fraction), &covered);
        assert_finding(result, Outcome::NotSupported, "token.gen_time_fraction");
    }
    for invalid in [
        &b"20261028053152"[..],
        b"2026102805315Z",
        b"20261028053152.Z",
        b"20261028053152.50Z",
        b"2026102805315a.5Z",
    ] {
        let result = check(&with_gen_time(&info, invalid), &covered);
        assert_finding(result, Outcome::Failed, "token.malformed");
    }
}

#[test]
fn tst_info_reports_ordering_and_the_authority_name() {
    let covered = document_digest();
    let facts = check(&tst_info(&covered).to_der().unwrap(), &covered).unwrap();
    assert!(!facts.ordering);
    assert_eq!(facts.tsa, None);
    let mut info = tst_info(&covered);
    info.ordering = true;
    info.tsa = Some(GeneralName::DirectoryName(tsa().subject));
    let facts = check(&info.to_der().unwrap(), &covered).unwrap();
    assert!(facts.ordering);
    assert_eq!(facts.tsa, Some(tsa().subject.to_string()));
    info.tsa = Some(GeneralName::DnsName(
        der::asn1::Ia5String::new("tsa.example").unwrap(),
    ));
    let facts = check(&info.to_der().unwrap(), &covered).unwrap();
    assert_eq!(facts.tsa.as_deref(), Some("dns:tsa.example"));
}

#[test]
fn an_explicit_default_ordering_is_not_der() {
    let covered = document_digest();
    let info = tst_info(&covered);
    let encoded = info.to_der().unwrap();
    let outer = der_walk::read_exact(&encoded).unwrap();
    let mut value = outer.value.to_vec();
    value.extend_from_slice(&[0x01, 0x01, 0x00]);
    let result = check(&encode(0x30, &value), &covered);
    assert_finding(result, Outcome::Failed, "token.malformed");
}
