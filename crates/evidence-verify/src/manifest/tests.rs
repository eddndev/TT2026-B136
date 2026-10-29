use super::{encode, parse, role_entry, Artifact, Manifest, Role};
use crate::certificate::sha256;
use crate::json::{self, Limits, Value};
use crate::report::Outcome;

const FIXTURE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/manifiesto.json"
));

const LIMITS: Limits = Limits { max_values: 1000 };

/// The fixture after `edit` changes its parsed value, serialized
/// canonically so that only the edited rule is broken.
fn edited(edit: impl FnOnce(&mut Vec<(String, Value)>)) -> Vec<u8> {
    let Value::Object(mut members) = json::parse(FIXTURE, LIMITS).expect("fixture") else {
        panic!("fixture is an object");
    };
    edit(&mut members);
    json::canonical(&Value::Object(members)).expect("canonical")
}

fn member<'a>(members: &'a mut [(String, Value)], key: &str) -> &'a mut Value {
    &mut members
        .iter_mut()
        .find(|(name, _)| name == key)
        .expect("member")
        .1
}

fn array(value: &mut Value) -> &mut Vec<Value> {
    match value {
        Value::Array(items) => items,
        _ => panic!("array"),
    }
}

fn object(value: &mut Value) -> &mut Vec<(String, Value)> {
    match value {
        Value::Object(members) => members,
        _ => panic!("object"),
    }
}

fn rejected(bytes: &[u8]) -> (Outcome, &'static str) {
    let finding = parse(bytes).expect_err("rejected");
    (finding.outcome, finding.code)
}

fn failed(code: &str, bytes: &[u8]) {
    assert_eq!(rejected(bytes), (Outcome::Failed, code), "{code}");
}

#[test]
fn the_fixture_round_trips_byte_for_byte() {
    let manifest = parse(FIXTURE).expect("fixture parses");
    assert_eq!(encode(&manifest).expect("encodes"), FIXTURE);
    assert_eq!(manifest.artifacts.len(), Role::ALL.len());
    assert_eq!(manifest.signers.len(), 1);
    assert_eq!(manifest.signers[0].id, "signer-1");
    assert_eq!(manifest.document_version, 1);
    let document = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.role == Role::Document)
        .expect("document artifact");
    assert_eq!(document.sha256, manifest.document_sha256);
    assert_eq!(document.entry, role_entry(Role::Document, &document.entry));
}

#[test]
fn encode_sorts_the_artifacts_by_entry_name() {
    let mut manifest = parse(FIXTURE).expect("fixture");
    manifest.artifacts.reverse();
    assert_eq!(encode(&manifest).expect("encodes"), FIXTURE);
}

#[test]
fn artifact_builds_the_row_of_one_role() {
    let row = Manifest::artifact("acta.pdf", Role::DocumentTimestamp, b"token");
    assert_eq!(
        row,
        Artifact {
            entry: "acta.pdf.tsr".to_owned(),
            role: Role::DocumentTimestamp,
            sha256: sha256(b"token"),
            size: 5,
        }
    );
}

#[test]
fn every_role_has_one_name_and_one_entry() {
    let mut entries: Vec<String> = Role::ALL
        .iter()
        .map(|role| role_entry(*role, "acta.pdf"))
        .collect();
    for role in Role::ALL {
        assert_eq!(Role::parse(role.as_str()), Some(role));
    }
    assert_eq!(Role::parse("manifest"), None);
    entries.sort();
    entries.dedup();
    assert_eq!(entries.len(), Role::ALL.len());
    assert_eq!(role_entry(Role::TsaIssuerCrl, "acta.pdf"), "tsa-crl.pem");
    assert_eq!(role_entry(Role::DocumentSignature, "x"), "x.sig");
}

#[test]
fn non_canonical_bytes_are_rejected() {
    let mut newline = FIXTURE.to_vec();
    newline.push(b'\n');
    failed("json.not_canonical", &newline);
    let text = std::str::from_utf8(FIXTURE).expect("ASCII");
    let reordered = text.replacen(
        r#""digest":"sha-256","signature":"rsa-pkcs1v15-sha256""#,
        r#""signature":"rsa-pkcs1v15-sha256","digest":"sha-256""#,
        1,
    );
    assert_ne!(reordered, text);
    failed("json.not_canonical", reordered.as_bytes());
}

#[test]
fn unsorted_and_duplicate_artifacts_are_rejected() {
    let unsorted = edited(|members| array(member(members, "artifacts")).swap(0, 1));
    failed("manifest.artifacts_unsorted", &unsorted);
    let duplicate = edited(|members| {
        let artifacts = array(member(members, "artifacts"));
        artifacts[1] = artifacts[0].clone();
    });
    failed("manifest.duplicate_artifact", &duplicate);
}

#[test]
fn unknown_and_missing_keys_are_rejected_at_every_level() {
    let top = edited(|members| members.push(("extra".into(), Value::Integer(1))));
    failed("manifest.schema", &top);
    let missing = edited(|members| members.retain(|(key, _)| key != "case_id"));
    failed("manifest.schema", &missing);
    let in_artifact = edited(|members| {
        let artifacts = array(member(members, "artifacts"));
        object(&mut artifacts[0]).push(("extra".into(), Value::Integer(1)));
    });
    failed("manifest.schema", &in_artifact);
    let in_entries = edited(|members| {
        let signers = array(member(members, "signers"));
        let entries = member(object(&mut signers[0]), "entries");
        object(entries).retain(|(key, _)| key != "binding_crl");
    });
    failed("manifest.schema", &in_entries);
    let in_algorithms = edited(|members| {
        let algorithms = object(member(members, "algorithms"));
        *member(algorithms, "digest") = Value::String("sha-1".into());
    });
    failed("manifest.schema", &in_algorithms);
}

