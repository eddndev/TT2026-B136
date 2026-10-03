use super::*;
use application::{identity::Principal, resource_hearings::*};
use domain::{cases::CaseId, crypto::Sha256Digest, identity::UserId};
use std::sync::{Arc, Mutex};

struct LostResponseStore {
    material: ResourceHearingMaterial,
    persisted: Mutex<Option<ResourceHearingCreation>>,
    writes: Mutex<usize>,
}
impl ResourceHearingStore for LostResponseStore {
    fn prepare(
        &self,
        _: UserId,
        _: CaseId,
        _: ResourceId,
        _: &ResourceHearingCommand,
    ) -> Result<ResourceHearingPreparation, ApplicationError> {
        Ok(match self.persisted.lock().unwrap().clone() {
            Some(saved) => ResourceHearingPreparation::Replay(Box::new(saved)),
            None => ResourceHearingPreparation::Ready(Box::new(self.material.clone())),
        })
    }
    fn commit(
        &self,
        _: UserId,
        _: CaseId,
        _: ResourceId,
        p: PreparedResourceHearing,
    ) -> Result<ResourceHearingCreation, ApplicationError> {
        *self.writes.lock().unwrap() += 1;
        *self.persisted.lock().unwrap() = Some(p.into_creation(case_support::instant())?);
        Err(ApplicationError::InvalidInput(
            "simulated response lost after commit".into(),
        ))
    }
}

#[test]
fn explicit_reconciliation_after_lost_response_preserves_one_creation() {
    let f = Fixture::new(Role::Owner);
    let draft = f.prepare().unwrap();
    let store = Arc::new(LostResponseStore {
        material: f.material.clone(),
        persisted: Mutex::new(None),
        writes: Mutex::new(0),
    });
    let (identity, _) = procedural_resource_support::identity_for(f.actor.clone(), 2);
    let first = ResourceHearingService::new(
        store.clone(),
        Arc::new(identity),
        hearing_support::hasher(),
        Arc::new(case_support::CountingClock::default()),
    );
    assert!(first
        .submit(
            "session",
            f.case(),
            f.resource(),
            f.command.clone(),
            draft.submission_digest
        )
        .is_err());
    assert_eq!(*store.writes.lock().unwrap(), 1);
    // A distinct service instance models reconciliation after the caller restarts.
    let (identity, _) = procedural_resource_support::identity_for(f.actor.clone(), 2);
    let resumed = ResourceHearingService::new(
        store.clone(),
        Arc::new(identity),
        hearing_support::hasher(),
        Arc::new(case_support::CountingClock::default()),
    );
    let recovered = resumed
        .submit(
            "session",
            f.case(),
            f.resource(),
            f.command.clone(),
            draft.submission_digest,
        )
        .unwrap();
    assert_eq!(Some(recovered), *store.persisted.lock().unwrap());
    assert_eq!(*store.writes.lock().unwrap(), 1);
}

fn creation(f: &Fixture) -> ResourceHearingCreation {
    prepare_resource_hearing_change(
        hearing_support::hasher(),
        &f.actor,
        f.case(),
        f.resource(),
        f.command.clone(),
        f.material.clone(),
    )
    .unwrap()
    .into_creation(case_support::instant())
    .unwrap()
}
fn replay(
    f: &Fixture,
    saved: ResourceHearingCreation,
    actor: Principal,
    count: usize,
    command: ResourceHearingCommand,
    digest: Sha256Digest,
) -> Result<ResourceHearingCreation, ApplicationError> {
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(ResourceHearingPreparation::Replay(Box::new(saved))));
    let (identity, _) = procedural_resource_support::identity_for(actor, count);
    service(store, identity).submit("session", f.case(), f.resource(), command, digest)
}
#[test]
fn replay_requires_exact_intent_but_retains_the_original_historical_author() {
    let f = Fixture::new(Role::Owner);
    let saved = creation(&f);
    let digest = saved.hearing.review.submission_digest;
    let mut changed = f.clone();
    changed.command.operation_id = ResourceHearingOperationId::new();
    assert!(replay(
        &f,
        saved.clone(),
        f.actor.clone(),
        1,
        changed.command,
        digest
    )
    .is_err());
    assert!(replay(
        &f,
        saved.clone(),
        f.actor.clone(),
        1,
        f.command.clone(),
        Sha256Digest::from_array([9; 32])
    )
    .is_err());
    let mut renamed = f.actor.clone();
    renamed.email = "current@example.com".into();
    let actual = replay(&f, saved.clone(), renamed, 2, f.command.clone(), digest).unwrap();
    assert_eq!(actual, saved);
    assert_eq!(actual.hearing.review.recorded_by.email, f.actor.email);
}
#[test]
fn future_receipt_and_lost_authority_during_replay_are_rejected() {
    let f = Fixture::new(Role::Owner);
    let future = prepare_resource_hearing_change(
        hearing_support::hasher(),
        &f.actor,
        f.case(),
        f.resource(),
        f.command.clone(),
        f.material.clone(),
    )
    .unwrap()
    .into_creation(case_support::instant() + time::Duration::hours(1))
    .unwrap();
    let digest = future.hearing.review.submission_digest;
    assert!(replay(&f, future, f.actor.clone(), 1, f.command.clone(), digest).is_err());
    let saved = creation(&f);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(ResourceHearingPreparation::Replay(Box::new(saved))));
    let mut identity = case_support::MockIdentity::new();
    let mut order = mockall::Sequence::new();
    let actor = f.actor.clone();
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut order)
        .return_once(move |_| Ok(actor));
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut order)
        .return_once(|_| Err(ApplicationError::InvalidSession));
    assert!(matches!(
        service(store, identity).submit("session", f.case(), f.resource(), f.command, digest),
        Err(ApplicationError::InvalidSession)
    ));
}
