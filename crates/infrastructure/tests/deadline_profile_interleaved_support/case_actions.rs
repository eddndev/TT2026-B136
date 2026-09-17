use super::{unchanged, watched};
use crate::{case_stage_database_support, deadline_profile_database_support::*};
use application::{cases::*, deadline_profiles::*, ApplicationError};
use domain::identity::Role;

fn close(db: &Fixture) {
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
}

#[test]
fn closed_case_keeps_private_profile_reads_and_rejects_all_three_mutations() {
    let Some(mut db) = Fixture::new() else { return };
    case_stage_database_support::complete(&db);
    let collection = DeadlineProfileCollection::ForCase(db.case);
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist(&workflow, collection, publish(Some(db.case)));
    let commands = [publish(Some(db.case)), replace(&first), retire(&first)];
    let prepared = commands
        .iter()
        .map(|command| {
            workflow
                .prepare("session", collection, command.clone())
                .unwrap()
        })
        .collect::<Vec<_>>();
    close(&db);
    assert_eq!(
        workflow.get("session", collection, first.id, None).unwrap(),
        first
    );
    assert_eq!(
        workflow
            .get("session", collection, first.id, Some(first.revision))
            .unwrap(),
        first
    );
    let history = workflow
        .history(
            "session",
            collection,
            first.id,
            DeadlineProfileHistoryQuery::new(20, None).unwrap(),
        )
        .unwrap();
    assert_eq!(history.revisions, vec![(&first).into()]);
    let before = snapshot(&mut db);
    for (command, draft) in commands.into_iter().zip(prepared) {
        assert!(matches!(
            workflow.prepare("session", collection, command.clone()),
            Err(ApplicationError::CaseClosed)
        ));
        assert!(matches!(
            workflow.submit("session", collection, command, draft.submission_digest),
            Err(ApplicationError::CaseClosed)
        ));
    }
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn collection_context_cannot_publish_or_replace_a_different_immutable_scope() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let global_collection = DeadlineProfileCollection::Global;
    let private_collection = DeadlineProfileCollection::ForCase(db.case);
    let global = persist(&workflow, global_collection, publish(None));
    let private = persist(&workflow, private_collection, publish(Some(db.case)));
    let before = snapshot(&mut db);
    for (collection, command) in [
        (global_collection, publish(Some(db.case))),
        (private_collection, publish(None)),
    ] {
        assert!(matches!(
            workflow.prepare("session", collection, command),
            Err(ApplicationError::DeadlineProfile(
                DeadlineProfileError::ScopeChangeForbidden
            ))
        ));
    }
    for (base, collection, scope) in [
        (&global, global_collection, altered_global_scope()),
        (
            &global,
            private_collection,
            DeadlineProfileScope::Case(db.case),
        ),
        (&private, private_collection, altered_global_scope()),
    ] {
        let mut changed = input(match base.definition.scope() {
            DeadlineProfileScope::Case(case) => Some(*case),
            _ => None,
        });
        changed.scope = scope;
        let mut command = replace(base);
        let DeadlineProfileChange::Replace { definition, .. } = &mut command.change else {
            unreachable!()
        };
        *definition = DeadlineProfileDefinition::new(changed).unwrap();
        assert!(matches!(
            workflow.prepare("session", collection, command),
            Err(ApplicationError::DeadlineProfile(
                DeadlineProfileError::ScopeChangeForbidden
            ))
        ));
    }
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn case_closure_after_internal_prepare_blocks_a_private_profile_commit() {
    let Some(mut db) = Fixture::new() else { return };
    case_stage_database_support::complete(&db);
    let collection = DeadlineProfileCollection::ForCase(db.case);
    let command = publish(Some(db.case));
    let draft = service(&db, db.owner, Role::Owner)
        .prepare("session", collection, command.clone())
        .unwrap();
    let repository = db.store();
    let (owner, case, at) = (db.owner, db.case, db.at);
    let (workflow, captured) = watched(&db, owner, move |_| {
        repository
            .change_administrative_status(
                owner,
                case,
                CaseRevisionExpectation::new(1),
                CaseAdministrativeStatus::Closed,
                at,
            )
            .unwrap();
    });
    let result = workflow.submit("session", collection, command, draft.submission_digest);
    assert!(
        matches!(result, Err(ApplicationError::CaseClosed)),
        "{result:?}"
    );
    unchanged(&mut db, captured);
}
