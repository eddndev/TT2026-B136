mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_profile_database_support;
mod deadline_profile_interleaved_support;

use application::{deadline_profiles::*, ApplicationError};
use deadline_profile_database_support::*;
use deadline_profile_interleaved_support::{counts, hooked, unchanged, watched};
use domain::identity::Role;
use std::sync::{Arc, Barrier};

#[test]
fn concurrent_stores_append_only_one_revision_event_and_audit_for_the_same_base() {
    let Some(mut db) = Fixture::new() else { return };
    let other_owner = db.user("owner", false);
    let collection = DeadlineProfileCollection::Global;
    let first = persist(
        &service(&db, db.owner, Role::Owner),
        collection,
        publish(None),
    );
    let before = counts(&mut db);
    let barrier = Arc::new(Barrier::new(2));
    let workers = [db.owner, other_owner]
        .into_iter()
        .map(|actor| {
            let command = replace(&first);
            let draft = service(&db, actor, Role::Owner)
                .prepare("session", collection, command.clone())
                .unwrap();
            let barrier = barrier.clone();
            let workflow = hooked(&db, actor, move || {
                barrier.wait();
            });
            std::thread::spawn(move || {
                workflow.submit("session", collection, command, draft.submission_digest)
            })
        })
        .collect::<Vec<_>>();
    let results = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results.iter().filter(|result| result.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                Err(ApplicationError::DeadlineProfile(
                    DeadlineProfileError::RevisionConflict
                ))
            ))
            .count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        counts(&mut db),
        (before.0, before.1 + 1, before.2 + 1, before.3 + 1)
    );
    let winner = results.into_iter().find_map(Result::ok).unwrap();
    assert_eq!(winner.revision.get(), first.revision.get() + 1);
    let row = db.admin.query_one("SELECT operation_id FROM deadline_source_events WHERE source_kind='profile' AND source_id=$1 AND revision=$2",
        &[&winner.id.as_uuid(), &i64::from(winner.revision.get())]).unwrap();
    assert_eq!(
        row.get::<_, uuid::Uuid>(0),
        winner.receipt.operation_id.as_uuid()
    );
}

#[test]
fn owner_revocation_and_demotion_after_internal_prepare_prevent_commit() {
    for inactive in [true, false] {
        let Some(mut db) = Fixture::new() else { return };
        let collection = DeadlineProfileCollection::Global;
        let command = publish(None);
        let draft = service(&db, db.owner, Role::Owner)
            .prepare("session", collection, command.clone())
            .unwrap();
        let owner = db.owner;
        let (workflow, captured) = watched(&db, owner, move |client| {
            client
                .execute(
                    if inactive {
                        "UPDATE users SET active=false WHERE id=$1"
                    } else {
                        "UPDATE users SET role='litigator' WHERE id=$1"
                    },
                    &[&owner.as_uuid()],
                )
                .unwrap();
        });
        let result = workflow.submit("session", collection, command, draft.submission_digest);
        if inactive {
            assert!(
                matches!(result, Err(ApplicationError::InvalidSession)),
                "{result:?}"
            );
        } else {
            assert!(
                matches!(result, Err(ApplicationError::PermissionDenied)),
                "{result:?}"
            );
        }
        unchanged(&mut db, captured);
    }
}

#[test]
fn a_new_head_after_internal_prepare_is_not_overwritten_by_a_stale_commit() {
    let Some(mut db) = Fixture::new() else { return };
    let collection = DeadlineProfileCollection::Global;
    let owner = db.owner;
    let ordinary = service(&db, owner, Role::Owner);
    let first = persist(&ordinary, collection, publish(None));
    let stale = replace(&first);
    let draft = ordinary
        .prepare("session", collection, stale.clone())
        .unwrap();
    let winner_command = replace(&first);
    let winner_operation = winner_command.operation_id;
    let (workflow, captured) = watched(&db, owner, move |_| {
        persist(&ordinary, collection, winner_command.clone());
    });
    let result = workflow.submit("session", collection, stale, draft.submission_digest);
    assert!(
        matches!(
            result,
            Err(ApplicationError::DeadlineProfile(
                DeadlineProfileError::RevisionConflict
            ))
        ),
        "{result:?}"
    );
    unchanged(&mut db, captured);
    let head = service(&db, owner, Role::Owner)
        .get("session", collection, first.id, None)
        .unwrap();
    assert_eq!(head.revision.get(), 2);
    assert_eq!(head.receipt.operation_id, winner_operation);
}
