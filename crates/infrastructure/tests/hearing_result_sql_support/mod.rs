#![allow(dead_code)]
use super::hearing_result_database_support::Fixture;
use application::hearing_results::*;
use domain::crypto::DocumentHasher;
use infrastructure::RingSha256Hasher;
use serde_json::{json, Value};

pub fn source(db: &mut Fixture, id: HearingResultId, root: bool) -> Value {
    let sql = if root {
        "SELECT to_jsonb(r) FROM case_hearing_results r WHERE id=$1"
    } else {
        "SELECT to_jsonb(r) FROM case_hearing_result_revisions r WHERE result_id=$1 ORDER BY revision DESC LIMIT 1"
    };
    db.admin.query_one(sql, &[&id.as_uuid()]).unwrap().get(0)
}
pub fn row(
    db: &Fixture,
    original: &Value,
    base: &HearingResultDetail,
    command: &HearingResultCommand,
    values: &HearingResultValues,
) -> Value {
    let mut row = original.clone();
    let canonical = values.canonical_bytes();
    let digest = RingSha256Hasher.hash_bytes(&canonical);
    let receipt = hearing_result_submission_bytes(
        db.owner,
        db.case,
        command,
        &base.snapshot.anchor,
        base.snapshot.continuation.as_ref(),
        digest,
    );
    for (key, value) in [
        ("result_id", json!(command.result_id.to_string())),
        ("hearing_id", json!(command.hearing_id.to_string())),
        ("operation_id", json!(command.operation_id.to_string())),
        ("action", json!(command.action().as_str())),
        ("revision", json!(command.result_revision().unwrap().get())),
        ("reason", json!(command.reason().map(|n| n.as_str()))),
        ("values_canonical", json!(format!("\\x{}", hex(&canonical)))),
        ("values_digest", json!(format!("\\x{}", digest.to_hex()))),
        (
            "submission_canonical",
            json!(format!("\\x{}", hex(&receipt))),
        ),
        (
            "submission_digest",
            json!(format!(
                "\\x{}",
                RingSha256Hasher.hash_bytes(&receipt).to_hex()
            )),
        ),
    ] {
        row[key] = value;
    }
    row
}
pub fn insert(
    client: &mut postgres::Client,
    schema: &str,
    value: &Value,
    root: bool,
) -> Result<u64, postgres::Error> {
    let table = if root {
        "case_hearing_results"
    } else {
        "case_hearing_result_revisions"
    };
    let relation = format!("{schema}.{table}");
    let columns: String = client.query_one("SELECT string_agg(quote_ident(attname),',' ORDER BY attnum) FROM pg_attribute WHERE attrelid=$1::text::regclass AND attnum>0 AND NOT attisdropped AND attgenerated=''", &[&relation]).unwrap().get(0);
    client.execute(&format!("INSERT INTO {relation}({columns}) SELECT {columns} FROM jsonb_populate_record(NULL::{relation},$1)"), &[value])
}
pub fn correction(first: &HearingResultDetail) -> HearingResultCommand {
    HearingResultCommand {
        operation_id: HearingResultOperationId::new(),
        hearing_id: first.snapshot.hearing_id,
        result_id: first.snapshot.id,
        change: HearingResultChange::Correct {
            expected_revision: first.snapshot.revision,
            values: first.snapshot.values.clone(),
            reason: HearingResultText::new("Correct declaration").unwrap(),
        },
    }
}
pub fn seed(db: &mut Fixture) -> HearingResultDetail {
    super::hearing_database_support::complete(db);
    let hearing = super::hearing_database_support::persist(
        &super::hearing_database_support::service(db, db.owner, domain::identity::Role::Owner),
        db.case,
        super::hearing_database_support::schedule(),
    );
    super::hearing_result_database_support::persist(
        &super::hearing_result_database_support::service(
            db,
            db.owner,
            domain::identity::Role::Owner,
        ),
        db.case,
        super::hearing_result_database_support::record(hearing.snapshot.id),
    )
}
pub fn reject(db: &Fixture, row: &Value, root: bool) {
    let error = insert(&mut db.runtime(), &db.schema, row, root).unwrap_err();
    assert!(
        matches!(error.code().map(|c| c.code()), Some("23514" | "42501")),
        "{error:?}"
    );
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
