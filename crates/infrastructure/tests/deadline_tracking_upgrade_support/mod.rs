mod legacy;
mod profile;
mod write;

pub use crate::case_administration_support::Fixture;
pub use legacy::seed_history;
use serde_json::Value;

// Explicit historical installation boundary; never use the current migrator here.
const THROUGH_0017: &[&str] = &[
    include_str!("../../../../migrations/0007_case_administration.sql"),
    include_str!("../../../../migrations/0008_case_stages.sql"),
    include_str!("../../../../migrations/0008_case_stage_values.sql"),
    include_str!("../../../../migrations/0008_case_stage_guards.sql"),
    include_str!("../../../../migrations/0009_participant_credential_trust.sql"),
    include_str!("../../../../migrations/0010_typed_values_primitives.sql"),
    include_str!("../../../../migrations/0010_typed_values.sql"),
    include_str!("../../../../migrations/0010_typed_participants.sql"),
    include_str!("../../../../migrations/0010_typed_guards.sql"),
    include_str!("../../../../migrations/0010_typed_reviews.sql"),
    include_str!("../../../../migrations/0010_typed_credentials.sql"),
    include_str!("../../../../migrations/0011_hearings_values.sql"),
    include_str!("../../../../migrations/0011_hearings_receipts.sql"),
    include_str!("../../../../migrations/0011_hearings.sql"),
    include_str!("../../../../migrations/0011_hearings_guards.sql"),
    include_str!("../../../../migrations/0012_hearing_results_time.sql"),
    include_str!("../../../../migrations/0012_hearing_results_values.sql"),
    include_str!("../../../../migrations/0012_hearing_results_receipts.sql"),
    include_str!("../../../../migrations/0012_hearing_results.sql"),
    include_str!("../../../../migrations/0012_hearing_results_guards.sql"),
    include_str!("../../../../migrations/0013_judicial_calendar_primitives.sql"),
    include_str!("../../../../migrations/0013_judicial_calendar_sources.sql"),
    include_str!("../../../../migrations/0013_judicial_calendar_values.sql"),
    include_str!("../../../../migrations/0013_judicial_calendar_receipts.sql"),
    include_str!("../../../../migrations/0013_judicial_calendar_tables.sql"),
    include_str!("../../../../migrations/0013_judicial_calendar_guards.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_primitives.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_time.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_people.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_provenance.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_values.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_source_items.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_sources.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_receipts.sql"),
    include_str!("../../../../migrations/0014_procedural_facts.sql"),
    include_str!("../../../../migrations/0014_procedural_fact_source_guards.sql"),
    include_str!("../../../../migrations/0014_procedural_facts_guards.sql"),
    include_str!("../../../../migrations/0015_deadline_source_events.sql"),
    include_str!("../../../../migrations/0016_deadline_profile_projection.sql"),
    include_str!("../../../../migrations/0016_deadline_profile_receipts.sql"),
    include_str!("../../../../migrations/0016_deadline_profile_tables.sql"),
    include_str!("../../../../migrations/0016_deadline_profile_guards.sql"),
    include_str!("../../../../migrations/0016_deadline_profile_events.sql"),
    include_str!("../../../../migrations/0017_deadline_input_selection.sql"),
    include_str!("../../../../migrations/0017_deadline_attention.sql"),
    include_str!("../../../../migrations/0017_deadline_receipts.sql"),
    include_str!("../../../../migrations/0017_deadline_tables.sql"),
    include_str!("../../../../migrations/0017_deadline_guards.sql"),
];
pub const NEW_COLUMNS: [&str; 4] = [
    "tracking_canonical",
    "observations_canonical",
    "tracking_administration_revision",
    "cause_event_sequence",
];

pub fn old_deadlines() -> Option<Fixture> {
    let mut db = Fixture::old()?;
    let mut tx = db.admin.transaction().unwrap();
    for sql in THROUGH_0017 {
        tx.batch_execute(sql).unwrap();
    }
    tx.commit().unwrap();
    let actual: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM pg_attribute WHERE attrelid='case_deadline_revisions'::regclass
        AND NOT attisdropped AND attname::text=ANY($1)",
            &[&&NEW_COLUMNS[..]],
        )
        .unwrap()
        .get(0);
    assert_eq!(actual, 0, "fixture must really precede tracking columns");
    let absent: bool = db
        .admin
        .query_one(
            "SELECT to_regprocedure('deadline_submission_v2(bytea)') IS NULL",
            &[],
        )
        .unwrap()
        .get(0);
    assert!(
        absent,
        "fixture must not install the tracked receipt parser"
    );
    Some(db)
}

pub fn assert_columns(db: &mut Fixture) {
    let rows = db
        .admin
        .query(
            "SELECT attname::text,format_type(atttypid,atttypmod),attnotnull,attgenerated::text
        FROM pg_attribute WHERE attrelid='case_deadline_revisions'::regclass
        AND NOT attisdropped AND attname::text=ANY($1) ORDER BY attname",
            &[&&NEW_COLUMNS[..]],
        )
        .unwrap();
    assert_eq!(rows.len(), 4, "upgrade must add all four tracking columns");
    for (row, (name, kind, generated)) in rows.iter().zip([
        ("cause_event_sequence", "bigint", "s"),
        ("observations_canonical", "bytea", ""),
        ("tracking_administration_revision", "bigint", "s"),
        ("tracking_canonical", "bytea", ""),
    ]) {
        assert_eq!(row.get::<_, String>(0), name);
        assert_eq!(row.get::<_, String>(1), kind);
        assert!(!row.get::<_, bool>(2));
        assert_eq!(row.get::<_, String>(3), generated);
    }
}

pub fn snapshot(db: &mut Fixture, upgraded: bool) -> Value {
    let mut result: Value = db.admin.query_one("SELECT jsonb_build_object(
        'deadlines',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM case_deadlines r),
        'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY deadline_id,revision) FROM case_deadline_revisions r),
        'profiles',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM deadline_profiles r),
        'profile_revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY profile_id,revision) FROM deadline_profile_revisions r),
        'cases',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM cases r),
        'administration',(SELECT jsonb_agg(to_jsonb(r) ORDER BY case_id,revision) FROM case_administration_revisions r),
        'users',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM users r),
        'memberships',(SELECT jsonb_agg(to_jsonb(r) ORDER BY case_id,user_id) FROM case_memberships r),
        'events',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM deadline_source_events r),
        'event_sequence',(SELECT jsonb_build_object('last_value',last_value,'is_called',is_called) FROM deadline_source_events_sequence),
        'audit',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM audit_events r))", &[]).unwrap().get(0);
    if let Some(rows) = result["revisions"].as_array_mut().filter(|_| upgraded) {
        for row in rows {
            let fields = row.as_object_mut().unwrap();
            for name in NEW_COLUMNS {
                assert_eq!(
                    fields.remove(name),
                    Some(Value::Null),
                    "legacy column {name}"
                );
            }
        }
    }
    result
}
