use super::{
    encode, parse, AccountRole, ContractArtifact, ContractManifest, ContractRole, ContractSigner,
    PARTY_ENTRIES,
};
use crate::json::{self, Limits, Value};
use crate::report::Outcome;

const LIMITS: Limits = Limits { max_values: 1000 };

fn signer(position: u8, account_role: AccountRole) -> ContractSigner {
    ContractSigner {
        account_id: format!("0000000{position}-0000-4000-8000-00000000000a"),
        account_role,
        binding_id: format!("0000000{position}-0000-4000-8000-00000000000b"),
        binding_registered_at: 1_800_000_000 + i64::from(position),
        operation_id: format!("0000000{position}-0000-4000-8000-00000000000c"),
    }
}

/// Every role of every party it belongs to, with distinct content.
fn artifacts() -> Vec<ContractArtifact> {
    let mut rows = Vec::new();
    for role in ContractRole::ALL {
        for party in 0..=2u8 {
            if role.entry(party).is_some() {
                let content = format!("{}-{party}", role.as_str());
                rows.push(ContractManifest::artifact(party, role, content.as_bytes()));
            }
        }
    }
    rows
}

fn manifest() -> ContractManifest {
    ContractManifest {
        deployment_id: "10000000-0000-4000-8000-000000000001".to_owned(),
        case_id: "10000000-0000-4000-8000-000000000002".to_owned(),
        request_id: "10000000-0000-4000-8000-000000000003".to_owned(),
        document_id: "10000000-0000-4000-8000-000000000004".to_owned(),
        document_version: 3,
        document_sha256: [0x11; 32],
        request_sha256: [0x22; 32],
        expires_at: 1_800_600_000,
        artifacts: artifacts(),
        signers: [
            signer(1, AccountRole::Client),
            signer(2, AccountRole::Litigator),
        ],
    }
}

fn encoded() -> Vec<u8> {
    encode(&manifest()).expect("encodes")
}

