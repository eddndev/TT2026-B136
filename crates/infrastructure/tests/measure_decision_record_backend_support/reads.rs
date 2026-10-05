use super::*;
use application::cases::{CaseAdministrativeStatus, CaseRepository, CaseRevisionExpectation};
use infrastructure::PostgresMeasureDecisionStore;

mod integrity;

fn reads(db: &Fixture, actor: Principal) -> MeasureDecisionRecordReadService {
    MeasureDecisionRecordReadService::new(
        crate::measure_fixture::store(db),
        Arc::new(TestIdentity(actor)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

fn chain(db: &mut Fixture) -> (RecordSeed, Vec<MeasureDecisionRecordReceipt>) {
    let seed = setup(db);
    let second = persist(db, seed.actor.clone(), seed.command.clone());
    let row = &second.group.measures[0];
    crate::administrative_fixture::persist(
        db,
        seed.actor.clone(),
        crate::administrative_fixture::correction(
            reference(row),
            seed.command.context,
            &row.result.values,
            "Correction after the actual M2",
        ),
    );
    let empty = persist(
        db,
        seed.actor.clone(),
        crate::measure_fixture::no_change(&seed.command),
    );
    let mut receipts = vec![
        MeasureDecisionRecordReceipt::V1(Box::new(seed.judicial.clone())),
        MeasureDecisionRecordReceipt::V2(Box::new(second)),
        MeasureDecisionRecordReceipt::V2(Box::new(empty)),
    ];
    receipts.sort_by_key(|r| r.command().decision_id.as_uuid());
    (seed, receipts)
}

fn same(actual: &MeasureDecisionRecordReceipt, expected: &MeasureDecisionRecordReceipt) {
    let mut actual = actual.clone();
    let mut expected = expected.clone();
    for receipt in [&mut actual, &mut expected] {
        match receipt {
            MeasureDecisionRecordReceipt::V1(r) => r
                .measure_history
                .groups
                .sort_by_key(|g| g.origin.operation_id.as_uuid()),
            MeasureDecisionRecordReceipt::V2(r) => {
                r.record_history
                    .records
                    .judicial
                    .groups
                    .sort_by_key(|g| g.origin.operation_id.as_uuid());
                r.record_history
                    .records
                    .administrative
                    .sort_by_key(|a| a.origin.operation_id.as_uuid());
                r.record_history
                    .decisions
                    .sort_by_key(|g| g.origin.operation_id.as_uuid());
            }
        }
    }
    assert_eq!(actual, expected);
}

#[test]
fn mixed_decision_reads_preserve_original_family_and_closure_for_every_selector() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, originals) = chain(&mut db);
    let reader = reads(&db, seed.actor);
    for original in &originals {
        same(
            &reader
                .get("session", db.case, original.command().decision_id)
                .unwrap(),
            original,
        );
        same(
            &reader
                .get_operation("session", db.case, original.command().operation_id)
                .unwrap(),
            original,
        );
    }
    let first = reader
        .list(
            "session",
            db.case,
            MeasureDecisionReadQuery::new(2, None).unwrap(),
        )
        .unwrap();
    assert_eq!(first.case_id, db.case);
    assert_eq!(first.items.len(), 2);
    assert!(first.has_more);
    assert_eq!(
        first.next_after_id,
        Some(originals[1].command().decision_id)
    );
    for (actual, original) in first.items.iter().zip(&originals[..2]) {
        same(actual, original);
    }
    let last = reader
        .list(
            "session",
            db.case,
            MeasureDecisionReadQuery::new(2, first.next_after_id).unwrap(),
        )
        .unwrap();
    assert_eq!(last.items.len(), 1);
    same(&last.items[0], &originals[2]);
    assert!(!last.has_more);
    assert_eq!(last.next_after_id, None);
    let actions: Vec<String> = db.admin.query("SELECT action FROM audit_events WHERE action IN
        ('measure_decision.read','measure_decision.operation','measure_decision.list') ORDER BY sequence", &[])
        .unwrap().into_iter().map(|r| r.get(0)).collect();
    assert_eq!(
        actions
            .iter()
            .filter(|a| a.as_str() == "measure_decision.read")
            .count(),
        3
    );
    assert_eq!(
        actions
            .iter()
            .filter(|a| a.as_str() == "measure_decision.operation")
            .count(),
        3
    );
    assert_eq!(
        actions
            .iter()
            .filter(|a| a.as_str() == "measure_decision.list")
            .count(),
        2
    );
}

#[test]
fn mixed_decision_read_port_requires_current_principal_and_exact_case_after_closure() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), seed.command.clone());
    let original_case = db.case;
    let user = db.user("paralegal", true);
    let actor = crate::measure_fixture::principal(&mut db, user);
    let storage = crate::measure_fixture::store(&db);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::Revision(seed.command.context.administration_revision),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let expected = MeasureDecisionRecordReceipt::V2(Box::new(original.clone()));
    same(
        &reads(&db, actor.clone())
            .get("session", db.case, original.origin.decision_id)
            .unwrap(),
        &expected,
    );
    let stale = Principal {
        email: "stale-reader@example.test".into(),
        ..actor.clone()
    };
    denied(&mut db, &storage, &stale, &original);
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &user.as_uuid()],
        )
        .unwrap();
    denied(&mut db, &storage, &actor, &original);
    let _other = crate::measure_fixture::setup(&mut db);
    let reader = reads(&db, seed.actor);
    let before = crate::administrative_fixture::snapshot(&mut db);
    assert!(reader
        .get("session", db.case, original.origin.decision_id)
        .is_err());
    assert!(reader
        .get_operation("session", db.case, original.origin.operation_id)
        .is_err());
    assert!(reader
        .get("session", original_case, MeasureDecisionId::new())
        .is_err());
    assert!(reader
        .get_operation("session", original_case, MeasureDecisionOperationId::new())
        .is_err());
    assert_eq!(crate::administrative_fixture::snapshot(&mut db), before);
    let empty = reader
        .list("session", db.case, MeasureDecisionReadQuery::default())
        .unwrap();
    assert!(empty.items.is_empty());
    assert!(!empty.has_more);
    assert_eq!(empty.next_after_id, None);
}

fn denied(
    db: &mut Fixture,
    storage: &PostgresMeasureDecisionStore,
    actor: &Principal,
    original: &MeasureDecisionRecordStoredOperation,
) {
    let before = crate::administrative_fixture::snapshot(db);
    assert!(MeasureDecisionRecordReadStore::get(
        storage,
        actor,
        db.case,
        original.origin.decision_id
    )
    .is_err());
    assert!(MeasureDecisionRecordReadStore::get_operation(
        storage,
        actor,
        db.case,
        original.origin.operation_id
    )
    .is_err());
    assert!(MeasureDecisionRecordReadStore::list(
        storage,
        actor,
        db.case,
        MeasureDecisionReadQuery::default()
    )
    .is_err());
    assert_eq!(crate::administrative_fixture::snapshot(db), before);
}
