use super::*;
use application::cases::{CaseAdministrativeStatus, CaseRepository, CaseRevisionExpectation};
use domain::precautionary_measures::MeasureCorrectionOperationId;
use uuid::Uuid;

#[path = "read_authorization.rs"]
mod authorization;
#[path = "read_integrity.rs"]
mod integrity;
#[path = "read_sources.rs"]
mod sources;

fn reads(db: &Fixture, actor: Principal) -> MeasureAdministrativeReadService {
    existing_reads(db, actor, store(db))
}

fn existing_reads(
    db: &Fixture,
    actor: Principal,
    storage: Arc<PostgresMeasureAdministrativeStore>,
) -> MeasureAdministrativeReadService {
    MeasureAdministrativeReadService::new(
        storage,
        Arc::new(TestIdentity(actor)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

fn id(value: u128) -> MeasureCorrectionOperationId {
    MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(value))
}

fn chain(db: &mut Fixture) -> (Seed, Vec<MeasureAdministrativeStoredOperation>) {
    let (seed, _, mut command) = setup(db);
    command.operation_id = id(10);
    let first = persist(db, seed.actor.clone(), command);
    let mut command = correction(
        corrected_reference(&first.capture),
        seed.command.context,
        &first.capture.review.result.values,
        "Second historical correction",
    );
    command.operation_id = id(30);
    let second = persist(db, seed.actor.clone(), command);
    let mut command = mark(corrected_reference(&second.capture), seed.command.context);
    command.operation_id = id(20);
    let marked = persist(db, seed.actor.clone(), command);
    (seed, vec![first, second, marked])
}

#[test]
fn administrative_operation_reads_preserve_each_original_capture_and_mark() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, originals) = chain(&mut db);
    let service = reads(&db, seed.actor);
    for original in &originals {
        let actual = service
            .get_operation("session", db.case, original.origin.operation_id)
            .unwrap();
        same_operation(&actual, original);
        assert_eq!(
            measure_administrative_capture_bytes(&actual.capture).unwrap(),
            measure_administrative_capture_bytes(&original.capture).unwrap()
        );
    }
    assert_eq!(
        originals[2].capture.review.result.validity,
        MeasureCaptureValidity::EnteredInError
    );
    let count: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action='measure_administrative.operation'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 3);
}

#[test]
fn administrative_pages_order_operations_with_exclusive_exact_cursors() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, mut expected) = chain(&mut db);
    expected.sort_by_key(|a| a.origin.operation_id.as_uuid());
    let service = reads(&db, seed.actor);
    let first = service
        .list(
            "session",
            db.case,
            MeasureAdministrativeReadQuery::new(2, None).unwrap(),
        )
        .unwrap();
    assert_eq!(first.case_id, db.case);
    assert_eq!(first.items.len(), 2);
    for (actual, expected) in first.items.iter().zip(&expected[..2]) {
        same_operation(actual, expected);
    }
    assert!(first.has_more);
    assert_eq!(first.next_after_operation_id, Some(id(20)));
    let last = service
        .list(
            "session",
            db.case,
            MeasureAdministrativeReadQuery::new(2, first.next_after_operation_id).unwrap(),
        )
        .unwrap();
    assert_eq!(last.items.len(), 1);
    same_operation(&last.items[0], &expected[2]);
    assert!(!last.has_more);
    assert_eq!(last.next_after_operation_id, None);
    let between = service
        .list(
            "session",
            db.case,
            MeasureAdministrativeReadQuery::new(1, Some(id(15))).unwrap(),
        )
        .unwrap();
    assert_eq!(between.items[0].origin.operation_id, id(20));
    assert!(between.has_more);
    assert_eq!(between.next_after_operation_id, Some(id(20)));
    let empty = service
        .list(
            "session",
            db.case,
            MeasureAdministrativeReadQuery::new(2, Some(id(30))).unwrap(),
        )
        .unwrap();
    assert!(empty.items.is_empty());
    assert!(!empty.has_more);
    assert_eq!(empty.next_after_operation_id, None);
}

#[test]
fn closed_case_reads_use_current_authority_and_retain_original_actor() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, command) = setup(&mut db);
    let user = db.user("litigator", true);
    let recorded_actor = crate::measure_fixture::principal(&mut db, user);
    let original = persist(&db, recorded_actor.clone(), command);
    db.admin
        .execute(
            "UPDATE users SET email='current-reader@example.test',role='paralegal',
         revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
            &[&user.as_uuid()],
        )
        .unwrap();
    let reader = crate::measure_fixture::principal(&mut db, user);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::Revision(seed.command.context.administration_revision),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let service = reads(&db, reader.clone());
    let actual = service
        .get_operation("session", db.case, original.origin.operation_id)
        .unwrap();
    same_operation(&actual, &original);
    assert_eq!(actual.capture.review.actor, recorded_actor);
    let page = service
        .list(
            "session",
            db.case,
            MeasureAdministrativeReadQuery::default(),
        )
        .unwrap();
    assert_eq!(page.items.len(), 1);
    same_operation(&page.items[0], &original);
    let audit = db
        .admin
        .query(
            "SELECT actor,action,resource FROM audit_events WHERE action IN
         ('measure_administrative.operation','measure_administrative.list') ORDER BY sequence",
            &[],
        )
        .unwrap();
    assert_eq!(audit.len(), 2);
    assert!(audit
        .iter()
        .all(|r| r.get::<_, String>("actor") == reader.email));
    let expected_marker = format!(
        "ma1:case:{}:operation:{}:measure:{}:revision:{}:submission:{}:review:{}:capture:{}",
        db.case,
        original.origin.operation_id,
        original.capture.review.result.id,
        original.capture.review.result.revision.get(),
        original.capture.review.submission_digest.to_hex(),
        original.capture.review.review_digest.to_hex(),
        original.capture.capture_digest.to_hex(),
    );
    assert_eq!(audit[0].get::<_, String>("resource"), expected_marker);
    assert_eq!(
        audit[1].get::<_, String>("resource"),
        format!("case:{}", db.case)
    );
}
