#[allow(dead_code)]
mod legacy_database_support;
use legacy_database_support::{Database, Source};

#[test]
fn initial_import_rejects_result_roots_when_other_history_is_empty() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let source = Source::new();
    db.seed_case(source.case_id);
    db.client
        .batch_execute("ALTER TABLE case_hearing_results DISABLE TRIGGER ALL")
        .unwrap();
    db.client
        .execute(
            "INSERT INTO case_hearing_results(id,case_id,hearing_id,anchor_revision,
         anchor_values_digest,anchor_submission_digest) VALUES($1,$2,$3,1,$4,$4)",
            &[
                &uuid::Uuid::new_v4(),
                &source.case_id.as_uuid(),
                &uuid::Uuid::new_v4(),
                &&[0_u8; 32][..],
            ],
        )
        .unwrap();
    db.client
        .batch_execute("ALTER TABLE case_hearing_results ENABLE TRIGGER ALL")
        .unwrap();
    let before = db.stored_state();
    assert!(source.inspect().check_target(&db.url).is_err());
    assert!(source.inspect().apply(&db.url).is_err());
    assert_eq!(db.stored_state(), before);
    source.assert_unmarked();
}
