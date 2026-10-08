use crate::{decision_support::*, measure_history_support::*};
use application::case_stages::StageDocumentFormat;
use application::precautionary_hearings::PrecautionaryContext;
use domain::clock::OffsetDateTime;
use domain::hearings::HearingSupportRef;
use domain::identity::UserId;
use time::Duration;
use uuid::Uuid;

fn reject_pair(first: &MeasureDecisionGroupCapture, second: &MeasureDecisionGroupCapture) {
    let evidence = history(vec![entry(first, &empty()), entry(second, &empty())]);
    let selected = [
        reference(&first.measures[0]),
        reference(&second.measures[0]),
    ];
    assert!(resolve_measure_targets(&Hasher, first.review.case_id, &selected, &evidence).is_err());
}

#[test]
fn full_subject_identity_remains_immutable_across_separately_valid_groups() {
    let first = root_fixture(1, 70).capture();
    for mutation in 0..3 {
        let mut fixture = root_fixture(2, 80);
        let source = &mut fixture.material.result_sources[0].sources.subject;
        match mutation {
            0 => source.changed_by.email = "other historical actor".into(),
            1 => source.changed_by.id = UserId::from_uuid(Uuid::from_u128(99)),
            _ => source.changed_at -= Duration::seconds(1),
        }
        let second = fixture.capture();
        reject_pair(&first, &second);
    }
}

#[test]
fn full_participant_identity_remains_immutable_across_separately_valid_groups() {
    let first = root_fixture(1, 70).capture();
    for mutation in 0..3 {
        let mut fixture = root_fixture(2, 80);
        let supervisor = fixture.material.result_sources[0]
            .sources
            .supervisor
            .as_mut()
            .unwrap();
        let source = crate::participant_support::typed_mut(supervisor);
        match mutation {
            0 => source.changed_by.email = "other historical actor".into(),
            1 => source.submission_digest = domain::crypto::Sha256Digest::from_array([99; 32]),
            _ => source.changed_at -= Duration::seconds(1),
        }
        reject_pair(&first, &fixture.capture());
    }
}

#[test]
fn full_bound_subject_identity_remains_immutable_across_separately_valid_groups() {
    let first = root_fixture(1, 70).capture();
    let mut fixture = root_fixture(2, 80);
    let supervisor = fixture.material.result_sources[0]
        .sources
        .supervisor
        .as_mut()
        .unwrap();
    crate::participant_support::typed_mut(supervisor).id =
        crate::participant_support::reference(8, 3).id();
    supervisor.bound_subject.as_mut().unwrap().changed_by.email = "other historical actor".into();
    let values = MeasureValues::new(crate::measure_source_support::input(
        &fixture.material.result_sources[0].sources,
    ));
    fixture.replace_values(id(80), values);
    reject_pair(&first, &fixture.capture());
}

#[test]
fn full_administration_and_stage_provenance_remain_immutable_across_groups() {
    let first = root_fixture(1, 70).capture();
    let mut fixture = root_fixture(2, 80);
    let mut context = fixture.material.context.material().clone();
    context.administration.changed_by.email = "another case recorder".into();
    context.stage_administration.changed_by = context.administration.changed_by.clone();
    let application::case_stages::CaseStageEntry::Initial(initial) = &mut context.stage else {
        panic!("initial context expected")
    };
    initial.recorded_by = context.administration.changed_by.clone();
    fixture.material.context = PrecautionaryContext::new(&Hasher, context).unwrap();
    fixture.command.context = expectation(&fixture.material.context);
    reject_pair(&first, &fixture.capture());
}

#[test]
fn exact_document_metadata_remains_immutable_across_groups() {
    let first = root_fixture(1, 70).capture();
    for mutation in 0..3 {
        let mut fixture = root_fixture(2, 80);
        match mutation {
            0 => fixture.material.support.name = "renamed.pdf".into(),
            1 => fixture.material.support.format = StageDocumentFormat::Docx,
            _ => {
                fixture.material.support.digest = domain::crypto::Sha256Digest::from_array([99; 32])
            }
        }
        let mut values = decision_input(&fixture.command.values);
        values.support = HearingSupportRef::new(
            fixture.material.support.reference,
            fixture.material.support.digest,
        );
        fixture.command.values = MeasureDecisionValues::new(values);
        reject_pair(&first, &fixture.capture());
    }
}

#[test]
fn a_predecessor_clock_cannot_be_hidden_by_rehashing_its_descendant() {
    let (mut group, mut evidence) = three_revisions();
    let early = evidence.groups[0].capture.recorded_at - Duration::nanoseconds(1);
    group.recorded_at = early;
    group.decision.recorded_at = early;
    for member in &mut group.measures {
        member.recorded_at = early;
    }
    refresh_digests(&mut group);
    for member in &mut group.measures {
        member.decision_digest = group.decision.capture_digest;
    }
    refresh_digests(&mut group);
    evidence.groups.pop();
    assert!(measure_group_origin(&Hasher, &group, &evidence).is_err());
    evidence.groups.push(claimed_entry(group.clone()));
    rejects(&group, &evidence);
}

#[test]
fn capture_offsets_remain_original_utc_even_through_history_resolution() {
    let (group, evidence) = three_revisions();
    let targets = resolve_measure_targets(
        &Hasher,
        group.review.case_id,
        &[reference(&group.measures[0])],
        &evidence,
    )
    .unwrap();
    let capture = &targets.targets()[0].capture;
    assert_eq!(capture.recorded_at.offset(), time::UtcOffset::UTC);
    assert_eq!(capture.recorded_at, at() + Duration::seconds(2));
    assert_ne!(capture.recorded_at, OffsetDateTime::UNIX_EPOCH);
}
