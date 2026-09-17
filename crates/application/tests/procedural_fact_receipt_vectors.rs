mod procedural_fact_receipt_vector_support;
use application::{procedural_facts::*, ApplicationError};
use domain::{cases::CaseId, identity::UserId};
use procedural_fact_receipt_vector_support::*;
use serde_json::Value;

fn submission(value: &Value, summary: &str) -> Result<Vec<u8>, ApplicationError> {
    fact_submission_bytes(
        UserId::from_uuid(uuid(&value["actor_id"])),
        CaseId::from_uuid(uuid(&value["case_id"])),
        &command(value, summary),
        digest(&value["values_digest"]),
        digest(&value["sources_digest"]),
    )
}
#[test]
fn sources_match_independent_python_bytes_and_digest_inputs() {
    let fixture = fixtures();
    let vectors = fixture["sources"].as_array().unwrap();
    assert_eq!(vectors.len(), 25);
    for vector in vectors {
        let sources = sources(&vector["sources"]);
        let encoded = fact_sources_bytes(&sources).unwrap();
        assert_eq!(encoded, bytes(&vector["hex"]), "{}", vector["name"]);
        assert_eq!(encoded.len(), number(&vector["length"]) as usize);
        let hasher = OracleHasher::new(vector);
        assert_eq!(
            fact_sources_digest(&hasher, &sources).unwrap(),
            digest(&vector["sha256"])
        );
        hasher.assert_used_once();
    }
}
#[test]
fn submission_targets_actions_and_changes_match_independent_python() {
    let fixture = fixtures();
    let vectors = fixture["submissions"].as_array().unwrap();
    assert_eq!(vectors.len(), 18);
    for vector in vectors {
        let value = &vector["submission"];
        let encoded = submission(value, "Original values").unwrap();
        assert_eq!(encoded, bytes(&vector["hex"]), "{}", vector["name"]);
        assert_eq!(encoded.len(), number(&vector["length"]) as usize);
        let hasher = OracleHasher::new(vector);
        assert_eq!(
            fact_submission_digest(
                &hasher,
                UserId::from_uuid(uuid(&value["actor_id"])),
                CaseId::from_uuid(uuid(&value["case_id"])),
                &command(value, "Original values"),
                digest(&value["values_digest"]),
                digest(&value["sources_digest"]),
            )
            .unwrap(),
            digest(&vector["sha256"])
        );
        hasher.assert_used_once();
    }
}
#[test]
fn explicit_digests_determine_receipts_without_rehashing_command_values() {
    let fixture = fixtures();
    for vector in fixture["submissions"].as_array().unwrap() {
        let value = &vector["submission"];
        assert_eq!(
            submission(value, "First summary").unwrap(),
            submission(value, "Other summary").unwrap()
        );
    }
}
#[test]
fn all_bounded_extremes_have_fixed_lengths() {
    let fixture = fixtures();
    let sizes: Vec<_> = fixture["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|vector| {
            fact_sources_bytes(&sources(&vector["sources"]))
                .unwrap()
                .len()
        })
        .collect();
    assert_eq!(sizes.iter().min(), Some(&19));
    assert_eq!(sizes.iter().max(), Some(&36847));
    for (family, minimum, maximum) in [("resolution", 141, 4145), ("notification", 157, 4161)] {
        let sizes: Vec<_> = fixture["submissions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|vector| vector["submission"]["family"] == family)
            .map(|vector| submission(&vector["submission"], "Summary").unwrap().len())
            .collect();
        assert_eq!(sizes.iter().min(), Some(&minimum));
        assert_eq!(sizes.iter().max(), Some(&maximum));
    }
}
#[test]
fn both_families_reject_revision_exhaustion_for_correct_and_withdraw() {
    let fixture = fixtures();
    for vector in fixture["submissions"].as_array().unwrap() {
        let mut value = vector["submission"].clone();
        if value["action"] == "record" {
            continue;
        }
        value["expected_revision"] = Value::from(u32::MAX);
        assert!(matches!(
            submission(&value, "Summary"),
            Err(ApplicationError::ProceduralFact(
                ProceduralFactError::RevisionExhausted
            ))
        ));
    }
}
