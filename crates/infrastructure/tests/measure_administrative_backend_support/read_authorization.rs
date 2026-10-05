use super::*;
use domain::identity::Role;

fn denied(
    db: &mut Fixture,
    storage: &PostgresMeasureAdministrativeStore,
    actor: &Principal,
    operation: MeasureCorrectionOperationId,
) {
    let before = snapshot(db);
    assert!(storage.get_operation(actor, db.case, operation).is_err());
    assert!(storage
        .list(actor, db.case, MeasureAdministrativeReadQuery::default())
        .is_err());
    assert_eq!(snapshot(db), before);
}

#[test]
fn administrative_reads_require_live_full_principal_and_case_membership() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, command) = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), command);
    let storage = store(&db);
    for role in ["owner", "litigator", "paralegal"] {
        let user = db.user(role, role != "owner");
        let actor = crate::measure_fixture::principal(&mut db, user);
        let actual = reads(&db, actor.clone())
            .get_operation("session", db.case, original.origin.operation_id)
            .unwrap();
        same_operation(&actual, &original);
        if role != "owner" {
            db.admin
                .execute(
                    "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                    &[&db.case.as_uuid(), &user.as_uuid()],
                )
                .unwrap();
            denied(&mut db, &storage, &actor, original.origin.operation_id);
        }
    }
    for role in ["client", "litigator", "paralegal"] {
        let user = db.user(role, role == "client");
        let actor = crate::measure_fixture::principal(&mut db, user);
        denied(&mut db, &storage, &actor, original.origin.operation_id);
    }
    for stale in [
        Principal {
            email: "stale@example.test".into(),
            ..seed.actor.clone()
        },
        Principal {
            role: Role::Litigator,
            ..seed.actor.clone()
        },
    ] {
        denied(&mut db, &storage, &stale, original.origin.operation_id);
    }
    same_operation(
        &storage
            .get_operation(&seed.actor, db.case, original.origin.operation_id)
            .unwrap(),
        &original,
    );
}

#[test]
fn administrative_operation_lookup_is_case_exact_and_missing_reads_leave_no_audit() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, command) = setup(&mut db);
    let original_case = db.case;
    let original = persist(&db, seed.actor.clone(), command);
    let _second = crate::measure_fixture::setup(&mut db);
    let service = reads(&db, seed.actor);
    let before = snapshot(&mut db);
    assert!(service
        .get_operation("session", db.case, original.origin.operation_id)
        .is_err());
    assert!(service
        .get_operation("session", original_case, id(999))
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    let empty = service
        .list(
            "session",
            db.case,
            MeasureAdministrativeReadQuery::default(),
        )
        .unwrap();
    assert!(empty.items.is_empty());
    assert!(!empty.has_more);
    assert_eq!(empty.next_after_operation_id, None);
    same_operation(
        &service
            .get_operation("session", original_case, original.origin.operation_id)
            .unwrap(),
        &original,
    );
}
