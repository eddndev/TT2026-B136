use super::*;
use crate::certificate;
use crate::report::Outcome;

mod mutations;

pub(super) const ROOT_CRL: &[u8] = include_bytes!("../../tests/fixtures/root.crl.der");
const ROOT_EMPTY_CRL: &[u8] = include_bytes!("../../tests/fixtures/root-empty.crl.der");
const FOREIGN_CRL: &[u8] = include_bytes!("../../tests/fixtures/foreign.crl.der");
pub(super) const ROOT: &[u8] = include_bytes!("../../tests/fixtures/root.der");
pub(super) const FOREIGN_ROOT: &[u8] = include_bytes!("../../tests/fixtures/foreign-root.der");
const HOLDER: &[u8] = include_bytes!("../../tests/fixtures/holder.der");
const REVOKED_HOLDER: &[u8] = include_bytes!("../../tests/fixtures/revoked-holder.der");
const FACTS: &str = include_str!("../../tests/fixtures/fixtures.json");

/// The JSON object recorded for `name` inside the top-level `section` of
/// `fixtures.json`, found by counting braces so nested objects stay inside.
fn object(section: &str, name: &str) -> &'static str {
    let section_start = FACTS.find(&format!("\"{section}\": {{")).unwrap();
    let after = &FACTS[section_start..];
    let start = after.find(&format!("\"{name}\": {{")).unwrap();
    let body = &after[start..];
    let open = body.find('{').unwrap();
    let mut depth = 0usize;
    for (index, character) in body[open..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &body[open..open + index + 1];
                }
            }
            _ => {}
        }
    }
    panic!("unterminated object for {name}");
}

/// The scalar text of the first `key` inside `object`, without quotes.
fn scalar<'a>(object: &'a str, key: &str) -> &'a str {
    let marker = format!("\"{key}\": ");
    let rest = &object[object.find(&marker).unwrap() + marker.len()..];
    if let Some(text) = rest.strip_prefix('"') {
        return &text[..text.find('"').unwrap()];
    }
    let end = rest.find([',', '\n', '}']).unwrap_or(rest.len());
    rest[..end].trim()
}

pub(super) fn crl_fact(file: &str, key: &str) -> i64 {
    scalar(object("crls", file), key).parse().unwrap()
}

fn gen_time() -> i64 {
    scalar(object("tokens", "token-ess-v2.tsr"), "gen_time")
        .parse()
        .unwrap()
}

pub(super) fn crl(der: &[u8]) -> Crl {
    parse(der).unwrap()
}

pub(super) fn cert(der: &[u8]) -> certificate::Certificate {
    certificate::parse(der).unwrap()
}

#[test]
fn parse_reads_the_fields_of_the_revoking_root_crl() {
    let parsed = crl(ROOT_CRL);
    assert_eq!(parsed.der, ROOT_CRL);
    assert_eq!(
        parsed.number_value(),
        crl_fact("root.crl.der", "crl_number") as u64
    );
    assert_eq!(parsed.number, [0x10, 0x01]);
    assert_eq!(parsed.this_update, crl_fact("root.crl.der", "this_update"));
    assert_eq!(parsed.next_update, crl_fact("root.crl.der", "next_update"));
    assert_eq!(parsed.issuer, cert(ROOT).subject);
    let revoked = cert(REVOKED_HOLDER);
    assert_eq!(
        revoked.serial_hex(),
        scalar(object("crls", "root.crl.der"), "serial_hex")
    );
    assert_eq!(
        parsed.revoked,
        vec![(revoked.serial, crl_fact("root.crl.der", "revocation"))]
    );
}

#[test]
fn parse_reads_the_empty_and_the_foreign_crls() {
    let empty = crl(ROOT_EMPTY_CRL);
    assert_eq!(
        empty.number_value(),
        crl_fact("root-empty.crl.der", "crl_number") as u64
    );
    assert!(empty.revoked.is_empty());
    assert_eq!(empty.issuer, cert(ROOT).subject);
    let foreign = crl(FOREIGN_CRL);
    assert_eq!(foreign.issuer, cert(FOREIGN_ROOT).subject);
    assert_eq!(
        foreign.this_update,
        crl_fact("foreign.crl.der", "this_update")
    );
}

