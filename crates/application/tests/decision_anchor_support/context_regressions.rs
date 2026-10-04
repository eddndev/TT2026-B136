use crate::{
    context_support, decision_anchor_support::*, decision_support::*, history_support as hs,
};
use application::{
    cases::CaseRevision,
    hearings::{hearing_receipt_matches, HearingDetail},
    precautionary_hearings::PrecautionaryContext,
};
use domain::crypto::Sha256Digest;
use time::Duration;

fn contradictory_administration_reference() -> HearingDetail {
    let mut detail = ordinary_initial();
    let forged = Sha256Digest::from_array([99; 32]);
    detail.snapshot.scheduling_context.administration_digest = forged;
    detail.snapshot.recorded_administration_digest = forged;
    hearing_receipt_matches(&Hasher, &detail).unwrap();
    detail
}

#[test]
fn regression_ordinary_admin_reference_must_match_retained_stage_administration() {
    let mut fixture = fresh_no_change(3);
    fixture.material.context = crate::precautionary_receipt_support::later_context();
    fixture.command.context = expectation(&fixture.material.context);
    assert_eq!(
        fixture
            .material
            .context
            .material()
            .administration
            .revision
            .get(),
        2
    );
    assert_eq!(
        fixture
            .material
            .context
            .material()
            .stage_administration
            .revision
            .get(),
        1
    );
    let mut valid = fixture.clone();
    attach_initial(&mut valid, ordinary_initial());
    assert!(prepare(valid, &empty()).is_ok());
    attach_initial(&mut fixture, contradictory_administration_reference());
    assert!(
        prepare(fixture, &empty()).is_err(),
        "the Initial hearing's R1 digest contradicts the retained full R1 snapshot"
    );
}

fn context_without_revision_one() -> PrecautionaryContext {
    let mut material = context_support::changed(context_support::intermediate());
    material.stage_administration.revision = CaseRevision::new(2).unwrap();
    material.stage_administration.changed_at = context_support::at() + Duration::seconds(5);
    material.administration = material.stage_administration.clone();
    material.administration.revision = CaseRevision::new(3).unwrap();
    material.administration.changed_at = context_support::at() + Duration::seconds(20);
    let digest = material.stage_administration.values_digest;
    let stage = context_support::changed_mut(&mut material);
    stage.administration_revision = CaseRevision::new(2).unwrap();
    stage.administration_digest = digest;
    PrecautionaryContext::new(&Hasher, material).unwrap()
}

#[test]
fn regression_ordinary_admin_reference_must_match_an_ancestor_context_snapshot() {
    let prior = hs::root_fixture(1, 70).capture();
    let evidence = hs::history(vec![hs::entry(&prior, &empty())]);
    let previous = hs::member(&prior, 70);
    let mut fixture = fresh_no_change(3);
    fixture.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Confirm {
                previous: reference(&previous.capture),
            },
        ]))
        .unwrap();
    fixture.material.result_sources = vec![MeasureResultSources {
        id: previous.capture.result.id,
        sources: previous.capture.result.sources.clone(),
    }];
    fixture.material.predecessors = vec![previous];
    fixture.material.context = context_without_revision_one();
    fixture.command.context = expectation(&fixture.material.context);
    assert_eq!(
        fixture
            .material
            .context
            .material()
            .administration
            .revision
            .get(),
        3
    );
    assert_eq!(
        fixture
            .material
            .context
            .material()
            .stage_administration
            .revision
            .get(),
        2
    );
    let mut valid = fixture.clone();
    attach_initial(&mut valid, ordinary_initial());
    assert!(prepare(valid, &evidence).is_ok());
    attach_initial(&mut fixture, contradictory_administration_reference());
    assert!(
        prepare(fixture, &evidence).is_err(),
        "the Initial hearing's R1 digest contradicts the ancestor's full R1 snapshot"
    );
}
