use super::header;
use application::{
    deadline_reevaluation::TechnicalService,
    deadlines::{DeadlineActorSnapshot, DeadlineError},
    ApplicationError,
};
use domain::identity::UserId;
use uuid::Uuid;

#[test]
fn legacy_actor_preserves_the_captured_human_identity() {
    let id = UserId::from_uuid(Uuid::nil());
    let author = DeadlineActorSnapshot::User {
        id,
        email: "owner@example.test".into(),
    };
    assert_eq!(
        header::legacy_actor(&author).unwrap(),
        (id, "owner@example.test")
    );
}

#[test]
fn legacy_actor_rejects_technical_authorship_with_a_typed_error() {
    let author = DeadlineActorSnapshot::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 1,
    };
    assert!(matches!(
        header::legacy_actor(&author),
        Err(ApplicationError::Deadline(
            DeadlineError::StoredInconsistent(_)
        ))
    ));
}