#[test]
fn parse_rejects_bytes_that_are_not_one_strict_crl() {
    let mut trailing = ROOT_CRL.to_vec();
    trailing.push(0);
    let truncated = &ROOT_CRL[..ROOT_CRL.len() - 1];
    for input in [&b""[..], b"not der at all", truncated, &trailing, ROOT] {
        let finding = parse(input).unwrap_err();
        assert_eq!(finding.outcome, Outcome::Failed, "{finding:?}");
        assert_eq!(finding.code, "crl.malformed", "{finding:?}");
    }
}

#[test]
fn parse_rejects_a_crl_over_the_size_limit_as_not_supported() {
    let finding = parse(&vec![0x30; CRL_LIMIT + 1]).unwrap_err();
    assert_eq!(finding.outcome, Outcome::NotSupported);
    assert_eq!(finding.code, "crl.too_large");
}

#[test]
fn the_right_issuer_is_accepted() {
    assert_eq!(check_issued_by(&crl(ROOT_CRL), &cert(ROOT)), Ok(()));
    assert_eq!(check_issued_by(&crl(ROOT_EMPTY_CRL), &cert(ROOT)), Ok(()));
    assert_eq!(
        check_issued_by(&crl(FOREIGN_CRL), &cert(FOREIGN_ROOT)),
        Ok(())
    );
}

#[test]
fn a_foreign_issuer_fails_on_the_key_identifier() {
    for (list, issuer) in [
        (FOREIGN_CRL, ROOT),
        (ROOT_CRL, FOREIGN_ROOT),
        (ROOT_CRL, HOLDER),
    ] {
        let finding = check_issued_by(&crl(list), &cert(issuer)).unwrap_err();
        assert_eq!(finding.outcome, Outcome::Failed);
        assert_eq!(finding.code, "crl.issuer.key_id_mismatch");
    }
}

#[test]
fn a_changed_signature_fails_the_issuer_check() {
    let mut changed = ROOT_CRL.to_vec();
    *changed.last_mut().unwrap() ^= 1;
    let finding = check_issued_by(&crl(&changed), &cert(ROOT)).unwrap_err();
    assert_eq!(finding.outcome, Outcome::Failed);
    assert_eq!(finding.code, "crl.issuer.signature_invalid");
}

#[test]
fn covers_needs_the_whole_interval_inside_the_inclusive_window() {
    let parsed = crl(ROOT_CRL);
    let (start, end) = (parsed.this_update, parsed.next_update);
    assert!(covers(&parsed, start, end));
    assert!(covers(&parsed, start, start));
    assert!(covers(&parsed, end, end));
    assert!(!covers(&parsed, start - 1, end));
    assert!(!covers(&parsed, start, end + 1));
    assert!(!covers(&parsed, end, start));
}

#[test]
fn revocation_of_finds_only_the_listed_serial() {
    let parsed = crl(ROOT_CRL);
    assert_eq!(
        revocation_of(&parsed, &cert(REVOKED_HOLDER).serial),
        Some(crl_fact("root.crl.der", "revocation"))
    );
    assert_eq!(revocation_of(&parsed, &cert(HOLDER).serial), None);
    assert_eq!(
        revocation_of(&crl(ROOT_EMPTY_CRL), &cert(REVOKED_HOLDER).serial),
        None
    );
}

#[test]
fn status_is_checked_for_a_serial_absent_from_a_covering_crl() {
    let at = gen_time();
    let finding = status(
        &crl(ROOT_CRL),
        &cert(ROOT),
        &cert(HOLDER).serial,
        at - 1,
        at + 1,
    );
    assert_eq!(finding.outcome, Outcome::Checked, "{finding:?}");
    assert_eq!(finding.code, "crl.not_revoked");
    let empty = crl(ROOT_EMPTY_CRL);
    let finding = status(&empty, &cert(ROOT), &cert(REVOKED_HOLDER).serial, at, at);
    assert_eq!(finding.outcome, Outcome::Checked, "{finding:?}");
}

#[test]
fn status_fails_for_a_serial_revoked_before_the_instant() {
    let at = gen_time();
    let finding = status(
        &crl(ROOT_CRL),
        &cert(ROOT),
        &cert(REVOKED_HOLDER).serial,
        at,
        at,
    );
    assert_eq!(finding.outcome, Outcome::Failed, "{finding:?}");
    assert_eq!(finding.code, "crl.revoked");
    let revocation = crl_fact("root.crl.der", "revocation").to_string();
    assert!(finding.detail.contains(&revocation), "{finding:?}");
}

