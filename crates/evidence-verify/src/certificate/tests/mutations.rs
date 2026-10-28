//! Profile rules exercised on decoded fixtures changed one field at a time.
//!
//! Only the root rule checks a signature inside the profile, so a changed
//! leaf keeps reaching the rule under test even though its issuer signature
//! no longer verifies.

use der::asn1::{BitString, ObjectIdentifier, OctetString};
use der::oid::AssociatedOid;
use der::{Decode, Encode};
use rsa::pkcs8::EncodePublicKey;
use rsa::traits::PublicKeyParts;
use rsa::{BigUint, RsaPublicKey};
use spki::SubjectPublicKeyInfoOwned;
use x509_cert::ext::pkix::name::GeneralName;
use x509_cert::ext::pkix::{
    AuthorityKeyIdentifier, BasicConstraints, ExtendedKeyUsage, KeyUsage, KeyUsages,
    SubjectKeyIdentifier,
};
use x509_cert::ext::Extension;
use x509_cert::name::Name;
use x509_cert::serial_number::SerialNumber;
use x509_cert::{Certificate as X509, Version};

use super::{parsed, FOREIGN_ROOT, HOLDER, ROOT, SERVER_SIGNER, TSA};
use crate::certificate::{check_profile, Role};
use crate::report::Outcome;

const CLIENT_AUTH: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.2");
const TIME_STAMPING: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.8");

fn decoded(der: &[u8]) -> X509 {
    X509::from_der(der).unwrap()
}

fn assert_rule(der: &[u8], role: Role, code: &str, change: impl FnOnce(&mut X509)) {
    let mut certificate = decoded(der);
    change(&mut certificate);
    let finding = check_profile(&parsed(&certificate.to_der().unwrap()), role).unwrap_err();
    assert_eq!(finding.outcome, Outcome::Failed, "{finding:?}");
    assert_eq!(finding.code, code, "{finding:?}");
}

fn extensions(certificate: &mut X509) -> &mut Vec<Extension> {
    certificate.tbs_certificate.extensions.as_mut().unwrap()
}

fn find<T: AssociatedOid>(certificate: &mut X509) -> &mut Extension {
    extensions(certificate)
        .iter_mut()
        .find(|extension| extension.extn_id == T::OID)
        .unwrap()
}

fn replace<T: AssociatedOid + Encode>(certificate: &mut X509, value: &T, critical: bool) {
    let extension = find::<T>(certificate);
    extension.critical = critical;
    extension.extn_value = OctetString::new(value.to_der().unwrap()).unwrap();
}

fn key_id(certificate: &mut X509) -> OctetString {
    let value = find::<SubjectKeyIdentifier>(certificate).extn_value.clone();
    SubjectKeyIdentifier::from_der(value.as_bytes()).unwrap().0
}

#[test]
fn header_rules_are_enforced() {
    let role = Role::IndividualSigner;
    assert_rule(HOLDER, role, "cert.profile.version", |c| {
        c.tbs_certificate.version = Version::V2;
    });
    assert_rule(HOLDER, role, "cert.profile.unique_ids", |c| {
        c.tbs_certificate.subject_unique_id = Some(BitString::from_bytes(&[1]).unwrap());
    });
    assert_rule(HOLDER, role, "cert.profile.unique_ids", |c| {
        c.tbs_certificate.issuer_unique_id = Some(BitString::from_bytes(&[1]).unwrap());
    });
    assert_rule(HOLDER, role, "cert.profile.empty_name", |c| {
        c.tbs_certificate.subject = Name::default();
    });
    assert_rule(HOLDER, role, "cert.profile.validity", |c| {
        let validity = &mut c.tbs_certificate.validity;
        core::mem::swap(&mut validity.not_before, &mut validity.not_after);
    });
}

