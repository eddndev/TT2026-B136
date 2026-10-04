use crate::{
    hearing_derived_deadline_capture_support as capture,
    hearing_derived_deadline_support::{case_id, Fixture},
    hearing_result_support as results,
};
use application::{
    documents::StageSupportReadLimits, hearing_derived_deadlines::*, identity::Principal,
    ApplicationError,
};
use domain::{cases::CaseId, identity::UserId};
use mockall::mock;
use std::sync::Arc;

mock! {
    pub Store {}
    impl HearingDerivedDeadlineStore for Store {
        fn prepare(&self,actor:UserId,case:CaseId,command:&HearingDerivedDeadlineCommand,limits:&StageSupportReadLimits)->Result<HearingDerivedDeadlinePreparation,ApplicationError>;
        fn commit(&self,actor:UserId,case:CaseId,prepared:PreparedHearingDerivedDeadline)->Result<HearingDerivedDeadlineRecord,ApplicationError>;
    }
}

pub fn inputs(fixture: &Fixture) -> HearingDerivedDeadlineInputs {
    HearingDerivedDeadlineInputs {
        actor: fixture.actor.clone(),
        result: fixture.result_preparation.clone(),
        profile: fixture.material.profile.clone(),
        profile_head: fixture.material.profile_head.clone(),
        calendar: fixture.material.calendar.clone(),
        calendar_head: fixture.material.calendar_head.clone(),
        responsible: fixture.material.responsible.clone(),
    }
}

pub fn record(fixture: &Fixture) -> HearingDerivedDeadlineRecord {
    let draft = fixture.prepare().unwrap();
    let source = capture::recorded_source(fixture, crate::case_support::instant());
    let event = capture::source_event(&source);
    let creation =
        finalize_hearing_derived_deadline(results::hasher().as_ref(), &draft, source, event)
            .unwrap();
    restore_hearing_derived_deadline(results::hasher().as_ref(), creation.evidence()).unwrap()
}

pub fn identity(actor: &Principal, calls: usize) -> results::MockIdentity {
    let mut identity = results::MockIdentity::new();
    let actor = actor.clone();
    identity
        .expect_authenticate()
        .times(calls)
        .returning(move |_| Ok(actor.clone()));
    identity
}

pub fn ready(store: &mut MockStore, fixture: &Fixture) {
    let input = inputs(fixture);
    let command = fixture.command.clone();
    let actor = fixture.actor.id;
    store
        .expect_prepare()
        .withf(move |id, case, value, _| *id == actor && *case == case_id() && *value == command)
        .times(1)
        .return_once(move |_, _, _, _| {
            Ok(HearingDerivedDeadlinePreparation::Ready(Box::new(input)))
        });
}

pub fn service(
    store: MockStore,
    identity: results::MockIdentity,
    validator: Arc<results::Validator>,
) -> (HearingDerivedDeadlineService, Arc<results::CountingClock>) {
    let clock = Arc::new(results::CountingClock::default());
    (
        HearingDerivedDeadlineService::new(
            Arc::new(store),
            Arc::new(identity),
            Arc::new(crate::crypto::processor()),
            validator,
            results::hasher(),
            clock.clone(),
        ),
        clock,
    )
}
