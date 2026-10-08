use crate::correction_support::*;
use application::{cases::CaseRevision, precautionary_hearings::PrecautionaryContext};
use domain::{
    cases::CaseId,
    crypto::{DocumentId, Sha256Digest},
    hearings::HearingSupportRef,
};
use time::Duration;
use uuid::Uuid;

#[path = "bounds_tests.rs"]
mod bounds;
#[path = "reconstruction_tests.rs"]
mod reconstruction;

fn later_with_different_support() -> CorrectionFixture {
    let initial = crate::measure_decision_fixtures::Fixture::single().capture();
    let mut next = crate::effect_support::LaterFixture::confirm(&initial);
    let support = &mut next.request.material.support;
    support.reference.id = DocumentId::from_uuid(Uuid::from_u128(92));
    support.digest = Sha256Digest::from_array([12; 32]);
    support.name = "later-decision.pdf".into();
    let mut input = crate::measure_decision_fixtures::decision_input(&next.request.command.values);
    input.support = HearingSupportRef::new(support.reference, support.digest);
    next.request.command.values = MeasureDecisionValues::new(input);
    let ancestors = next.evidence.clone();
    let group = next.capture();
    CorrectionFixture::from_group(&group, &ancestors, group.measures[0].result.id)
}

#[test]
fn correction_requires_exact_case_measure_revision_and_digest() {
    for mutation in 0..4 {
        let mut fixture = CorrectionFixture::initial();
        let target = fixture.command.target;
        match mutation {
            0 => fixture.case_id = CaseId::from_uuid(Uuid::from_u128(999)),
            1 => {
                fixture.command.target =
                    PrecautionaryMeasureRef::new(id(999), target.revision(), target.digest())
            }
            2 => {
                fixture.command.target = PrecautionaryMeasureRef::new(
                    target.id(),
                    MeasureRevision::new(2).unwrap(),
                    target.digest(),
                )
            }
            _ => {
                fixture.command.target = PrecautionaryMeasureRef::new(
                    target.id(),
                    target.revision(),
                    Sha256Digest::from_array([99; 32]),
                )
            }
        }
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn correction_context_expectation_binds_both_revisions_and_full_material() {
    for mutation in 0..3 {
        let mut fixture = CorrectionFixture::initial();
        match mutation {
            0 => fixture.command.context.administration_revision = CaseRevision::new(2).unwrap(),
            1 => {
                fixture.command.context.stage_revision =
                    application::case_stages::CaseStageRevision::new(2).unwrap()
            }
            _ => fixture.command.context.context_digest = Sha256Digest::from_array([99; 32]),
        }
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn correction_rejects_missing_duplicate_or_corrupt_origin_bound_history() {
    for mutation in 0..4 {
        let mut fixture = CorrectionFixture::initial();
        let groups = &mut fixture.history.groups;
        match mutation {
            0 => groups.clear(),
            1 => groups.push(groups[0].clone()),
            2 => groups[0].origin.group_digest = Sha256Digest::from_array([99; 32]),
            _ => groups[0].capture.measures[0].capture_digest = Sha256Digest::from_array([99; 32]),
        }
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn correcting_one_member_still_requires_its_whole_owning_group() {
    let group = crate::measure_decision_fixtures::Fixture::multiple().capture();
    let empty = crate::effect_support::empty_history();
    for omit in [false, true] {
        let mut fixture =
            CorrectionFixture::from_group(&group, &empty, group.measures[0].result.id);
        let entry = &mut fixture.history.groups[0];
        if omit {
            entry.capture.measures.pop();
        } else {
            entry.capture.measures[1]
                .result
                .projection
                .subject
                .display_name = "Invented sibling".into();
        }
        crate::measure_decision_fixtures::refresh_digests(&mut entry.capture);
        entry.origin.group_digest = entry.capture.capture_digest;
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn correction_cannot_reuse_a_judicial_operation_identity() {
    let mut fixture = CorrectionFixture::initial();
    fixture.command.operation_id = MeasureCorrectionOperationId::from_uuid(
        fixture
            .previous_group()
            .review
            .command
            .operation_id
            .as_uuid(),
    );
    assert!(fixture.prepare().is_err());
}

#[test]
fn correction_context_cannot_regress_known_administration_or_its_clock() {
    let mut request = crate::measure_decision_fixtures::Fixture::single();
    let mut material = request.material.context.material().clone();
    material.administration.revision = CaseRevision::new(2).unwrap();
    material.administration.changed_at = crate::context_support::at() + Duration::seconds(10);
    request.material.context = PrecautionaryContext::new(&Hasher, material).unwrap();
    request.command.context = expectation(&request.material.context);
    let group = request.capture();
    for revision in [1, 3] {
        let mut fixture = CorrectionFixture::from_group(
            &group,
            &crate::effect_support::empty_history(),
            group.measures[0].result.id,
        );
        let mut material = crate::context_support::initial();
        if revision == 3 {
            material.administration.revision = CaseRevision::new(3).unwrap();
            material.administration.changed_at =
                crate::context_support::at() + Duration::seconds(5);
        }
        fixture.context = PrecautionaryContext::new(&Hasher, material).unwrap();
        fixture.command.context = expectation(&fixture.context);
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn independently_valid_new_context_cannot_rewrite_historical_stage_administration() {
    let mut fixture = CorrectionFixture::initial();
    let mut material = crate::context_support::initial();
    material.administration.revision = CaseRevision::new(2).unwrap();
    material.administration.changed_at += Duration::seconds(1);
    material.stage_administration.changed_by.email = "different historic stage recorder".into();
    let application::case_stages::CaseStageEntry::Initial(stage) = &mut material.stage else {
        unreachable!()
    };
    stage.recorded_by = material.stage_administration.changed_by.clone();
    fixture.context = PrecautionaryContext::new(&Hasher, material).unwrap();
    fixture.command.context = expectation(&fixture.context);
    assert!(fixture.prepare().is_err());
}

#[test]
fn new_context_support_must_agree_with_every_ancestor_not_only_the_selected_owner() {
    use domain::case_stages::{
        CaseStageChange, DeclaredStageTime, StageSupportRef, StageTransition,
    };
    let mut fixture = later_with_different_support();
    let support = fixture.history.groups[0]
        .capture
        .review
        .material
        .support
        .clone();
    let values = CaseStageChange::Transition(StageTransition::to_intermediate(
        DeclaredStageTime::instant(crate::context_support::at()).unwrap(),
        StageSupportRef::new(support.reference, support.digest),
        None,
    ));
    let mut material = crate::context_support::changed(values);
    crate::context_support::changed_mut(&mut material).supports[0].name =
        "contradictory-old-support.pdf".into();
    fixture.context = PrecautionaryContext::new(&Hasher, material).unwrap();
    fixture.command.context = expectation(&fixture.context);
    assert_ne!(
        support.reference,
        fixture.previous_group().review.material.support.reference
    );
    assert!(fixture.prepare().is_err());
}
