mod case_administration_support;

use case_administration_support::Fixture;
use serde_json::json;

fn vectors() -> Vec<&'static str> {
    include_str!("../../application/tests/hearing_support/wire-vectors.txt")
        .lines()
        .collect()
}

#[test]
fn sql_decodes_independent_operation_receipts_for_every_action() {
    let Some(mut db) = Fixture::new() else { return };
    for (index, bytes) in vectors().iter().enumerate() {
        let value: serde_json::Value = db
            .admin
            .query_one("SELECT hearing_submission(decode($1,'hex'))", &[bytes])
            .unwrap()
            .get(0);
        assert_eq!(
            value,
            json!({
                "operation_id":"00000000-0000-0000-0000-000000000001",
                "actor_id":"00000000-0000-0000-0000-000000000002",
                "case_id":"00000000-0000-0000-0000-000000000003",
                "hearing_id":"00000000-0000-0000-0000-000000000004",
                "action":(["schedule","replace","cancel"][index]),
                "expected_revision":if index==0 {0} else {7},
                "expected_context":if index==2 {json!(null)} else {json!({"case_revision":11,"stage_revision":13})},
                "values_digest":"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
                "reason":if index==0 {json!(null)} else {json!("Motivo\n\u{e1}")}
            })
        );
    }
}

#[test]
fn sql_rejects_bad_receipt_tags_context_revision_reason_and_trailing_bytes() {
    let Some(mut db) = Fixture::new() else { return };
    let vectors = vectors();
    let schedule = vectors[0];
    let replace = vectors[1];
    for invalid in [
        String::new(),
        format!("{schedule}00"),
        schedule[..schedule.len() - 2].to_owned(),
        format!("{}03{}", &schedule[..138], &schedule[140..]),
        format!("{}00000001{}", &schedule[..140], &schedule[148..]),
        format!("{}00{}", &schedule[..148], &schedule[150..]),
        format!("{}00000000{}", &schedule[..150], &schedule[158..]),
        format!("{}ffffffff{}", &replace[..140], &replace[148..]),
        replace.replacen("4d6f7469766f", "206f7469766f", 1),
        "00".repeat(4121),
    ] {
        let failure = db
            .admin
            .query_one("SELECT hearing_submission(decode($1,'hex'))", &[&invalid])
            .unwrap_err();
        assert_eq!(
            failure.code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
    }
}
