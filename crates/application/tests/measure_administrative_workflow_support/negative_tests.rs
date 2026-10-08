use crate::measure_administrative_workflow_support::*;
use application::identity::Principal;
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[path = "bounds_tests.rs"]
mod bounds;
#[path = "confirmation_tests.rs"]
mod confirmation_tests;
#[path = "heads_dependants.rs"]
mod dependants;
#[path = "identity_clock_tests.rs"]
mod identity_clocks;
#[path = "result_tests.rs"]
mod results;

fn reject_before_admission(fixture: Fixture) {
    let harness = harness(fixture.store(), identity(fixture.actor.clone()));
    assert!(harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
    assert!(harness.events().is_empty());
    assert_eq!(harness.validator.calls(), 0);
}

fn base(fixture: &Fixture) -> MeasureDecisionGroupCapture {
    fixture.history.records.judicial.groups[0].capture.clone()
}

fn reject_returned(fixture: Fixture, returned: MeasureAdministrativeStoredOperation, replay: bool) {
    let expected = confirmation(&fixture.review());
    let store = if replay {
        fixture.replay_store(returned)
    } else {
        let mut store = fixture.store();
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, _| Ok(returned));
        store
    };
    let harness = harness(store, identity(fixture.actor));
    assert!(harness
        .service
        .submit("session", fixture.case_id, fixture.command, expected)
        .is_err());
}
