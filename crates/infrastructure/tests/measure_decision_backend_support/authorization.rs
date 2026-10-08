use super::*;
use application::{documents::StageSupportReadLimits, ApplicationError};

#[test]
fn complete_current_principal_is_reloaded_before_replay_or_disclosure() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), seed.command.clone());
    let storage = store(&db);
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
        let before = snapshot(&mut db);
        assert!(MeasureDecisionStore::prepare(
            storage.as_ref(),
            &stale,
            db.case,
            &seed.command,
            &StageSupportReadLimits::standard()
        )
        .is_err());
        assert!(MeasureDecisionReadStore::get_operation(
            storage.as_ref(),
            &stale,
            db.case,
            seed.command.operation_id
        )
        .is_err());
        assert_eq!(snapshot(&mut db), before);
    }
    assert_eq!(
        MeasureDecisionReadStore::get_operation(
            storage.as_ref(),
            &seed.actor,
            db.case,
            seed.command.operation_id
        )
        .unwrap(),
        original
    );
}

#[test]
fn assigned_staff_reads_and_writes_follow_existing_roles_and_membership() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let litigator = db.user("litigator", true);
    let actor = principal(&mut db, litigator);
    let original = persist(&db, actor.clone(), seed.command.clone());
    let paralegal = db.user("paralegal", true);
    let reader = principal(&mut db, paralegal);
    assert_eq!(
        reads(&db, reader.clone())
            .get("session", db.case, seed.command.decision_id)
            .unwrap(),
        original
    );
    let before = snapshot(&mut db);
    assert!(matches!(
        service(&db, reader).prepare("session", db.case, no_change(&seed.command)),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(snapshot(&mut db), before);
    for role in ["client", "litigator", "paralegal"] {
        let user = db.user(role, role == "client");
        let unauthorized = principal(&mut db, user);
        let before = snapshot(&mut db);
        assert!(reads(&db, unauthorized.clone())
            .get_operation("session", db.case, seed.command.operation_id)
            .is_err());
        assert!(service(&db, unauthorized)
            .prepare("session", db.case, no_change(&seed.command))
            .is_err());
        assert_eq!(snapshot(&mut db), before);
    }
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &litigator.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    assert!(service(&db, actor)
        .prepare("session", db.case, seed.command)
        .is_err());
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn another_case_cannot_read_an_operation_or_supply_the_selected_subject() {
    let Some(mut db) = Fixture::new() else { return };
    let first = setup(&mut db);
    let first_case = db.case;
    let original = persist(&db, first.actor.clone(), first.command.clone());
    let second = setup(&mut db);
    let before = snapshot(&mut db);
    let query = reads(&db, second.actor.clone());
    assert!(query
        .get("session", db.case, first.command.decision_id)
        .is_err());
    assert!(query
        .get_operation("session", db.case, first.command.operation_id)
        .is_err());
    let mut foreign = fresh(&second.command);
    foreign.outcome = impositions(&first.subject, 1);
    assert!(service(&db, second.actor.clone())
        .prepare("session", db.case, foreign)
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(
        query
            .get("session", first_case, first.command.decision_id)
            .unwrap(),
        original
    );
}
