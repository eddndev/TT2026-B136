use crate::{
    decision_anchor_support::*, decision_support::*, history_support as hs,
    measure_source_support as sources,
};
use application::hearings::*;
use domain::{crypto::Sha256Digest, participants::DirectoryStatus};
use uuid::Uuid;

fn source_fixture(typed: bool) -> sources::Fixture {
    if typed {
        sources::Fixture::typed(false, DirectoryStatus::Archived)
    } else {
        sources::Fixture::manual(DirectoryStatus::Archived)
    }
}

fn ordinary_with_participant(source: &sources::Fixture, serial: u128) -> HearingDetail {
    let projection = source.resolve().unwrap().supervisor.unwrap();
    let mut detail = ordinary_initial();
    detail.snapshot.id = HearingId::from_uuid(Uuid::from_u128(5000 + serial));
    detail.snapshot.receipt.operation_id =
        HearingOperationId::from_uuid(Uuid::from_u128(6000 + serial));
    let values = &detail.snapshot.values;
    detail.snapshot.values = HearingValues::new(HearingValuesInput {
        kind: values.kind(),
        scheduled_at: values.scheduled_at(),
        modality: values.modality(),
        venue: values.venue().clone(),
        note: values.note().cloned(),
        participants: vec![HearingParticipantRef::new(
            projection.overview.id,
            projection.overview.revision,
        )],
        conviction_basis: values.conviction_basis().cloned(),
    })
    .unwrap();
    detail.participants = vec![HearingParticipantSnapshot {
        overview: projection.overview,
        values_digest: projection.snapshot.values_digest,
    }];
    refresh_ordinary(&mut detail);
    hearing_receipt_matches(&Hasher, &detail).unwrap();
    detail
}

fn decision_with_supervisor(source: sources::Fixture) -> Fixture {
    let mut fixture = Fixture::single();
    fixture.replace_values(id(70), source.values);
    fixture.material.result_sources[0].sources = source.sources;
    fixture
}

#[test]
fn ordinary_participant_matches_the_exact_manual_or_typed_supervisor_projection() {
    for typed in [false, true] {
        let source = source_fixture(typed);
        let detail = ordinary_with_participant(&source, 1);
        let expected = detail.participants[0].clone();
        let mut fixture = decision_with_supervisor(source);
        attach_initial(&mut fixture, detail);
        let group = capture(fixture, &empty(), at());
        let actual = group.measures[0]
            .result
            .projection
            .supervisor
            .as_ref()
            .unwrap();
        assert_eq!(expected.overview, actual.overview);
        assert_eq!(expected.values_digest, actual.snapshot.values_digest);
        measure_decision_group_with_history_matches(&Hasher, &group, &empty()).unwrap();
    }
}

#[test]
fn ordinary_participant_overview_cannot_contradict_the_same_exact_supervisor() {
    for typed in [false, true] {
        let source = source_fixture(typed);
        let mut detail = ordinary_with_participant(&source, 1);
        detail.participants[0].overview.display_name = "Different retained name".into();
        hearing_receipt_matches(&Hasher, &detail).unwrap();
        let mut fixture = decision_with_supervisor(source);
        attach_initial(&mut fixture, detail);
        assert!(prepare(fixture, &empty()).is_err());
    }
}

#[test]
fn ordinary_participant_digest_cannot_contradict_the_same_exact_supervisor() {
    for typed in [false, true] {
        let source = source_fixture(typed);
        let mut detail = ordinary_with_participant(&source, 1);
        detail.participants[0].values_digest = Sha256Digest::from_array([99; 32]);
        hearing_receipt_matches(&Hasher, &detail).unwrap();
        let mut fixture = decision_with_supervisor(source);
        attach_initial(&mut fixture, detail);
        assert!(prepare(fixture, &empty()).is_err());
    }
}

fn independently_anchored_group(
    serial: u128,
    measure: u128,
    detail: HearingDetail,
) -> MeasureDecisionGroupCapture {
    let mut fixture = hs::root_fixture(serial, measure);
    let source = sources::Fixture::unknown(false);
    fixture.replace_values(id(measure), source.values);
    fixture.material.result_sources[0].sources = source.sources;
    attach_initial(&mut fixture, detail);
    capture(fixture, &empty(), at())
}

#[test]
fn different_ordinary_anchors_may_retain_the_same_exact_participant_projection() {
    let source = source_fixture(true);
    let first = independently_anchored_group(1, 70, ordinary_with_participant(&source, 1));
    let second = independently_anchored_group(2, 80, ordinary_with_participant(&source, 2));
    let evidence = hs::history(vec![
        hs::entry(&first, &empty()),
        hs::entry(&second, &empty()),
    ]);
    resolve_measure_targets(
        &Hasher,
        first.review.case_id,
        &[
            reference(&first.measures[0]),
            reference(&second.measures[0]),
        ],
        &evidence,
    )
    .unwrap();
}

#[test]
fn different_ordinary_anchors_cannot_change_one_exact_participant_projection() {
    let source = source_fixture(true);
    let first = independently_anchored_group(1, 70, ordinary_with_participant(&source, 1));
    for change_digest in [false, true] {
        let mut detail = ordinary_with_participant(&source, 2);
        if change_digest {
            detail.participants[0].values_digest = Sha256Digest::from_array([99; 32]);
        } else {
            detail.participants[0].overview.display_name = "Different retained name".into();
        }
        hearing_receipt_matches(&Hasher, &detail).unwrap();
        let second = independently_anchored_group(2, 80, detail);
        let evidence = hs::history(vec![
            hs::entry(&first, &empty()),
            hs::entry(&second, &empty()),
        ]);
        assert!(resolve_measure_targets(
            &Hasher,
            first.review.case_id,
            &[
                reference(&first.measures[0]),
                reference(&second.measures[0]),
            ],
            &evidence,
        )
        .is_err());
    }
}

fn ordinary_with_distinct_participant_on_shared_subject() -> HearingDetail {
    let mut source = source_fixture(true);
    sources::typed_mut(source.sources.supervisor.as_mut().unwrap()).id =
        sources::reference(8, 3).id();
    source.values = MeasureValues::new(sources::input(&source.sources));
    ordinary_with_participant(&source, 3)
}

#[test]
fn different_participants_may_share_one_exact_subject_reference() {
    let detail = ordinary_with_distinct_participant_on_shared_subject();
    let source = source_fixture(true);
    let supervisor = source.sources.supervisor.as_ref().unwrap();
    assert_ne!(detail.participants[0].overview.id, supervisor.id());
    assert_eq!(
        detail.participants[0].overview.subject.unwrap(),
        sources::subject_ref(supervisor.bound_subject.as_ref().unwrap()),
    );
    let mut fixture = decision_with_supervisor(source);
    attach_initial(&mut fixture, detail);
    let group = capture(fixture, &empty(), at());
    measure_decision_group_with_history_matches(&Hasher, &group, &empty()).unwrap();
}

#[test]
fn ordinary_subject_reference_cannot_conflict_with_another_participants_full_subject() {
    let mut detail = ordinary_with_distinct_participant_on_shared_subject();
    detail.participants[0]
        .overview
        .subject
        .as_mut()
        .unwrap()
        .values_digest = Sha256Digest::from_array([99; 32]);
    hearing_receipt_matches(&Hasher, &detail).unwrap();
    let mut fixture = decision_with_supervisor(source_fixture(true));
    attach_initial(&mut fixture, detail);
    assert!(prepare(fixture, &empty()).is_err());
}
