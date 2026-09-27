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
mod resource_deadline_support;
use application::{deadlines::*, resource_activities::*, resource_deadlines::*, ApplicationError};
use domain::{crypto::Sha256Digest, identity::Role};
use resource_deadline_support::*;

#[test]
fn preview_binds_exact_resource_and_act_to_prospective_deadline_capture() {
    let fixture = Fixture::new(Role::Owner);
    let draft = fixture.preview();
    assert_eq!(draft.command, fixture.command);
    assert_eq!(draft.association.resource, fixture.material.resource);
    assert_eq!(draft.association.act, fixture.material.act);
    assert_eq!(draft.association.observed_resource_head.revision.get(), 3);
    let ResourceActivityChange::Link { selection } = draft.association.command.change else {
        panic!("only linking is allowed");
    };
    assert_eq!(selection.resource, fixture.command.resource);
    assert_eq!(selection.act, fixture.command.act);
    assert_eq!(
        selection.target,
        ResourceActivityTarget::Deadline {
            id: draft.deadline.command.deadline_id,
            revision: draft.deadline.result_revision,
            capture_digest: draft.deadline.capture_digest,
        }
    );
    assert_eq!(
        draft.association.command.operation_id.as_uuid(),
        draft.deadline.command.operation_id.as_uuid()
    );
    assert!(matches!(
        draft.deadline.receipt_version,
        DeadlineReceiptVersion::Tracked(_)
    ));
}

#[test]
fn all_reviewed_deadline_content_and_association_identity_bind_confirmation() {
    let fixture = Fixture::new(Role::Owner);
    let original = fixture.preview();
    let mut changed = fixture.clone();
    let (mut command, policies) = changed.command.deadline.into_parts();
    let DeadlineChange::Register { definition } = &mut command.change else {
        unreachable!()
    };
    definition.title = domain::procedural_facts::FactLabel::new("Another declared title").unwrap();
    changed.command.deadline = DeadlineHumanCommand::new(command, policies).unwrap();
    let modified = changed.preview();
    assert_ne!(
        modified.deadline.capture_digest,
        original.deadline.capture_digest
    );
    assert_ne!(
        modified.association.submission_digest,
        original.association.submission_digest
    );
    assert_ne!(modified.submission_digest, original.submission_digest);
    changed.command.association_id = ResourceActivityId::new();
    assert_ne!(
        changed.preview().submission_digest,
        modified.submission_digest
    );
    let workflow = fixture.service(1);
    assert!(matches!(
        workflow.submit(
            "session",
            fixture.case(),
            fixture.resource(),
            fixture.command.clone(),
            Sha256Digest::from_array([9; 32])
        ),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::SubmissionMismatch
        ))
    ));
}

#[test]
fn only_owner_and_litigator_can_prepare_contextual_registration() {
    for role in [Role::Owner, Role::Litigator] {
        assert_eq!(
            Fixture::new(role).preview().deadline.result_revision.get(),
            1
        );
    }
    for role in [Role::Paralegal, Role::Client] {
        let fixture = Fixture::new(role);
        let (identity, _) = procedural_resource_support::identity_for(fixture.actor.clone(), 1);
        assert!(matches!(
            service(MockStore::new(), identity).prepare(
                "session",
                fixture.case(),
                fixture.resource(),
                fixture.command
            ),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}

#[test]
fn non_registration_and_scope_substitution_fail_before_commit() {
    let fixture = Fixture::new(Role::Owner);
    let (mut command, _) = fixture.command.deadline.clone().into_parts();
    command.change = DeadlineChange::Retire {
        expected_revision: DeadlineRevision::initial(),
        reason: deadline_support::evaluation::text("Not a creation"),
    };
    let mut combined = fixture.command.clone();
    combined.deadline = DeadlineHumanCommand::new(command, None).unwrap();
    let (identity, _) = procedural_resource_support::identity_for(fixture.actor.clone(), 1);
    assert!(service(MockStore::new(), identity)
        .prepare("session", fixture.case(), fixture.resource(), combined)
        .is_err());
    let mut changed = fixture.clone();
    changed.command.resource.capture_digest = Sha256Digest::from_array([7; 32]);
    assert!(changed
        .service(1)
        .prepare(
            "session",
            changed.case(),
            changed.resource(),
            changed.command
        )
        .is_err());
}

#[test]
fn session_revocation_after_preparation_prevents_atomic_commit() {
    let fixture = Fixture::new(Role::Owner);
    let digest = fixture.preview().submission_digest;
    let mut identity = case_support::MockIdentity::new();
    let mut sequence = mockall::Sequence::new();
    let actor = fixture.actor.clone();
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(actor));
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(|_| Err(ApplicationError::InvalidSession));
    assert!(matches!(
        service(fixture.store(), identity).submit(
            "session",
            fixture.case(),
            fixture.resource(),
            fixture.command,
            digest
        ),
        Err(ApplicationError::InvalidSession)
    ));
}
