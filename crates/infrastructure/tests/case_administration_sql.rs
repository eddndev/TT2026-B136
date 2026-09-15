mod case_administration_support;

use case_administration_support::Fixture;
use postgres::{error::SqlState, GenericClient};
use uuid::Uuid;

fn revision<C: GenericClient>(
    client: &mut C,
    case: Uuid,
    actor: Uuid,
    number: i64,
    status: &str,
    nuc: Option<&str>,
) -> Result<u64, postgres::Error> {
    let authority = nuc.map(|_| "Authority");
    let offenses = nuc.map(|_| vec!["Offense"]);
    client.execute("INSERT INTO case_administration_revisions(case_id,revision,title,reference,administrative_status,nuc,nuc_authority,judicial_case_number,judicial_authority,offenses,values_digest,changed_at,changed_by,changed_by_email)
        VALUES($1,$2,'Baseline','REF-OLD',$3,$4,$5,$4,$5,$6,pg_catalog.sha256(case_administration_bytes($3,'Baseline','REF-OLD',$4,$5,$4,$5,$6,NULL,NULL)),'2025-01-01T00:00:00.123456789Z',$7,'owner@example.test')", &[&case,&number,&status,&nuc,&authority,&offenses,&actor])
}

#[test]
fn complete_new_registration_requires_initial_stage_but_baseline_completion_forbids_it() {
    let Some(mut db) = Fixture::new() else {
        return;
    };
    let id = Uuid::new_v4();
    let mut tx = db.admin.transaction().unwrap();
    tx.execute(
        "INSERT INTO cases(id,title,reference,created_by) VALUES($1,'Baseline','REF-OLD',$2)",
        &[&id, &db.owner.as_uuid()],
    )
    .unwrap();
    revision(&mut tx, id, db.owner.as_uuid(), 1, "active", Some("NUC-1")).unwrap();
    assert_eq!(
        tx.commit().unwrap_err().code(),
        Some(&SqlState::CHECK_VIOLATION)
    );
    let mut tx = db.admin.transaction().unwrap();
    tx.execute(
        "INSERT INTO cases(id,title,reference,created_by) VALUES($1,'Baseline','REF-OLD',$2)",
        &[&id, &db.owner.as_uuid()],
    )
    .unwrap();
    revision(&mut tx, id, db.owner.as_uuid(), 1, "active", Some("NUC-1")).unwrap();
    tx.execute(
        "INSERT INTO case_initial_stage_registrations(case_id,stage) VALUES($1,'investigation')",
        &[&id],
    )
    .unwrap();
    tx.commit().unwrap();
    revision(
        &mut db.admin,
        db.case.as_uuid(),
        db.owner.as_uuid(),
        1,
        "active",
        Some("NUC-2"),
    )
    .unwrap();
    assert_eq!(db.admin.execute("INSERT INTO case_initial_stage_registrations(case_id,stage) VALUES($1,'investigation')", &[&db.case.as_uuid()]).unwrap_err().code(),Some(&SqlState::CHECK_VIOLATION));
}

#[test]
fn current_identifiers_include_closed_cases_and_superseded_values_can_be_reused() {
    let Some(mut db) = Fixture::new() else {
        return;
    };
    let other = Uuid::new_v4();
    db.admin.execute("INSERT INTO cases(id,title,reference,created_by,required_initial_revision) VALUES($1,'Baseline','REF-OLD',$2,NULL)", &[&other,&db.owner.as_uuid()]).unwrap();
    revision(
        &mut db.admin,
        db.case.as_uuid(),
        db.owner.as_uuid(),
        1,
        "active",
        Some("NUC-1"),
    )
    .unwrap();
    revision(
        &mut db.admin,
        db.case.as_uuid(),
        db.owner.as_uuid(),
        2,
        "closed",
        Some("NUC-1"),
    )
    .unwrap();
    assert_eq!(
        revision(
            &mut db.admin,
            other,
            db.owner.as_uuid(),
            1,
            "active",
            Some("NUC-1")
        )
        .unwrap_err()
        .code(),
        Some(&SqlState::UNIQUE_VIOLATION)
    );
    revision(
        &mut db.admin,
        db.case.as_uuid(),
        db.owner.as_uuid(),
        3,
        "active",
        Some("NUC-1"),
    )
    .unwrap();
    revision(
        &mut db.admin,
        db.case.as_uuid(),
        db.owner.as_uuid(),
        4,
        "active",
        Some("NUC-2"),
    )
    .unwrap();
    revision(
        &mut db.admin,
        other,
        db.owner.as_uuid(),
        1,
        "active",
        Some("NUC-1"),
    )
    .unwrap();
    let count: i64 = db
        .admin
        .query_one(
            "SELECT COUNT(*) FROM case_administration_revisions WHERE nuc='NUC-1'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 4);
}

#[test]
fn closed_and_status_only_successors_preserve_values_and_complete_profiles_cannot_be_removed() {
    let Some(mut db) = Fixture::new() else {
        return;
    };
    revision(
        &mut db.admin,
        db.case.as_uuid(),
        db.owner.as_uuid(),
        1,
        "active",
        Some("NUC-1"),
    )
    .unwrap();
    for (status, nuc) in [("closed", Some("NUC-2")), ("active", None)] {
        assert_eq!(
            revision(
                &mut db.admin,
                db.case.as_uuid(),
                db.owner.as_uuid(),
                2,
                status,
                nuc
            )
            .unwrap_err()
            .code(),
            Some(&SqlState::CHECK_VIOLATION)
        );
    }
    revision(
        &mut db.admin,
        db.case.as_uuid(),
        db.owner.as_uuid(),
        2,
        "closed",
        Some("NUC-1"),
    )
    .unwrap();
    assert_eq!(
        revision(
            &mut db.admin,
            db.case.as_uuid(),
            db.owner.as_uuid(),
            3,
            "closed",
            Some("NUC-2")
        )
        .unwrap_err()
        .code(),
        Some(&SqlState::CHECK_VIOLATION)
    );
    revision(
        &mut db.admin,
        db.case.as_uuid(),
        db.owner.as_uuid(),
        3,
        "closed",
        Some("NUC-1"),
    )
    .unwrap();
}

#[test]
fn revision_writes_reject_a_repeatable_read_snapshot() {
    let Some(mut db) = Fixture::new() else {
        return;
    };
    let mut tx = db
        .admin
        .build_transaction()
        .isolation_level(postgres::IsolationLevel::RepeatableRead)
        .start()
        .unwrap();
    let error = revision(
        &mut tx,
        db.case.as_uuid(),
        db.owner.as_uuid(),
        1,
        "active",
        None,
    )
    .unwrap_err();
    assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
}