#[test]
fn a_second_signer_is_not_supported() {
    let second = edited(|members| {
        let signers = array(member(members, "signers"));
        let mut other = signers[0].clone();
        *member(object(&mut other), "id") = Value::String("signer-2".into());
        signers.push(other);
    });
    assert_eq!(
        rejected(&second),
        (Outcome::NotSupported, "manifest.signers_not_supported")
    );
    let renamed = edited(|members| {
        let signers = array(member(members, "signers"));
        *member(object(&mut signers[0]), "id") = Value::String("signer-2".into());
    });
    failed("manifest.schema", &renamed);
    let none = edited(|members| array(member(members, "signers")).clear());
    failed("manifest.schema", &none);
}

#[test]
fn a_second_signer_does_not_hide_a_wrong_profile_or_algorithm() {
    let with_second = |members: &mut Vec<(String, Value)>| {
        let signers = array(member(members, "signers"));
        let mut other = signers[0].clone();
        *member(object(&mut other), "id") = Value::String("signer-2".into());
        signers.push(other);
    };
    let profile = edited(|members| {
        with_second(members);
        *member(members, "profile") = Value::String("other".into());
    });
    failed("manifest.schema", &profile);
    let version = edited(|members| {
        with_second(members);
        *member(members, "profile_version") = Value::Integer(2);
    });
    failed("manifest.schema", &version);
    let purpose = edited(|members| {
        with_second(members);
        *member(members, "purpose") = Value::String("other".into());
    });
    failed("manifest.schema", &purpose);
    let algorithm = edited(|members| {
        with_second(members);
        let algorithms = object(member(members, "algorithms"));
        *member(algorithms, "digest") = Value::String("sha-512".into());
    });
    failed("manifest.schema", &algorithm);
}

#[test]
fn fixed_values_digests_and_identifiers_are_strict() {
    let upper = edited(|members| {
        let Value::String(digest) = member(members, "document_sha256") else {
            panic!("string");
        };
        *digest = digest.to_uppercase();
    });
    failed("manifest.schema", &upper);
    let short = edited(|members| {
        *member(members, "document_sha256") = Value::String("00".into());
    });
    failed("manifest.schema", &short);
    let uuid = edited(|members| {
        let Value::String(id) = member(members, "case_id") else {
            panic!("string");
        };
        *id = id.to_uppercase();
    });
    failed("manifest.schema", &uuid);
    let version = edited(|members| *member(members, "profile_version") = Value::Integer(2));
    failed("manifest.schema", &version);
    let profile = edited(|members| {
        *member(members, "profile") = Value::String("other".into());
    });
    failed("manifest.schema", &profile);
    let zero = edited(|members| *member(members, "document_version") = Value::Integer(0));
    failed("manifest.schema", &zero);
    let role = edited(|members| {
        let artifacts = array(member(members, "artifacts"));
        *member(object(&mut artifacts[0]), "role") = Value::String("other".into());
    });
    failed("manifest.schema", &role);
    let text = edited(|members| {
        let artifacts = array(member(members, "artifacts"));
        *member(object(&mut artifacts[0]), "size") = Value::String("1".into());
    });
    failed("manifest.schema", &text);
}

#[test]
fn a_repeated_role_is_rejected() {
    let repeated = edited(|members| {
        let artifacts = array(member(members, "artifacts"));
        let role = member(object(&mut artifacts[0]), "role").clone();
        *member(object(&mut artifacts[1]), "role") = role;
    });
    failed("manifest.schema", &repeated);
}

#[test]
fn integers_beyond_the_canonical_range_are_rejected() {
    let text = std::str::from_utf8(FIXTURE).expect("ASCII");
    let large = text.replacen(
        r#""document_version":1"#,
        r#""document_version":9007199254740992"#,
        1,
    );
    assert_ne!(large, text);
    failed("json.integer_out_of_range", large.as_bytes());
}

#[test]
fn encode_refuses_a_manifest_the_parser_would_reject() {
    let mut manifest = parse(FIXTURE).expect("fixture");
    manifest.case_id = "not-a-uuid".to_owned();
    assert_eq!(encode(&manifest).unwrap_err().code, "manifest.schema");
    let mut manifest = parse(FIXTURE).expect("fixture");
    manifest.artifacts[0].size = 1 << 53;
    assert_eq!(
        encode(&manifest).unwrap_err().code,
        "json.integer_out_of_range"
    );
}

#[test]
fn an_unknown_member_name_reaches_the_detail_only_as_ascii() {
    let bytes = edited(|members| {
        members.push(("z\u{202e}\u{e9}\n".to_owned(), Value::Integer(1)));
    });
    let finding = parse(&bytes).expect_err("rejected");
    assert_eq!(finding.code, "manifest.schema");
    assert!(finding.detail.is_ascii(), "{:?}", finding.detail);
    assert!(finding.detail.contains("\\u{e9}"), "{:?}", finding.detail);
}
