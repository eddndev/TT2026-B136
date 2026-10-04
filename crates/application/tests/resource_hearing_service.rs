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
mod resource_hearing_support;
use application::{resource_activities::*, ApplicationError};
use domain::{crypto::Sha256Digest, hearings::*, identity::Role, resource_hearings::*};
use resource_hearing_support::*;

#[test]
fn resource_hearing_preparation_does_not_require_or_fabricate_an_ordinary_stage() {
    let fixture = Fixture::new(Role::Owner);
    assert!(matches!(
        fixture.material.resource.recorded_stage,
        application::case_stages::CurrentCaseStage::Unregistered
    ));
    let draft = fixture.prepare().unwrap();
    assert_eq!(draft.command, fixture.command);
    assert_eq!(draft.resource, fixture.material.resource);
    assert_eq!(draft.act, fixture.material.act);
    assert_eq!(draft.observed_resource_head.revision.get(), 3);
    assert_eq!(
        draft.support.reference,
        fixture
            .command
            .values
            .scheduling_basis()
            .support()
            .reference()
    );
    assert_eq!(draft.recorded_by.id, fixture.actor.id);
}

#[test]
fn resource_hearing_only_accepts_a_compatible_explicit_resource_kind() {
    let mut fixture = Fixture::new(Role::Owner);
    fixture.change_values(|input| input.kind = ResourceHearingKind::AppealArguments);
    assert!(matches!(
        fixture.prepare_rejected(),
        Err(ApplicationError::InvalidInput(_))
    ));
}

#[test]
fn archived_resource_and_stale_head_cannot_prepare_a_new_hearing() {
    let mut fixture = Fixture::new(Role::Owner);
    fixture.command.expected_resource_revision = ResourceRevision::initial();
    assert!(matches!(
        fixture.prepare_rejected(),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::ResourceRevisionConflict
        ))
    ));
    let mut fixture = Fixture::new(Role::Owner);
    let head = fixture.material.resource_head.clone();
    let command = application::procedural_resources::ResourceCommand {
        operation_id: application::procedural_resources::ResourceOperationId::new(),
        resource_id: head.id,
        change: application::procedural_resources::ResourceChange::Archive {
            expected_revision: head.revision,
            reason: procedural_resource_support::text("Archived resource"),
        },
    };
    fixture.material.resource_head = procedural_resource_support::commit(
        &fixture.actor,
        fixture.case(),
        command,
        procedural_resource_support::retained(head),
    );
    fixture.command.expected_resource_revision = fixture.material.resource_head.revision;
    assert!(matches!(
        fixture.prepare_rejected(),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::ResourceArchived
        ))
    ));
}

#[test]
fn scheduling_basis_must_match_previously_admitted_exact_resource_support() {
    let mut fixture = Fixture::new(Role::Owner);
    fixture.change_values(|input| {
        let support = input.scheduling_basis.support();
        input.scheduling_basis = ResourceHearingSchedulingBasis::new(
            input.scheduling_basis.statement().clone(),
            HearingSupportRef::new(support.reference(), Sha256Digest::from_array([7; 32])),
        );
    });
    assert!(fixture.prepare_rejected().is_err());
}

#[test]
fn substituted_historical_resource_or_act_is_rejected() {
    for change_act in [false, true] {
        let mut fixture = Fixture::new(Role::Owner);
        if change_act {
            fixture.command.act.as_mut().unwrap().capture_digest =
                Sha256Digest::from_array([9; 32]);
        } else {
            fixture.command.resource.capture_digest = Sha256Digest::from_array([9; 32]);
        }
        assert!(fixture.prepare_rejected().is_err());
    }
}

#[test]
fn owner_and_litigator_can_review_but_read_only_roles_never_reach_the_store() {
    for role in [Role::Owner, Role::Litigator] {
        Fixture::new(role).prepare().unwrap();
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
fn principal_change_during_preparation_prevents_returning_the_review() {
    let fixture = Fixture::new(Role::Owner);
    let mut identity = case_support::MockIdentity::new();
    let mut sequence = mockall::Sequence::new();
    let actor = fixture.actor.clone();
    let mut changed = actor.clone();
    changed.email = "changed@example.com".into();
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(actor));
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(changed));
    assert!(matches!(
        service(fixture.store(), identity).prepare(
            "session",
            fixture.case(),
            fixture.resource(),
            fixture.command
        ),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn new_hearing_review_binds_identity_and_all_declared_values() {
    let fixture = Fixture::new(Role::Owner);
    let original = fixture.prepare().unwrap();
    let mut changed = fixture.clone();
    changed.command.association_id = ResourceActivityId::new();
    assert_ne!(
        original.submission_digest,
        changed.prepare().unwrap().submission_digest
    );
    let mut changed = fixture.clone();
    changed.change_values(|input| input.venue = HearingVenue::new("Other room").unwrap());
    assert_ne!(
        original.submission_digest,
        changed.prepare().unwrap().submission_digest
    );
    let mut changed = fixture.clone();
    changed.actor.email = "new@example.com".into();
    assert_ne!(
        original.submission_digest,
        changed.prepare().unwrap().submission_digest
    );
}

#[path = "resource_hearing_boundaries.rs"]
mod boundaries;

#[path = "resource_hearing_submission.rs"]
mod submission;

#[path = "resource_hearing_recovery.rs"]
mod recovery;

#[path = "resource_hearing_association.rs"]
mod association;
