mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_result_database_support;
mod hearing_result_inventory_support;
mod hearing_result_restore_support;
use application::hearing_results::*;
use case_administration_support::Fixture;
use domain::identity::Role;
use hearing_result_database_support::{persist, service, values};
use hearing_result_restore_support::{correct, record_at, withdraw};
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;

fn seed(db: &mut Fixture) -> HearingResultDetail {
    hearing_database_support::complete(db);
    let hearing = hearing_database_support::persist(
        &hearing_database_support::service(db, db.owner, Role::Owner),
        db.case,
        hearing_database_support::schedule(),
    );
    persist(
        &service(db, db.owner, Role::Owner),
        db.case,
        record_at(
            hearing.snapshot.id,
            hearing.snapshot.revision,
            None,
            values("Declared account"),
        ),
    )
}

fn rejects_startup(db: &Fixture) {
    assert!(PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err());
}

#[test]
fn startup_rejects_result_roots_without_their_first_revision() {
    let Some(mut db) = Fixture::new() else { return };
    db.store();
    db.admin
        .batch_execute("ALTER TABLE case_hearing_results DISABLE TRIGGER ALL")
        .unwrap();
    db.admin
        .execute(
            "INSERT INTO case_hearing_results(id,case_id,hearing_id,anchor_revision,
         anchor_values_digest,anchor_submission_digest) VALUES($1,$2,$3,1,$4,$4)",
            &[
                &uuid::Uuid::new_v4(),
                &db.case.as_uuid(),
                &uuid::Uuid::new_v4(),
                &&[0_u8; 32][..],
            ],
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE case_hearing_results ENABLE TRIGGER ALL")
        .unwrap();
    assert!(PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err());
}

#[test]
fn startup_checks_exact_sources_even_for_the_nil_uuid_root() {
    let Some(mut db) = Fixture::new() else { return };
    let first = seed(&mut db);
    let mut command = record_at(
        first.snapshot.hearing_id,
        first.snapshot.anchor.revision,
        None,
        values("Minimum UUID must be included in inventory"),
    );
    command.result_id = HearingResultId::from_uuid(uuid::Uuid::nil());
    let selected = persist(&service(&db, db.owner, Role::Owner), db.case, command);
    db.admin
        .batch_execute("ALTER TABLE case_hearing_results DISABLE TRIGGER hearing_result_immutable")
        .unwrap();
    db.admin
        .execute(
            "UPDATE case_hearing_results SET anchor_values_digest=$1 WHERE id=$2",
            &[&&[9_u8; 32][..], &selected.snapshot.id.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE case_hearing_results ENABLE TRIGGER hearing_result_immutable")
        .unwrap();
    rejects_startup(&db);
}

#[test]
fn startup_detects_continuation_cycles_without_recursive_source_expansion() {
    let Some(mut db) = Fixture::new() else { return };
    let first = seed(&mut db);
    let next = persist(
        &service(&db, db.owner, Role::Owner),
        db.case,
        record_at(
            first.snapshot.hearing_id,
            first.snapshot.anchor.revision,
            Some(HearingResultContinuationRef::new(
                first.snapshot.id,
                first.snapshot.revision,
            )),
            values("Declared continuation"),
        ),
    );
    db.admin
        .batch_execute("ALTER TABLE case_hearing_results DISABLE TRIGGER hearing_result_immutable")
        .unwrap();
    db.admin
        .execute(
            "UPDATE case_hearing_results SET continuation_hearing_id=$1,continuation_result_id=$2,
         continuation_revision=$3,continuation_values_digest=$4,continuation_submission_digest=$5
         WHERE id=$6",
            &[
                &next.snapshot.hearing_id.as_uuid(),
                &next.snapshot.id.as_uuid(),
                &i64::from(next.snapshot.revision.get()),
                &&next.snapshot.values_digest.as_bytes()[..],
                &&next.snapshot.receipt.submission_digest.as_bytes()[..],
                &first.snapshot.id.as_uuid(),
            ],
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE case_hearing_results ENABLE TRIGGER hearing_result_immutable")
        .unwrap();
    rejects_startup(&db);
}

#[test]
fn startup_rejects_missing_intermediate_result_history() {
    let Some(mut db) = Fixture::new() else { return };
    let first = seed(&mut db);
    let svc = service(&db, db.owner, Role::Owner);
    let corrected = persist(&svc, db.case, correct(&first, values("Corrected account")));
    persist(&svc, db.case, withdraw(&corrected));
    db.admin
        .batch_execute("ALTER TABLE case_hearing_result_revisions DISABLE TRIGGER ALL")
        .unwrap();
    db.admin
        .execute(
            "DELETE FROM case_hearing_result_revisions WHERE result_id=$1 AND revision=2",
            &[&first.snapshot.id.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE case_hearing_result_revisions ENABLE TRIGGER ALL")
        .unwrap();
    rejects_startup(&db);
}

#[test]
fn reads_and_startup_reject_withdrawal_that_changed_the_prior_content() {
    let Some(mut db) = Fixture::new() else { return };
    let first = seed(&mut db);
    let svc = service(&db, db.owner, Role::Owner);
    let row = hearing_result_inventory_support::forged_revision(
        &mut db,
        &first,
        &withdraw(&first),
        &values("Illicitly changed withdrawn content"),
    );
    db.admin
        .batch_execute(
            "ALTER TABLE case_hearing_result_revisions DISABLE TRIGGER hearing_result_sequence",
        )
        .unwrap();
    hearing_result_inventory_support::insert(&mut db, &row);
    db.admin
        .batch_execute(
            "ALTER TABLE case_hearing_result_revisions ENABLE TRIGGER hearing_result_sequence",
        )
        .unwrap();
    assert!(svc
        .get(
            "session",
            db.case,
            first.snapshot.hearing_id,
            first.snapshot.id,
            None
        )
        .is_err());
    rejects_startup(&db);
}
