//! CRL profile rules exercised on the decoded root CRL changed one field at a
//! time.
//!
//! [`parse`] applies every rule that does not depend on the issuer, so a
//! changed list reaches the rule under test even though the issuer signature
//! no longer verifies over it.

use core::time::Duration;

use der::asn1::{BitString, ObjectIdentifier, OctetString, UtcTime};
use der::oid::AssociatedOid;
use der::{Decode, Encode};
use x509_cert::crl::{CertificateList, RevokedCert};
use x509_cert::ext::pkix::{AuthorityKeyIdentifier, CrlNumber};
use x509_cert::ext::Extension;
use x509_cert::serial_number::SerialNumber;
use x509_cert::time::Time;
use x509_cert::Version;

use super::{cert, crl, FOREIGN_ROOT, ROOT, ROOT_CRL};
use crate::crl::{check_issued_by, parse};
use crate::report::Outcome;

const REASON_CODE: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.21");
const SHA1_WITH_RSA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.5");

fn changed(change: impl FnOnce(&mut CertificateList)) -> Vec<u8> {
    let mut list = CertificateList::from_der(ROOT_CRL).unwrap();
    change(&mut list);
    list.to_der().unwrap()
}

fn assert_rule(code: &str, change: impl FnOnce(&mut CertificateList)) {
    let finding = parse(&changed(change)).unwrap_err();
    assert_eq!(finding.outcome, Outcome::Failed, "{finding:?}");
    assert_eq!(finding.code, code, "{finding:?}");
}

fn extensions(list: &mut CertificateList) -> &mut Vec<Extension> {
    list.tbs_cert_list.crl_extensions.as_mut().unwrap()
}

fn find<T: AssociatedOid>(list: &mut CertificateList) -> &mut Extension {
    extensions(list)
        .iter_mut()
        .find(|extension| extension.extn_id == T::OID)
        .unwrap()
}

fn replace<T: AssociatedOid + Encode>(list: &mut CertificateList, value: &T) {
    find::<T>(list).extn_value = OctetString::new(value.to_der().unwrap()).unwrap();
}

fn aki(list: &mut CertificateList) -> AuthorityKeyIdentifier {
    let value = find::<AuthorityKeyIdentifier>(list).extn_value.clone();
    AuthorityKeyIdentifier::from_der(value.as_bytes()).unwrap()
}

fn entries(list: &mut CertificateList) -> &mut Vec<RevokedCert> {
    list.tbs_cert_list.revoked_certificates.as_mut().unwrap()
}

fn utc(seconds: i64) -> Time {
    let duration = Duration::from_secs(u64::try_from(seconds).unwrap());
    Time::UtcTime(UtcTime::from_unix_duration(duration).unwrap())
}

#[test]
fn header_rules_are_enforced() {
    assert_rule("crl.profile.version", |l| {
        l.tbs_cert_list.version = Version::V1;
    });
    assert_rule("crl.profile.algorithm", |l| {
        l.signature_algorithm.oid = SHA1_WITH_RSA;
    });
    assert_rule("crl.profile.algorithm", |l| {
        l.tbs_cert_list.signature.oid = SHA1_WITH_RSA;
        l.signature_algorithm.oid = SHA1_WITH_RSA;
    });
    assert_rule("crl.profile.algorithm", |l| {
        l.tbs_cert_list.signature.parameters = None;
        l.signature_algorithm.parameters = None;
    });
    assert_rule("crl.profile.signature_length", |l| {
        l.signature = BitString::from_bytes(&[1; 383]).unwrap();
    });
    assert_rule("crl.profile.signature_length", |l| {
        l.signature = BitString::new(1, [2; 384].to_vec()).unwrap();
    });
}

#[test]
fn extension_inventory_is_exact() {
    assert_rule("crl.profile.extensions", |l| {
        l.tbs_cert_list.crl_extensions = None;
    });
    assert_rule("crl.profile.extensions", |l| {
        extensions(l).retain(|extension| extension.extn_id != CrlNumber::OID);
    });
    assert_rule("crl.profile.extensions", |l| {
        let copy = find::<CrlNumber>(l).clone();
        extensions(l).push(copy);
    });
    assert_rule("crl.profile.extensions", |l| {
        let mut copy = find::<CrlNumber>(l).clone();
        copy.extn_id = REASON_CODE;
        extensions(l).push(copy);
    });
}

