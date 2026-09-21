#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_support;
#[allow(dead_code)]
mod hearing_support;
use deadline_support::evaluation::inputs::facts as procedural_fact_service_support;
mod procedural_resource_support;
mod resource_activity_support;
use application::{resource_activities::*, ApplicationError};
use domain::{crypto::Sha256Digest, identity::Role};
use procedural_resource_support::identity_for;
use resource_activity_support::*;

#[test]
fn linking_retains_resource_r1_and_original_act_after_its_correction() {
    let (_, actor) = case_support::identity(Role::Litigator, 0);
    let fixture = Fixture::new(&actor);
    let draft = fixture.draft(&actor);
    assert_eq!(draft.selection.resource.revision.get(), 1);
    assert_eq!(draft.selection.act.unwrap().resource_revision.get(), 2);
    assert_eq!(draft.selection.act.unwrap().revision.get(), 1);
    assert_eq!(draft.observed_resource_head.revision.get(), 3);
    assert_eq!(draft.sources, fixture.material.sources);
    assert_eq!(draft.recorded_by.id, actor.id);
    assert_eq!(draft.recorded_by.email, actor.email);
    assert_eq!(draft.status, ResourceActivityStatus::Linked);
    assert_eq!(draft.result_revision, ResourceActivityRevision::initial());
    assert_eq!(draft.previous, None);
}

#[test]
fn commit_and_exact_replay_keep_one_receipt_and_do_not_commit_twice() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&actor);
    let original = fixture.committed(&actor);
    resource_activity_receipt_matches(hearing_support::hasher().as_ref(), &original).unwrap();
    assert_eq!(original.sources, fixture.material.sources);
    let expected = original.receipt.submission_digest;
    let replayed = original.clone();
    let (identity, _) = identity_for(actor.clone(), 2);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(ResourceActivityPreparation::Replay(Box::new(replayed))));
    let actual = service(store, identity)
        .submit(
            "session",
            fixture.case_id,
            fixture.resource_id,
            fixture.command.clone(),
            expected,
        )
        .unwrap();
    assert_eq!(actual, original);
    let mut changed = original;
    changed.recorded_by.email = "different@example.test".into();
    assert!(
        resource_activity_receipt_matches(hearing_support::hasher().as_ref(), &changed).is_err()
    );
}

#[test]
fn unlink_archived_resource_preserves_sources_and_all_independent_activity_bytes() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let mut fixture = Fixture::new(&actor);
    let linked = fixture.committed(&actor);
    let original_selection = linked.selection;
    let original_sources = linked.sources.clone();
    fixture.archive_resource(&actor);
    fixture.unlink(linked.clone());
    let unlinked = fixture.committed(&actor);
    assert_eq!(unlinked.status, ResourceActivityStatus::Unlinked);
    assert_eq!(unlinked.revision.get(), 2);
    assert_eq!(unlinked.selection, original_selection);
    assert_eq!(unlinked.sources, original_sources);
    assert_eq!(
        unlinked.receipt.previous.unwrap().capture_digest,
        linked.receipt.capture_digest
    );
    assert_eq!(linked.status, ResourceActivityStatus::Linked);
    assert_eq!(linked.revision.get(), 1);
}

#[test]
fn new_link_rejects_an_archived_resource_head_but_never_silently_substitutes_it() {
    let (identity, actor) = case_support::identity(Role::Owner, 1);
    let mut fixture = Fixture::new(&actor);
    fixture.archive_resource(&actor);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| {
            Ok(ResourceActivityPreparation::Ready(Box::new(
                fixture.material,
            )))
        });
    assert!(matches!(
        service(store, identity).prepare(
            "session",
            fixture.case_id,
            fixture.resource_id,
            fixture.command
        ),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::ResourceArchived
        ))
    ));
}

#[test]
fn reviewed_digest_and_operation_reuse_cannot_confirm_another_selection() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let fixture = Fixture::new(&actor);
    let original = fixture.committed(&actor);
    let (identity, _) = identity_for(actor.clone(), 1);
    let mut store = MockStore::new();
    let material = fixture.material.clone();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(ResourceActivityPreparation::Ready(Box::new(material))));
    assert!(matches!(
        service(store, identity).submit(
            "session",
            fixture.case_id,
            fixture.resource_id,
            fixture.command.clone(),
            Sha256Digest::from_array([9; 32]),
        ),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::SubmissionMismatch
        ))
    ));
    let (identity, _) = identity_for(actor, 1);
    let mut store = MockStore::new();
    let expected = original.receipt.submission_digest;
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(ResourceActivityPreparation::Replay(Box::new(original))));
    let mut changed = fixture.command;
    changed.association_id = ResourceActivityId::new();
    assert!(matches!(
        service(store, identity).submit(
            "session",
            fixture.case_id,
            fixture.resource_id,
            changed,
            expected,
        ),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::OperationConflict
        ))
    ));
}

#[test]
fn linking_and_unlinking_a_deadline_preserve_its_capture_calculation_and_attention() {
    let (_, actor) = case_support::identity(Role::Owner, 0);
    let mut fixture = Fixture::new(&actor);
    fixture.use_deadline();
    let original = fixture.material.sources.target.clone();
    let linked = fixture.committed(&actor);
    assert!(matches!(
        linked.selection.target,
        ResourceActivityTarget::Deadline { .. }
    ));
    assert_eq!(linked.sources.target, original);
    fixture.unlink(linked);
    let unlinked = fixture.committed(&actor);
    assert_eq!(unlinked.sources.target, original);
    assert_eq!(unlinked.status, ResourceActivityStatus::Unlinked);
}
