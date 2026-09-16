use super::hearing_result_database_support::Fixture;
use application::hearing_results::*;
use domain::crypto::DocumentHasher;
use infrastructure::RingSha256Hasher;
use serde_json::{json, Value};

pub fn forged_revision(
    db: &mut Fixture,
    base: &HearingResultDetail,
    command: &HearingResultCommand,
    values: &HearingResultValues,
) -> Value {
    let mut row: Value = db
        .admin
        .query_one(
            "SELECT to_jsonb(r) FROM case_hearing_result_revisions r
         WHERE result_id=$1 AND revision=$2",
            &[
                &base.snapshot.id.as_uuid(),
                &i64::from(base.snapshot.revision.get()),
            ],
        )
        .unwrap()
        .get(0);
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
        ("revision", json!(command.result_revision().unwrap().get())),
        ("operation_id", json!(command.operation_id.to_string())),
        ("action", json!(command.action().as_str())),
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

pub fn insert(db: &mut Fixture, value: &Value) {
    let columns: String = db
        .admin
        .query_one(
            "SELECT string_agg(quote_ident(attname),',' ORDER BY attnum) FROM pg_attribute
         WHERE attrelid='case_hearing_result_revisions'::regclass AND attnum>0
         AND NOT attisdropped AND attgenerated=''",
            &[],
        )
        .unwrap()
        .get(0);
    db.admin
        .execute(
            &format!(
                "INSERT INTO case_hearing_result_revisions({columns})
         SELECT {columns} FROM jsonb_populate_record(NULL::case_hearing_result_revisions,$1)"
            ),
            &[value],
        )
        .unwrap();
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