#[test]
fn signature_algorithm_and_value_rules_are_enforced() {
    let role = Role::IndividualSigner;
    assert_rule(HOLDER, role, "cert.profile.signature_algorithm", |c| {
        c.tbs_certificate.signature.parameters = None;
        c.signature_algorithm.parameters = None;
    });
    assert_rule(HOLDER, role, "cert.profile.signature_algorithm", |c| {
        c.signature_algorithm.oid = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.12");
    });
    assert_rule(HOLDER, role, "cert.profile.signature_algorithm", |c| {
        let sha384 = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.12");
        c.signature_algorithm.oid = sha384;
        c.tbs_certificate.signature.oid = sha384;
    });
    assert_rule(HOLDER, role, "cert.profile.signature_length", |c| {
        let bytes = c.signature.raw_bytes()[1..].to_vec();
        c.signature = BitString::from_bytes(&bytes).unwrap();
    });
    assert_rule(HOLDER, role, "cert.profile.signature_length", |c| {
        let bytes = c.signature.raw_bytes().to_vec();
        c.signature = BitString::new(1, bytes).unwrap();
    });
}

#[test]
fn serial_rules_are_enforced() {
    let role = Role::IndividualSigner;
    assert_rule(HOLDER, role, "cert.profile.serial", |c| {
        c.tbs_certificate.serial_number = SerialNumber::new(&[0]).unwrap();
    });
    assert_rule(HOLDER, role, "cert.profile.serial", |c| {
        let mut encoded = vec![0x02, 21, 0x00];
        encoded.extend_from_slice(&[0x80; 20]);
        c.tbs_certificate.serial_number = SerialNumber::from_der(&encoded).unwrap();
    });
}

#[test]
fn public_key_rules_are_enforced() {
    let role = Role::IndividualSigner;
    assert_rule(HOLDER, role, "cert.profile.key_algorithm", |c| {
        c.tbs_certificate
            .subject_public_key_info
            .algorithm
            .parameters = None;
    });
    assert_rule(HOLDER, role, "cert.profile.key_algorithm", |c| {
        c.tbs_certificate.subject_public_key_info.algorithm.oid =
            ObjectIdentifier::new_unwrap("1.2.840.10045.2.1");
    });
    assert_rule(HOLDER, role, "cert.profile.key_exponent", |c| {
        let modulus = parsed(HOLDER).public_key.unwrap().n().clone();
        let key = RsaPublicKey::new(modulus, BigUint::from(3u32)).unwrap();
        let der = key.to_public_key_der().unwrap();
        c.tbs_certificate.subject_public_key_info =
            SubjectPublicKeyInfoOwned::from_der(der.as_bytes()).unwrap();
    });
}

#[test]
fn extension_inventory_is_exact() {
    assert_rule(
        HOLDER,
        Role::IndividualSigner,
        "cert.profile.extensions",
        |c| {
            c.tbs_certificate.extensions = None;
        },
    );
    assert_rule(
        HOLDER,
        Role::IndividualSigner,
        "cert.profile.extensions",
        |c| {
            let duplicate = find::<SubjectKeyIdentifier>(c).clone();
            *find::<AuthorityKeyIdentifier>(c) = duplicate;
        },
    );
    assert_rule(ROOT, Role::Root, "cert.profile.extensions", |c| {
        let authority = decoded(HOLDER)
            .tbs_certificate
            .extensions
            .unwrap()
            .into_iter()
            .find(|extension| extension.extn_id == AuthorityKeyIdentifier::OID)
            .unwrap();
        extensions(c).push(authority);
    });
}

#[test]
fn extended_key_usage_is_exact_per_role() {
    assert_rule(
        SERVER_SIGNER,
        Role::ServerSealSigner,
        "cert.profile.extended_key_usage",
        |c| {
            replace(c, &ExtendedKeyUsage(vec![CLIENT_AUTH]), false);
        },
    );
    assert_rule(
        SERVER_SIGNER,
        Role::ServerSealSigner,
        "cert.profile.extended_key_usage",
        |c| {
            let email = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.4");
            replace(c, &ExtendedKeyUsage(vec![CLIENT_AUTH, email]), true);
        },
    );
    assert_rule(
        TSA,
        Role::TimeStamping,
        "cert.profile.extended_key_usage",
        |c| {
            replace(c, &ExtendedKeyUsage(vec![TIME_STAMPING, CLIENT_AUTH]), true);
        },
    );
}

