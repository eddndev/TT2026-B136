use super::*;
use application::ApplicationError;
use domain::{
    audit::{chain_digest, AuditEvent},
    crypto::{DocumentHasher, Sha256Digest},
};
use postgres::{error::SqlState, Transaction};
use serde_json::{json, Value};

const ROOT: &str = "case_precautionary_hearings";
const REVISIONS: &str = "case_precautionary_hearing_revisions";

fn open(db: &Fixture) -> Result<PostgresPrecautionaryHearingStore, ApplicationError> {
    PostgresPrecautionaryHearingStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

#[test]
fn repeated_migration_preserves_hearing_rows_and_catalog_identity() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    persist(&db, actor, command);
    let before = snapshot(&mut db);
    let catalog = |db: &mut Fixture| -> Value {
        db.admin.query_one(
            "SELECT jsonb_agg(jsonb_build_array(oid,conname,pg_get_constraintdef(oid)) ORDER BY oid)
             FROM pg_constraint WHERE conrelid IN ($1::text::regclass,$2::text::regclass)",
            &[&ROOT, &REVISIONS],
        ).unwrap().get(0)
    };
    let constraints = catalog(&mut db);
    db.migrate();
    assert_eq!(catalog(&mut db), constraints);
    assert_eq!(snapshot(&mut db), before);
    open(&db).unwrap();
    for table in [ROOT, REVISIONS] {
        for statement in [
            format!("UPDATE {table} SET case_id=case_id"),
            format!("DELETE FROM {table}"),
            format!("TRUNCATE {table} CASCADE"),
        ] {
            assert_eq!(
                db.admin.batch_execute(&statement).unwrap_err().code(),
                Some(&SqlState::CHECK_VIOLATION)
            );
            assert_eq!(
                db.runtime().batch_execute(&statement).unwrap_err().code(),
                Some(&SqlState::INSUFFICIENT_PRIVILEGE)
            );
        }
    }
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn startup_rejects_changed_hearing_columns_checks_indexes_and_guard_bodies() {
    let Some(mut db) = Fixture::new() else { return };
    open(&db).unwrap();
    for (damage, repair) in [
        (format!("ALTER TABLE {ROOT} ADD COLUMN extra boolean"), format!("ALTER TABLE {ROOT} DROP COLUMN extra")),
        (format!("ALTER TABLE {ROOT} ALTER COLUMN initial_revision SET DEFAULT 2"), format!("ALTER TABLE {ROOT} ALTER COLUMN initial_revision SET DEFAULT 1")),
        (format!("ALTER TABLE {REVISIONS} DISABLE TRIGGER USER"), format!("ALTER TABLE {REVISIONS} ENABLE TRIGGER USER")),
        ("ALTER INDEX precautionary_hearing_case_order RENAME TO changed_hearing_order".into(), "ALTER INDEX changed_hearing_order RENAME TO precautionary_hearing_case_order".into()),
        (format!("ALTER TABLE {ROOT} DROP CONSTRAINT precautionary_hearing_initial"), format!("ALTER TABLE {ROOT} ADD CONSTRAINT precautionary_hearing_initial CHECK(initial_revision=1)")),
    ] {
        db.admin.batch_execute(&damage).unwrap();
        assert!(open(&db).is_err(), "accepted altered catalog: {damage}");
        db.admin.batch_execute(&repair).unwrap();
        open(&db).unwrap();
    }
    let definition: String = db
        .admin
        .query_one(
            "SELECT pg_get_functiondef('enforce_precautionary_hearing_sequence()'::regprocedure)",
            &[],
        )
        .unwrap()
        .get(0);
    db.admin.batch_execute("CREATE OR REPLACE FUNCTION enforce_precautionary_hearing_sequence()
        RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RETURN NEW; END; $$").unwrap();
    assert!(open(&db).is_err());
    db.admin.batch_execute(&definition).unwrap();
    open(&db).unwrap();
}

#[test]
fn runtime_requires_exact_nondelegatable_column_insert_authority() {
    let Some(mut db) = Fixture::new() else { return };
    for table in [ROOT, REVISIONS] {
        let grants = db
            .admin
            .query_one(
                "SELECT has_table_privilege($1,$2,'SELECT'),has_table_privilege($1,$2,'INSERT'),
             has_column_privilege($1,$2,'case_id','INSERT')",
                &[&db.role, &table],
            )
            .unwrap();
        assert!(grants.get::<_, bool>(0));
        assert!(!grants.get::<_, bool>(1));
        assert!(grants.get::<_, bool>(2));
        let columns: String = db
            .admin
            .query_one(
                "SELECT string_agg(quote_ident(attname),',' ORDER BY attnum)
             FROM pg_attribute WHERE attrelid=$1::text::regclass AND attnum>0 AND NOT attisdropped",
                &[&table],
            )
            .unwrap()
            .get(0);
        for (damage, repair) in [
            (
                format!("GRANT SELECT ON {table} TO PUBLIC"),
                format!("REVOKE SELECT ON {table} FROM PUBLIC"),
            ),
            (
                format!("GRANT UPDATE(case_id) ON {table} TO {}", db.role),
                format!("REVOKE UPDATE(case_id) ON {table} FROM {}", db.role),
            ),
            (
                format!("GRANT INSERT ON {table} TO {}", db.role),
                format!(
                    "REVOKE INSERT ON {table} FROM {}; GRANT INSERT({columns}) ON {table} TO {}",
                    db.role, db.role
                ),
            ),
            (
                format!(
                    "GRANT INSERT(case_id) ON {table} TO {} WITH GRANT OPTION",
                    db.role
                ),
                format!(
                    "REVOKE GRANT OPTION FOR INSERT(case_id) ON {table} FROM {}",
                    db.role
                ),
            ),
        ] {
            db.admin.batch_execute(&damage).unwrap();
            assert!(open(&db).is_err(), "accepted excessive authority: {damage}");
            db.admin.batch_execute(&repair).unwrap();
            assert!(
                open(&db).is_ok(),
                "failed to restore original grants after: {damage}"
            );
        }
    }
    db.admin
        .batch_execute(
            "GRANT EXECUTE ON FUNCTION enforce_precautionary_hearing_sequence() TO PUBLIC",
        )
        .unwrap();
    assert!(open(&db).is_err());
    db.admin
        .batch_execute(
            "REVOKE EXECUTE ON FUNCTION enforce_precautionary_hearing_sequence() FROM PUBLIC",
        )
        .unwrap();
    open(&db).unwrap();
}

#[test]
fn direct_append_rejects_wrong_sequence_actor_or_support_and_root_without_r1() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let first = persist(&db, actor.clone(), command);
    let command = replacement(&first);
    let capture = prepare_precautionary_hearing_capture(
        &RingSha256Hasher,
        &actor,
        db.case,
        command,
        first.capture.review.observed_context.clone(),
        first.capture.review.sources.clone(),
        Some(&first.capture),
    )
    .unwrap()
    .into_capture(&RingSha256Hasher, db.at)
    .unwrap();
    let row = replacement_row(&mut db, &capture);
    let before = snapshot(&mut db);
    let mut runtime = db.runtime();
    for damage in 0..4 {
        let mut row = row.clone();
        match damage {
            1 => row["revision"] = json!(3),
            2 => row["recorded_by_role"] = json!("litigator"),
            3 => {
                row["values_view"]["scheduling_basis"]["document_id"] = json!(uuid::Uuid::new_v4())
            }
            _ => {}
        }
        let mut tx = runtime.transaction().unwrap();
        append_audit(&mut tx, &mut row, &capture);
        let result = tx.execute(
            "INSERT INTO case_precautionary_hearing_revisions
             SELECT * FROM jsonb_populate_record(NULL::case_precautionary_hearing_revisions,$1)",
            &[&row],
        );
        if damage == 0 {
            assert_eq!(
                result.unwrap(),
                1,
                "unchanged direct append is the positive control"
            );
        } else {
            let expected = if damage == 2 {
                &SqlState::INSUFFICIENT_PRIVILEGE
            } else {
                &SqlState::CHECK_VIOLATION
            };
            assert_eq!(result.unwrap_err().code(), Some(expected));
        }
        tx.rollback().unwrap();
        assert_eq!(snapshot(&mut db), before);
    }
    let mut tx = runtime.transaction().unwrap();
    tx.execute(
        "INSERT INTO case_precautionary_hearings(id,case_id) VALUES($1,$2)",
        &[&uuid::Uuid::new_v4(), &db.case.as_uuid()],
    )
    .unwrap();
    assert_eq!(
        tx.commit().unwrap_err().code(),
        Some(&SqlState::FOREIGN_KEY_VIOLATION)
    );
    assert_eq!(snapshot(&mut db), before);
}

fn replacement_row(db: &mut Fixture, capture: &PrecautionaryHearingCapture) -> Value {
    let review = &capture.review;
    let mut row: Value = db.admin.query_one(
        "SELECT to_jsonb(r) FROM case_precautionary_hearing_revisions r WHERE hearing_id=$1 AND revision=1",
        &[&review.command.hearing_id.as_uuid()],
    ).unwrap().get(0);
    let PrecautionaryHearingChange::Replace {
        expected_capture_digest,
        reason,
        values,
        ..
    } = &review.command.change
    else {
        unreachable!()
    };
    row["revision"] = json!(review.result_revision.get());
    row["operation_id"] = json!(review.command.operation_id.as_uuid());
    row["action"] = json!("replace");
    row["previous_capture_digest"] = json!(format!("\\x{}", expected_capture_digest.to_hex()));
    row["reason"] = json!(reason.as_str());
    let bytes = values.canonical_bytes();
    row["values_canonical"] = json!(format!(
        "\\x{}",
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ));
    row["values_view"] = infrastructure::precautionary_hearing_codec::view(values);
    row["values_digest"] = json!(format!(
        "\\x{}",
        RingSha256Hasher.hash_bytes(&bytes).to_hex()
    ));
    for (name, digest) in [
        ("submission_digest", review.submission_digest),
        ("review_digest", review.review_digest),
        ("capture_digest", capture.capture_digest),
    ] {
        row[name] = json!(format!("\\x{}", digest.to_hex()));
    }
    row
}

fn append_audit(tx: &mut Transaction<'_>, row: &mut Value, capture: &PrecautionaryHearingCapture) {
    let previous = tx
        .query_one(
            "SELECT sequence,chain FROM audit_events ORDER BY sequence DESC LIMIT 1",
            &[],
        )
        .unwrap();
    let sequence = previous.get::<_, i64>(0) + 1;
    let chain = Sha256Digest::from_bytes(&previous.get::<_, Vec<u8>>(1)).unwrap();
    let review = &capture.review;
    let marker = format!(
        "ph1:case:{}:hearing:{}:operation:{}:revision:{}:submission:{}:review:{}:capture:{}",
        review.case_id,
        review.command.hearing_id,
        review.command.operation_id,
        row["revision"],
        review.submission_digest.to_hex(),
        review.review_digest.to_hex(),
        capture.capture_digest.to_hex()
    );
    let event = AuditEvent::new(
        sequence as u64,
        capture.recorded_at,
        &review.actor.email,
        "precautionary_hearing.replace",
        marker,
    );
    let digest = chain_digest(&RingSha256Hasher, &chain, &event).unwrap();
    tx.execute("INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain) VALUES($1,$2,$3,$4,$5,$6)",
        &[&sequence, &event.timestamp_rfc3339().unwrap(), &event.actor, &event.action, &event.resource, &digest.as_bytes().as_slice()]).unwrap();
    row["audit_sequence"] = json!(sequence);
}
