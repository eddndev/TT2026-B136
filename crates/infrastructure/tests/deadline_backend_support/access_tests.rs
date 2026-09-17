use super::*;
use application::cases::*;
use domain::{cases::CaseId, identity::UserId};

#[test]
fn store_authorizes_all_four_roles_before_reads_mutations_and_cross_case_lookups() {
    let Some(mut db) = Fixture::new() else { return };
    let command = setup(&db);
    let first = persist(
        &service(&db, db.owner, Role::Owner),
        db.case,
        command.clone(),
    );
    let owner = db.owner;
    let actors = [
        (owner, Role::Owner),
        (db.user("litigator", true), Role::Litigator),
        (db.user("paralegal", true), Role::Paralegal),
        (db.user("client", true), Role::Client),
    ];
    for (actor, role) in actors {
        let adapter = store(&db);
        for result in [
            adapter
                .list(actor, db.case, query(20, None), db.at)
                .map(|_| ()),
            adapter
                .get(actor, db.case, first.id, None, db.at)
                .map(|_| ()),
            adapter
                .history(actor, db.case, first.id, history_query(20, None), db.at)
                .map(|_| ()),
        ] {
            if role == Role::Client {
                assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
            } else {
                result.unwrap();
            }
        }
        let before = snapshot(&mut db);
        let result = adapter.prepare(actor, db.case, &correct(&first));
        if matches!(role, Role::Owner | Role::Litigator) {
            result.unwrap();
        } else {
            assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
        }
        assert_eq!(snapshot(&mut db), before);
    }
    let outsider = db.user("litigator", false);
    let other_case = CaseId::new();
    db.admin.execute("INSERT INTO cases(id,title,reference,created_by,required_initial_revision) VALUES($1,'Other case','OTHER',$2,NULL)", &[&other_case.as_uuid(), &db.owner.as_uuid()]).unwrap();
    let before = snapshot(&mut db);
    assert!(matches!(
        store(&db).list(outsider, db.case, query(20, None), db.at),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(matches!(
        store(&db).prepare(outsider, db.case, &correct(&first)),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(matches!(
        store(&db).get(db.owner, other_case, first.id, None, db.at),
        Err(ApplicationError::Deadline(DeadlineError::NotFound))
    ));
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn responsible_must_be_active_staff_authorized_for_the_case_but_history_is_immutable() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let profile = profile(&db);
    let source = source(&db);
    let paralegal = db.user("paralegal", true);
    for actor in [db.owner, db.user("litigator", true), paralegal] {
        let mut candidate = command(&db, &profile, &source);
        definition_mut(&mut candidate).responsible = actor;
        let row = persist(&workflow, db.case, candidate);
        assert_eq!(row.responsible.id, actor);
    }
    let client = db.user("client", true);
    let outsider = db.user("litigator", false);
    let inactive = db.user("paralegal", true);
    db.admin
        .execute(
            "UPDATE users SET active=false WHERE id=$1",
            &[&inactive.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    for actor in [client, outsider, inactive, UserId::new()] {
        let mut candidate = command(&db, &profile, &source);
        definition_mut(&mut candidate).responsible = actor;
        assert!(matches!(
            workflow.prepare("session", db.case, candidate),
            Err(ApplicationError::Deadline(
                DeadlineError::ResponsibleUnavailable
            ))
        ));
    }
    assert_eq!(snapshot(&mut db), before);
    let mut candidate = command(&db, &profile, &source);
    definition_mut(&mut candidate).responsible = paralegal;
    let row = persist(&workflow, db.case, candidate);
    db.admin
        .execute(
            "UPDATE users SET active=false,email='former@example.test' WHERE id=$1",
            &[&paralegal.as_uuid()],
        )
        .unwrap();
    assert_eq!(
        workflow
            .get("session", db.case, row.id, Some(row.revision))
            .unwrap(),
        row
    );
}

#[test]
fn closed_case_preserves_reads_and_rejects_every_mutation_and_prepared_commit() {
    let Some(mut db) = Fixture::new() else { return };
    crate::case_stage_database_support::complete(&db);
    let workflow = service(&db, db.owner, Role::Owner);
    let initial = setup(&db);
    let first = persist(&workflow, db.case, initial.clone());
    let mut new = initial;
    new.deadline_id = DeadlineId::new();
    new.operation_id = DeadlineOperationId::new();
    let commands = [new, correct(&first), attention(&first), retire(&first)];
    let pending = prepared(&db, db.owner, &commands[1]);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    assert_eq!(
        workflow.get("session", db.case, first.id, None).unwrap(),
        first
    );
    assert_eq!(
        workflow
            .history("session", db.case, first.id, history_query(20, None))
            .unwrap()
            .revisions
            .len(),
        1
    );
    assert_eq!(
        workflow
            .list("session", db.case, query(20, None))
            .unwrap()
            .deadlines
            .len(),
        1
    );
    let before = snapshot(&mut db);
    for command in commands {
        assert!(matches!(
            workflow.prepare("session", db.case, command),
            Err(ApplicationError::CaseClosed)
        ));
    }
    assert!(matches!(
        store(&db).commit(db.owner, pending),
        Err(ApplicationError::CaseClosed)
    ));
    assert_eq!(snapshot(&mut db), before);
}