/// The encoded manifest after `edit` changes its parsed value, serialized
/// canonically so that only the edited rule is broken.
fn edited(edit: impl FnOnce(&mut Vec<(String, Value)>)) -> Vec<u8> {
    let Value::Object(mut members) = json::parse(&encoded(), LIMITS).expect("encoded") else {
        panic!("an object");
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

fn failed(code: &str, bytes: &[u8]) {
    let finding = parse(bytes).expect_err("rejected");
    assert_eq!(
        (finding.outcome, finding.code),
        (Outcome::Failed, code),
        "{}",
        finding.detail
    );
}

fn schema(bytes: &[u8]) {
    failed("manifest.contract_schema", bytes);
}

#[test]
fn a_full_manifest_round_trips_with_sorted_artifacts() {
    let bytes = encoded();
    let parsed = parse(&bytes).expect("parses");
    assert_eq!(parsed.artifacts.len(), 23);
    let mut expected = manifest();
    expected
        .artifacts
        .sort_by(|left, right| left.entry.as_bytes().cmp(right.entry.as_bytes()));
    assert_eq!(parsed, expected);
    let text = String::from_utf8(bytes).expect("ASCII");
    for fixed in [
        r#""profile":"qadra-evidence-manifest""#,
        r#""profile_version":2"#,
        r#""purpose":"bilateral-contract-signing""#,
        r#""id":"signer-1""#,
        r#""side":"firm""#,
        r#""account_role":"litigator""#,
        r#""statement_signature":"parte-2-declaracion.sig""#,
    ] {
        assert!(text.contains(fixed), "{fixed}");
    }
}

#[test]
fn each_role_names_one_entry_per_party() {
    assert_eq!(ContractRole::Document.entry(0), Some("contrato.pdf"));
    assert_eq!(ContractRole::Document.entry(1), None);
    assert_eq!(
        ContractRole::Statement.entry(2),
        Some("parte-2-declaracion.bin")
    );
    assert_eq!(ContractRole::Statement.entry(0), None);
    assert_eq!(
        ContractRole::StatementTimestamp.entry(1),
        Some("parte-1-declaracion.sig.tsr")
    );
    assert_eq!(ContractRole::StatementTimestamp.entry(2), None);
    assert_eq!(ContractRole::BindingCrl.entry(3), None);
    assert_eq!(PARTY_ENTRIES[1].crl, "parte-2-crl.pem");
    for role in ContractRole::ALL {
        assert_eq!(ContractRole::parse(role.as_str()), Some(role));
    }
}

#[test]
fn encode_refuses_what_parse_rejects() {
    let mut wrong = manifest();
    wrong.signers[1].account_role = AccountRole::Client;
    assert_eq!(
        encode(&wrong).expect_err("rejected").code,
        "manifest.contract_schema"
    );
    let mut wrong = manifest();
    wrong.document_version = 0;
    assert!(encode(&wrong).is_err());
}

#[test]
fn rejects_another_version_purpose_or_fixed_member() {
    schema(&edited(|top| {
        *member(top, "profile_version") = Value::Integer(1);
    }));
    schema(&edited(|top| {
        *member(top, "purpose") = Value::String("individual-document-signing".to_owned());
    }));
    schema(&edited(|top| {
        *member(object(member(top, "algorithms")), "digest") = Value::String("sha-1".to_owned());
    }));
    schema(&edited(|top| {
        top.retain(|(key, _)| key != "request_sha256");
    }));
}

#[test]
fn requires_exactly_two_signers_in_position_order() {
    schema(&edited(|top| {
        array(member(top, "signers")).pop();
    }));
    schema(&edited(|top| {
        let signers = array(member(top, "signers"));
        let extra = signers[1].clone();
        signers.push(extra);
    }));
    schema(&edited(|top| {
        array(member(top, "signers")).swap(0, 1);
    }));
    schema(&edited(|top| {
        let signers = array(member(top, "signers"));
        *member(object(&mut signers[0]), "side") = Value::String("firm".to_owned());
    }));
    schema(&edited(|top| {
        let signers = array(member(top, "signers"));
        *member(object(&mut signers[1]), "account_role") = Value::String("paralegal".to_owned());
    }));
    schema(&edited(|top| {
        let signers = array(member(top, "signers"));
        let entries = object(member(object(&mut signers[0]), "entries"));
        *member(entries, "issuer_crl") = Value::String("parte-2-crl.pem".to_owned());
    }));
}

#[test]
fn rejects_artifacts_outside_their_party_and_role() {
    let artifact_member = |index: usize, key: &'static str, value: Value| {
        edited(move |top| {
            let artifacts = array(member(top, "artifacts"));
            *member(object(&mut artifacts[index]), key) = value;
        })
    };
    // Index 0 is ca.pem, the first entry in byte order.
    schema(&artifact_member(0, "party", Value::Integer(1)));
    schema(&artifact_member(0, "party", Value::Integer(3)));
    schema(&artifact_member(
        0,
        "role",
        Value::String("tsa-chain".to_owned()),
    ));
    schema(&artifact_member(
        0,
        "role",
        Value::String("unknown".to_owned()),
    ));
    failed(
        "manifest.artifacts_unsorted",
        &edited(|top| array(member(top, "artifacts")).swap(0, 1)),
    );
    failed(
        "manifest.duplicate_artifact",
        &edited(|top| {
            let artifacts = array(member(top, "artifacts"));
            let first = artifacts[0].clone();
            artifacts.insert(0, first);
        }),
    );
}

#[test]
fn the_individual_signing_parser_and_this_one_reject_each_other() {
    let individual = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/manifiesto.json"
    ));
    schema(individual);
    let finding = crate::manifest::parse(&encoded()).expect_err("v1 parser");
    assert_eq!(finding.code, "manifest.schema");
}

#[test]
fn rejects_non_canonical_bytes() {
    let mut bytes = encoded();
    bytes.push(b'\n');
    failed("json.not_canonical", &bytes);
}