#[test]
fn basic_constraints_and_key_usage_are_exact() {
    assert_rule(ROOT, Role::Root, "cert.profile.basic_constraints", |c| {
        let constraints = BasicConstraints {
            ca: true,
            path_len_constraint: Some(0),
        };
        replace(c, &constraints, true);
    });
    assert_rule(
        HOLDER,
        Role::IndividualSigner,
        "cert.profile.basic_constraints",
        |c| {
            let constraints = BasicConstraints {
                ca: false,
                path_len_constraint: None,
            };
            replace(c, &constraints, false);
        },
    );
    assert_rule(
        HOLDER,
        Role::IndividualSigner,
        "cert.profile.key_usage",
        |c| {
            let usage = KeyUsages::DigitalSignature | KeyUsages::NonRepudiation;
            replace(c, &KeyUsage(usage), false);
        },
    );
    assert_rule(
        HOLDER,
        Role::IndividualSigner,
        "cert.profile.key_usage",
        |c| {
            let usage = KeyUsages::DigitalSignature
                | KeyUsages::NonRepudiation
                | KeyUsages::KeyEncipherment;
            replace(c, &KeyUsage(usage), true);
        },
    );
    assert_rule(TSA, Role::TimeStamping, "cert.profile.key_usage", |c| {
        let usage = KeyUsages::DigitalSignature | KeyUsages::NonRepudiation;
        replace(c, &KeyUsage(usage), true);
    });
}

#[test]
fn subject_key_identifier_is_the_sha1_of_the_key() {
    assert_rule(
        HOLDER,
        Role::IndividualSigner,
        "cert.profile.subject_key_id",
        |c| {
            let mut value = key_id(c).as_bytes().to_vec();
            value[0] ^= 1;
            replace(
                c,
                &SubjectKeyIdentifier(OctetString::new(value).unwrap()),
                false,
            );
        },
    );
    assert_rule(
        HOLDER,
        Role::IndividualSigner,
        "cert.profile.subject_key_id",
        |c| {
            let value = key_id(c);
            replace(c, &SubjectKeyIdentifier(value), true);
        },
    );
}

#[test]
fn authority_key_identifier_is_a_bare_twenty_byte_key_id() {
    let issuer = decoded(ROOT).tbs_certificate;
    let mut root = decoded(ROOT);
    let root_id = key_id(&mut root);
    let aki = |key_identifier| AuthorityKeyIdentifier {
        key_identifier,
        authority_cert_issuer: None,
        authority_cert_serial_number: None,
    };
    let role = Role::IndividualSigner;
    let code = "cert.profile.authority_key_id";
    assert_rule(HOLDER, role, code, |c| {
        let mut value = aki(Some(root_id.clone()));
        value.authority_cert_issuer = Some(vec![GeneralName::DirectoryName(issuer.subject)]);
        value.authority_cert_serial_number = Some(issuer.serial_number);
        replace(c, &value, false);
    });
    assert_rule(HOLDER, role, code, |c| {
        let short = OctetString::new(&root_id.as_bytes()[..19]).unwrap();
        replace(c, &aki(Some(short)), false);
    });
    assert_rule(HOLDER, role, code, |c| replace(c, &aki(None), false));
    assert_rule(HOLDER, role, code, |c| {
        replace(c, &aki(Some(root_id.clone())), true)
    });
}

#[test]
fn the_root_must_be_self_signed() {
    assert_rule(ROOT, Role::Root, "cert.profile.self_signature", |c| {
        let mut bytes = c.signature.raw_bytes().to_vec();
        bytes[383] ^= 1;
        c.signature = BitString::from_bytes(&bytes).unwrap();
    });
    assert_rule(ROOT, Role::Root, "cert.profile.self_signature", |c| {
        c.tbs_certificate.issuer = decoded(FOREIGN_ROOT).tbs_certificate.subject;
    });
}