#[test]
fn status_is_indeterminate_when_the_right_crl_does_not_cover_the_interval() {
    let parsed = crl(ROOT_CRL);
    let (start, end) = (parsed.this_update, parsed.next_update);
    let root = cert(ROOT);
    let serial = cert(HOLDER).serial;
    for (from, to) in [(start - 1, start), (end, end + 1), (end + 10, end + 20)] {
        let finding = status(&parsed, &root, &serial, from, to);
        assert_eq!(finding.outcome, Outcome::Indeterminate, "{finding:?}");
        assert_eq!(finding.code, "crl.not_covering");
    }
}

#[test]
fn status_fails_for_a_crl_of_another_issuer_even_when_it_covers() {
    let at = gen_time();
    let finding = status(&crl(FOREIGN_CRL), &cert(ROOT), &cert(HOLDER).serial, at, at);
    assert_eq!(finding.outcome, Outcome::Failed);
    assert_eq!(finding.code, "crl.issuer.key_id_mismatch");
    let start = crl(FOREIGN_CRL).this_update;
    let finding = status(
        &crl(FOREIGN_CRL),
        &cert(ROOT),
        &cert(HOLDER).serial,
        start - 10,
        start - 5,
    );
    assert_eq!(finding.outcome, Outcome::Failed, "{finding:?}");
}

#[test]
fn status_fails_for_a_crl_whose_signature_does_not_verify() {
    let mut changed = ROOT_CRL.to_vec();
    *changed.last_mut().unwrap() ^= 1;
    let at = gen_time();
    let finding = status(&crl(&changed), &cert(ROOT), &cert(HOLDER).serial, at, at);
    assert_eq!(finding.outcome, Outcome::Failed);
    assert_eq!(finding.code, "crl.issuer.signature_invalid");
}

#[test]
fn status_reports_a_revocation_after_the_interval_as_checked() {
    let at = gen_time();
    let mut later = crl(ROOT_CRL);
    later.revoked[0].1 = at + 60;
    let finding = status(&later, &cert(ROOT), &cert(REVOKED_HOLDER).serial, at, at);
    assert_eq!(finding.outcome, Outcome::Checked, "{finding:?}");
    assert_eq!(finding.code, "crl.revoked_later");
    assert!(
        finding.detail.contains(&(at + 60).to_string()),
        "{finding:?}"
    );
}

#[test]
fn status_fails_for_a_serial_listed_before_the_interval_even_without_coverage() {
    let parsed = crl(ROOT_CRL);
    let (start, end) = (parsed.this_update, parsed.next_update);
    let revocation = crl_fact("root.crl.der", "revocation");
    let root = cert(ROOT);
    let serial = cert(REVOKED_HOLDER).serial;
    for (from, to) in [(start - 1, start), (end, end + 1), (end + 1, end + 2)] {
        let finding = status(&parsed, &root, &serial, from, to);
        assert_eq!(finding.outcome, Outcome::Failed, "{finding:?}");
        assert_eq!(finding.code, "crl.revoked", "{finding:?}");
        assert!(
            finding.detail.contains(&revocation.to_string()),
            "{finding:?}"
        );
    }
}

#[test]
fn status_names_a_later_revocation_when_the_crl_does_not_cover() {
    let parsed = crl(ROOT_CRL);
    let revocation = crl_fact("root.crl.der", "revocation");
    let serial = cert(REVOKED_HOLDER).serial;
    let at = revocation - 1;
    let finding = status(&parsed, &cert(ROOT), &serial, at, at);
    assert_eq!(finding.outcome, Outcome::Indeterminate, "{finding:?}");
    assert_eq!(finding.code, "crl.not_covering");
    assert!(
        finding.detail.contains(&revocation.to_string()),
        "{finding:?}"
    );
}

#[test]
fn parse_rejects_nesting_beyond_the_bound_as_malformed() {
    let mut nested: Vec<u8> = Vec::new();
    for _ in 0..crate::der_walk::MAX_DEPTH + 1 {
        let mut outer = vec![0x30, u8::try_from(nested.len()).unwrap()];
        outer.extend_from_slice(&nested);
        nested = outer;
    }
    let finding = parse(&nested).unwrap_err();
    assert_eq!(finding.outcome, Outcome::Failed, "{finding:?}");
    assert_eq!(finding.code, "crl.malformed", "{finding:?}");
    assert_eq!(finding.detail, "der.too_deep", "{finding:?}");
}
