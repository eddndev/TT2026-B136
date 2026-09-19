mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
#[allow(dead_code)]
mod deadline_dispatch_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
#[allow(dead_code)]
mod deadline_worker_backend_support;
mod procedural_fact_backend_support;

use application::{
    cases::*, deadline_currentness::DeadlineFreshness, deadlines::*, ApplicationError,
};
use deadline_backend_support as dl;
use deadline_worker_backend_support as worker;
use domain::cases::CaseId;

#[test]
fn current_reads_recheck_roles_membership_and_case_isolation() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (_, base) = worker::accepted(&mut db, 30);
    let litigator = db.user("litigator", true);
    let paralegal = db.user("paralegal", true);
    let client = db.user("client", true);
    let outsider = db.user("litigator", false);
    let store = dl::store(&db);
    for actor in [db.owner, litigator, paralegal] {
        assert_eq!(
            store.current(actor, db.case, base.id).unwrap().detail(),
            &base
        );
    }
    let other = CaseId::new();
    db.admin.execute("INSERT INTO cases(id,title,reference,created_by,required_initial_revision) VALUES($1,'Other case','CURRENT-OTHER',$2,NULL)", &[&other.as_uuid(), &db.owner.as_uuid()]).unwrap();
    let before = worker::snapshot(&mut db);
    assert!(matches!(
        store.current(client, db.case, base.id),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        store.current(outsider, db.case, base.id),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(matches!(
        store.current(db.owner, other, base.id),
        Err(ApplicationError::Deadline(DeadlineError::NotFound))
    ));
    assert_eq!(worker::snapshot(&mut db), before);
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &litigator.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        store.current(litigator, db.case, base.id),
        Err(ApplicationError::CaseNotFound)
    ));
    db.admin
        .execute(
            "UPDATE users SET active=false WHERE id=$1",
            &[&paralegal.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        store.current(paralegal, db.case, base.id),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn administrative_closure_preserves_a_current_active_deadline_and_its_capture() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    case_stage_database_support::complete(&db);
    let (_, base) = worker::accepted(&mut db, 40);
    assert!(base.calculation.result.due_at().is_some());
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let read = dl::store(&db).current(db.owner, db.case, base.id).unwrap();
    assert_eq!(read.detail(), &base);
    assert_eq!(read.operational().freshness(), DeadlineFreshness::Current);
    assert_eq!(
        read.operational().due_at(),
        base.calculation.result.due_at()
    );
}

#[test]
fn failed_current_read_audit_returns_no_projection_or_partial_changes() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (_, base) = worker::accepted(&mut db, 50);
    db.admin.batch_execute("CREATE FUNCTION reject_current_read_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='deadline.current_read' THEN RAISE EXCEPTION 'injected audit failure'; END IF; RETURN NEW; END; $$; CREATE TRIGGER reject_current_read_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_current_read_audit()").unwrap();
    let before = worker::snapshot(&mut db);
    assert!(dl::store(&db).current(db.owner, db.case, base.id).is_err());
    assert_eq!(worker::snapshot(&mut db), before);
}