#[test]
fn authority_key_identifier_is_non_critical_and_key_id_only() {
    assert_rule("crl.profile.authority_key_id", |l| {
        find::<AuthorityKeyIdentifier>(l).critical = true;
    });
    assert_rule("crl.profile.authority_key_id", |l| {
        let mut value = aki(l);
        value.authority_cert_serial_number = Some(SerialNumber::new(&[1]).unwrap());
        replace(l, &value);
    });
    assert_rule("crl.profile.authority_key_id", |l| {
        let mut value = aki(l);
        value.key_identifier = None;
        replace(l, &value);
    });
    assert_rule("crl.profile.authority_key_id", |l| {
        find::<AuthorityKeyIdentifier>(l).extn_value = OctetString::new([5, 0]).unwrap();
    });
}

#[test]
fn crl_number_is_non_critical_and_at_most_eight_bytes() {
    assert_rule("crl.profile.number", |l| {
        find::<CrlNumber>(l).critical = true;
    });
    assert_rule("crl.profile.number", |l| {
        let nine = der::asn1::Uint::new(&[1; 9]).unwrap();
        replace(l, &CrlNumber(nine));
    });
    let eight = changed(|l| {
        let value = der::asn1::Uint::new(&[0x7F; 8]).unwrap();
        replace(l, &CrlNumber(value));
    });
    assert_eq!(parse(&eight).unwrap().number_value(), 0x7F7F_7F7F_7F7F_7F7F);
}

#[test]
fn next_update_is_present_and_after_this_update() {
    assert_rule("crl.profile.next_update", |l| {
        l.tbs_cert_list.next_update = None;
    });
    assert_rule("crl.profile.next_update", |l| {
        l.tbs_cert_list.next_update = Some(l.tbs_cert_list.this_update);
    });
}

#[test]
fn entries_are_bounded_unique_plain_and_not_after_this_update() {
    assert_rule("crl.profile.entry_serial", |l| {
        entries(l)[0].serial_number = SerialNumber::new(&[0]).unwrap();
    });
    assert_rule("crl.profile.entry_serial", |l| {
        let mut long = vec![0x02, 21, 0x00];
        long.extend([0x80; 20]);
        entries(l)[0].serial_number = SerialNumber::from_der(&long).unwrap();
    });
    assert_rule("crl.profile.entry_serial", |l| {
        let copy = entries(l)[0].clone();
        entries(l).push(copy);
    });
    assert_rule("crl.profile.entry_extensions", |l| {
        entries(l)[0].crl_entry_extensions = Some(vec![Extension {
            extn_id: REASON_CODE,
            critical: false,
            extn_value: OctetString::new([0x0A, 0x01, 0x01]).unwrap(),
        }]);
    });
    let this_update = super::crl_fact("root.crl.der", "this_update");
    assert_rule("crl.profile.revocation_date", |l| {
        entries(l)[0].revocation_date = utc(this_update + 1);
    });
    let at_this_update = changed(|l| entries(l)[0].revocation_date = utc(this_update));
    assert_eq!(parse(&at_this_update).unwrap().revoked[0].1, this_update);
}

#[test]
fn more_than_ten_thousand_entries_are_refused() {
    assert_rule("crl.profile.entry_limit", |l| {
        let entry = entries(l)[0].clone();
        entries(l).resize(10_001, entry);
    });
}

#[test]
fn issuer_name_must_be_the_issuer_subject() {
    let renamed = changed(|l| {
        l.tbs_cert_list.issuer = cert(FOREIGN_ROOT).subject;
    });
    let finding = check_issued_by(&crl(&renamed), &cert(ROOT)).unwrap_err();
    assert_eq!(finding.outcome, Outcome::Failed);
    assert_eq!(finding.code, "crl.issuer.name_mismatch");
}

#[test]
fn a_changed_list_no_longer_verifies() {
    let later = changed(|l| {
        let next = l.tbs_cert_list.next_update.unwrap();
        let seconds = next.to_unix_duration().as_secs() + 1;
        l.tbs_cert_list.next_update = Some(utc(i64::try_from(seconds).unwrap()));
    });
    let finding = check_issued_by(&crl(&later), &cert(ROOT)).unwrap_err();
    assert_eq!(finding.code, "crl.issuer.signature_invalid");
}
