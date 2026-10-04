use crate::{
    case_administration_support::Fixture, deadline_profile_database_support as profiles,
    hearing_derived_deadline_storage_support as origins,
};
use application::deadline_profiles::DeadlineProfileCollection;
use domain::identity::Role;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use postgres::{error::SqlState, IsolationLevel};
use std::sync::Arc;
use uuid::Uuid;

const TABLE: &str = "case_hearing_derived_deadline_origins";
const COLUMNS: [&str; 17] = [
    "operation_id",
    "case_id",
    "hearing_id",
    "result_id",
    "result_revision",
    "deadline_id",
    "deadline_revision",
    "deadline_operation_id",
    "source_event_sequence",
    "actor_id",
    "actor_email",
    "actor_role",
    "review_canonical",
    "review_digest",
    "capture_canonical",
    "capture_digest",
    "audit_sequence",
];

#[test]
fn runtime_rejects_changed_origin_components_before_duplicate_key_checks() {
    let Some(mut db) = Fixture::new() else { return };
    let record = origins::record(&mut db);
    origins::insert(&mut db, &record);
    let operation = record.evidence().command.result.operation_id.as_uuid();
    let before = snapshot(&mut db);
    let foreign = format!("'{}'::uuid", Uuid::new_v4());
    let mut runtime = db.runtime();
    for (column, expression) in [
        ("source_event_sequence", "9223372036854775807::bigint"),
        ("case_id", foreign.as_str()),
        ("operation_id", foreign.as_str()),
        ("result_id", foreign.as_str()),
        ("deadline_id", foreign.as_str()),
        ("deadline_operation_id", foreign.as_str()),
        ("capture_canonical", "set_byte(capture_canonical,0,0)"),
    ] {
        let error = runtime
            .execute(&copy_origin(Some((column, expression))), &[&operation])
            .unwrap_err();
        assert_eq!(
            error.code(),
            Some(&SqlState::CHECK_VIOLATION),
            "altered {column} reached a different rejection: {error}"
        );
        assert_eq!(snapshot(&mut db), before, "altered {column} wrote state");
    }
    // The unmodified copy reaches uniqueness, proving the mutations were checked first.
    let error = runtime
        .execute(&copy_origin(None), &[&operation])
        .unwrap_err();
    assert_eq!(error.code(), Some(&SqlState::UNIQUE_VIOLATION));
    assert_eq!(snapshot(&mut db), before);
    open(&db).unwrap();
}

#[test]
fn runtime_requires_read_committed_before_inspecting_duplicate_origins() {
    let Some(mut db) = Fixture::new() else { return };
    let record = origins::record(&mut db);
    origins::insert(&mut db, &record);
    let operation = record.evidence().command.result.operation_id.as_uuid();
    let before = snapshot(&mut db);
    let mut runtime = db.runtime();
    let mut tx = runtime
        .build_transaction()
        .isolation_level(IsolationLevel::Serializable)
        .start()
        .unwrap();
    let error = tx.execute(&copy_origin(None), &[&operation]).unwrap_err();
    assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
    tx.rollback().unwrap();
    assert_eq!(snapshot(&mut db), before);
    open(&db).unwrap();
}

#[test]
fn inventory_preserves_original_author_role_and_selected_profile_after_heads_change() {
    let Some(mut db) = Fixture::new() else { return };
    let record = origins::record(&mut db);
    origins::insert(&mut db, &record);
    open(&db).unwrap();
    let original = origin_snapshot(&mut db);
    let profile = &record.evidence().material.profile;
    assert_eq!(profile.revision.get(), 1);
    let replacement = profiles::persist(
        &profiles::service(&db, db.owner, Role::Owner),
        DeadlineProfileCollection::ForCase(db.case),
        profiles::replace(profile),
    );
    assert_eq!(replacement.id, profile.id);
    assert_eq!(replacement.revision.get(), 2);
    db.user("owner", false);
    db.admin
        .execute(
            "UPDATE users SET role='litigator',revision=revision+1,
             auth_generation=auth_generation+1 WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    let actual_role: String = db
        .admin
        .query_one("SELECT role FROM users WHERE id=$1", &[&db.owner.as_uuid()])
        .unwrap()
        .get(0);
    assert_eq!(actual_role, "litigator");
    assert_eq!(record.evidence().actor.role, Role::Owner);
    open(&db).unwrap();
    assert_eq!(origin_snapshot(&mut db), original);
}

fn copy_origin(replacement: Option<(&str, &str)>) -> String {
    let selected = COLUMNS
        .iter()
        .map(|column| match replacement {
            Some((name, expression)) if name == *column => expression,
            _ => *column,
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "INSERT INTO {TABLE}({}) SELECT {selected} FROM {TABLE} WHERE operation_id=$1",
        COLUMNS.join(",")
    )
}

fn origin_snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin
        .query_one(
            &format!("SELECT jsonb_agg(to_jsonb(o) ORDER BY operation_id) FROM {TABLE} o"),
            &[],
        )
        .unwrap()
        .get(0)
}

fn snapshot(db: &mut Fixture) -> serde_json::Value {
    serde_json::json!({"origin": origin_snapshot(db), "case": db.snapshot()})
}

fn open(db: &Fixture) -> Result<PostgresCaseRepository, application::ApplicationError> {
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher))
}
