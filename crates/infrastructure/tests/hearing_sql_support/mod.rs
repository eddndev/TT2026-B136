#![allow(dead_code)]
use super::hearing_database_support::Fixture;
use application::hearings::*;
use domain::crypto::DocumentHasher;
use infrastructure::RingSha256Hasher;
use serde_json::{json, Value};

pub fn source(db: &mut Fixture, id: HearingId) -> Value {
    db.admin.query_one("SELECT to_jsonb(r) FROM case_hearing_revisions r WHERE hearing_id=$1 ORDER BY revision DESC LIMIT 1", &[&id.as_uuid()]).unwrap().get(0)
}
pub fn row(
    db: &Fixture,
    original: &Value,
    command: &HearingCommand,
    values: &HearingValues,
) -> Value {
    let mut row = original.clone();
    let canonical = values.canonical_bytes();
    let digest = RingSha256Hasher.hash_bytes(&canonical);
    let receipt = hearing_submission_bytes(db.owner, db.case, command, digest);
    for (key, value) in [
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
) -> Result<u64, postgres::Error> {
    let columns: String = client.query_one("SELECT string_agg(quote_ident(attname),',' ORDER BY attnum) FROM pg_attribute WHERE attrelid=$1::text::regclass AND attnum>0 AND NOT attisdropped AND attgenerated=''", &[&format!("{schema}.case_hearing_revisions")]).unwrap().get(0);
    client.execute(&format!("INSERT INTO {schema}.case_hearing_revisions({columns}) SELECT {columns} FROM jsonb_populate_record(NULL::{schema}.case_hearing_revisions,$1)"), &[value])
}
pub fn replacement(first: &HearingDetail) -> HearingCommand {
    HearingCommand {
        operation_id: HearingOperationId::new(),
        hearing_id: first.snapshot.id,
        change: HearingChange::Replace {
            expected_revision: first.snapshot.revision,
            context: super::hearing_database_support::context(),
            values: first.snapshot.values.clone(),
            reason: HearingNote::new("Changed appointment").unwrap(),
        },
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
