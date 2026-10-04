use application::case_stages::*;
use application::precautionary_hearings::*;
use domain::crypto::Sha256Digest;
use domain::hearings::HearingSupportRef;
use domain::precautionary_hearings::{
    PrecautionaryHearingSchedulingBasis, PrecautionaryHearingValues,
    PrecautionaryHearingValuesInput,
};
use time::Duration;

use crate::context_support;
use crate::precautionary_receipt_support::*;

fn replace_declared_support(fixture: &mut Fixture) {
    let PrecautionaryHearingChange::Replace { values, .. } = &mut fixture.command.change else {
        panic!("replacement fixture expected")
    };
    let selected = &fixture.sources.support;
    let basis = values.scheduling_basis();
    *values = PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
        purpose: values.purpose(),
        scheduled_at: values.scheduled_at(),
        modality: values.modality(),
        venue: values.venue().clone(),
        note: values.note().cloned(),
        participants: values.participants().to_vec(),
        scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
            basis.statement().clone(),
            HearingSupportRef::new(selected.reference, selected.digest),
            basis.locator().clone(),
        ),
        review_targets: values.review_targets().to_vec(),
    })
    .unwrap();
}

fn mutate_support(support: &mut StageSupportSnapshot, field: usize) {
    match field {
        0 => support.name = "contradictory-name.pdf".into(),
        1 => support.format = StageDocumentFormat::Docx,
        _ => support.digest = Sha256Digest::from_array([99; 32]),
    }
}

fn set_context(fixture: &mut Fixture, context: PrecautionaryContext) {
    match &mut fixture.command.change {
        PrecautionaryHearingChange::Schedule {
            context: expected, ..
        }
        | PrecautionaryHearingChange::Replace {
            context: expected, ..
        } => *expected = expectation(&context),
        PrecautionaryHearingChange::Cancel { .. } => {}
    }
    fixture.context = context;
}

fn stage_context(support: StageSupportSnapshot, trial: bool) -> PrecautionaryContext {
    let reference = StageSupportRef::new(support.reference, support.digest);
    let declared = DeclaredStageTime::instant(context_support::at()).unwrap();
    let change = if trial {
        StageTransition::to_trial(
            declared,
            reference,
            declared,
            StageCourt::new("Trial court").unwrap(),
            None,
            None,
            None,
        )
        .unwrap()
    } else {
        StageTransition::to_intermediate(declared, reference, None)
    };
    let mut material = context_support::changed(CaseStageChange::Transition(change));
    let stage = context_support::changed_mut(&mut material);
    stage.supports = vec![support];
    if trial {
        stage.recorded_at += Duration::seconds(1);
    }
    PrecautionaryContext::new(&Hasher, material).unwrap()
}

#[test]
fn regression_replacement_preserves_metadata_for_the_same_exact_support_reference() {
    let prior = scheduled();
    for field in 0..3 {
        let mut fixture = Fixture::replace(&prior);
        mutate_support(&mut fixture.sources.support, field);
        replace_declared_support(&mut fixture);
        let mut standalone = fixture.clone();
        let PrecautionaryHearingChange::Replace { values, .. } = &fixture.command.change else {
            unreachable!()
        };
        standalone.command.change = PrecautionaryHearingChange::Schedule {
            context: expectation(&standalone.context),
            values: values.clone(),
        };
        assert!(standalone.prepare(None).is_ok());
        assert!(fixture.prepare(Some(&prior)).is_err());
    }
}

#[test]
fn regression_direct_and_stage_supports_agree_when_they_share_an_exact_reference() {
    let original = Fixture::schedule();
    let mut consistent = original.clone();
    set_context(
        &mut consistent,
        stage_context(original.sources.support.clone(), false),
    );
    assert!(consistent.prepare(None).is_ok());
    for field in 0..3 {
        let mut fixture = original.clone();
        let mut conflicting_support = fixture.sources.support.clone();
        mutate_support(&mut conflicting_support, field);
        set_context(&mut fixture, stage_context(conflicting_support, false));
        assert_eq!(
            fixture.sources.support.reference,
            original.sources.support.reference
        );
        assert!(fixture.prepare(None).is_err());
    }
}

#[test]
fn regression_support_metadata_is_immutable_across_retained_and_observed_stages() {
    let material = context_support::changed(context_support::intermediate());
    let CaseStageEntry::Changed(stage) = &material.stage else {
        unreachable!()
    };
    let support = stage.supports[0].clone();
    let mut fixture = Fixture::schedule();
    set_context(&mut fixture, stage_context(support.clone(), false));
    let prior = fixture.capture(None, at());
    let mut consistent = Fixture::cancel(&prior);
    set_context(&mut consistent, stage_context(support.clone(), true));
    assert!(consistent.prepare(Some(&prior)).is_ok());
    for field in 0..3 {
        let mut fixture = Fixture::cancel(&prior);
        let mut contradictory = support.clone();
        mutate_support(&mut contradictory, field);
        set_context(&mut fixture, stage_context(contradictory, true));
        assert!(fixture.prepare(Some(&prior)).is_err());
    }
}
