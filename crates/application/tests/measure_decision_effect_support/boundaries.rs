use super::*;
use application::cases::CaseRevision;
use application::precautionary_hearings::PrecautionaryContext;
use time::Duration;

#[test]
fn higher_administration_revision_cannot_regress_its_capture_clock() {
    let mut initial = Fixture::single();
    let mut before = crate::context_support::initial();
    before.administration.revision = CaseRevision::new(2).unwrap();
    before.administration.changed_at += Duration::seconds(20);
    initial.material.context = PrecautionaryContext::new(&Hasher, before).unwrap();
    initial.command.context = expectation(&initial.material.context);
    let previous = initial.capture();
    let mut fixture = LaterFixture::confirm(&previous);
    let mut after = fixture.request.material.context.material().clone();
    after.administration.revision = CaseRevision::new(3).unwrap();
    after.administration.changed_at -= Duration::seconds(1);
    fixture.request.material.context = PrecautionaryContext::new(&Hasher, after).unwrap();
    fixture.request.command.context = expectation(&fixture.request.material.context);
    assert!(fixture.prepare().is_err());
}

#[test]
fn higher_stage_revision_cannot_regress_its_capture_clock() {
    let mut initial = Fixture::single();
    let mut before = crate::context_support::changed(crate::context_support::intermediate());
    crate::context_support::changed_mut(&mut before).recorded_at =
        crate::context_support::at() + Duration::seconds(20);
    initial.material.context = PrecautionaryContext::new(&Hasher, before).unwrap();
    initial.command.context = expectation(&initial.material.context);
    let previous = initial.capture();
    let mut fixture = LaterFixture::confirm(&previous);
    let mut after = crate::context_support::changed(crate::context_support::trial());
    crate::context_support::changed_mut(&mut after).recorded_at =
        crate::context_support::at() + Duration::seconds(19);
    fixture.request.material.context = PrecautionaryContext::new(&Hasher, after).unwrap();
    fixture.request.command.context = expectation(&fixture.request.material.context);
    assert!(fixture.prepare().is_err());
}

#[test]
fn later_group_clock_cannot_predate_its_actual_owning_predecessor_group() {
    let previous = Fixture::single().capture();
    let fixture = LaterFixture::confirm(&previous);
    assert!(fixture
        .clone()
        .prepare()
        .unwrap()
        .into_group_capture(&Hasher, previous.recorded_at - Duration::nanoseconds(1),)
        .is_err());
    fixture
        .prepare()
        .unwrap()
        .into_group_capture(&Hasher, previous.recorded_at)
        .unwrap();
}
