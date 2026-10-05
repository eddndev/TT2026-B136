use super::*;
use application::{cases::CaseRevision, precautionary_hearings::PrecautionaryContext};
use time::{Duration, UtcOffset};

fn advanced_first() -> RecordFixture {
    let mut fixture = RecordFixture::initial();
    let mut material = fixture.context.material().clone();
    material.administration.revision = CaseRevision::new(2).unwrap();
    material.administration.changed_at = fixture.recorded_at - Duration::seconds(1);
    fixture.context = PrecautionaryContext::new(&Hasher, material).unwrap();
    fixture.command.context = expectation(&fixture.context);
    fixture
}

#[test]
fn repeated_correction_compares_context_to_previous_administrative_record_not_only_last_judicial() {
    let first = advanced_first();
    let capture = first.capture();
    for mutation in 0..3 {
        let mut next = RecordFixture::next(&capture, &first.history, 1);
        let mut material = if mutation == 0 {
            crate::context_support::initial()
        } else {
            next.context.material().clone()
        };
        match mutation {
            1 => {
                material.administration.revision = CaseRevision::new(3).unwrap();
                material.administration.changed_at -= Duration::seconds(1);
            }
            2 => {
                material.administration.changed_by.email =
                    "different administration recorder".into()
            }
            _ => {}
        }
        next.context = PrecautionaryContext::new(&Hasher, material).unwrap();
        next.command.context = expectation(&next.context);
        assert!(next.prepare().is_err());
    }
}

#[test]
fn repeated_correction_cannot_regress_a_stage_captured_only_by_the_previous_correction() {
    use domain::{
        case_stages::{CaseStageChange, DeclaredStageTime, StageSupportRef, StageTransition},
        crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    };
    let mut first = RecordFixture::initial();
    let reference = DocumentVersionRef {
        id: DocumentId::from_uuid(Uuid::from_u128(92)),
        version: DocumentVersion::initial(),
    };
    let values = CaseStageChange::Transition(StageTransition::to_intermediate(
        DeclaredStageTime::instant(crate::context_support::at()).unwrap(),
        StageSupportRef::new(reference, Sha256Digest::from_array([12; 32])),
        None,
    ));
    first.context =
        PrecautionaryContext::new(&Hasher, crate::context_support::changed(values)).unwrap();
    first.command.context = expectation(&first.context);
    let capture = first.capture();
    let mut next = RecordFixture::next(&capture, &first.history, 1);
    next.context = PrecautionaryContext::new(&Hasher, crate::context_support::initial()).unwrap();
    next.command.context = expectation(&next.context);
    assert!(next.prepare().is_err());
}

#[test]
fn rehashed_administrative_capture_cannot_regress_time_or_use_non_utc_provenance() {
    let (fixture, original, _) = chain();
    let previous_at = fixture.history.administrative[0].capture.recorded_at;
    for at in [
        previous_at - Duration::nanoseconds(1),
        original
            .recorded_at
            .to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
    ] {
        let mut capture = original.clone();
        capture.recorded_at = at;
        capture.records[0].recorded_at = at;
        super::reconstruction::refresh(&mut capture);
        super::reconstruction::reject(&capture, &fixture);
    }
}

#[test]
fn independently_valid_mixed_branches_must_agree_on_all_shared_immutable_sources() {
    let first = RecordFixture::initial();
    let capture = first.capture();
    for context_conflict in [false, true] {
        let mut request = independent_request(9);
        if context_conflict {
            let mut material = request.material.context.material().clone();
            material.administration.changed_by.email = "other captured administrator".into();
            material.stage_administration = material.administration.clone();
            let application::case_stages::CaseStageEntry::Initial(stage) = &mut material.stage
            else {
                unreachable!()
            };
            stage.recorded_by = material.administration.changed_by.clone();
            request.material.context = PrecautionaryContext::new(&Hasher, material).unwrap();
            request.command.context = expectation(&request.material.context);
        } else {
            request.material.result_sources[0]
                .sources
                .subject
                .changed_by
                .email = "other captured subject recorder".into();
        }
        let group = request.capture();
        let mut other = RecordFixture::from_first(CorrectionFixture::from_group(
            &group,
            &crate::effect_support::empty_history(),
            group.measures[0].result.id,
        ));
        other.command.operation_id = MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(999));
        let other_capture = other.capture();
        let mut evidence = append_administrative(&first.history, &capture);
        let other_evidence = append_administrative(&other.history, &other_capture);
        evidence
            .judicial
            .groups
            .extend(other_evidence.judicial.groups);
        evidence
            .administrative
            .extend(other_evidence.administrative);
        let selections = [
            record_reference(&capture.records[0]),
            record_reference(&other_capture.records[0]),
        ];
        assert!(resolve_measure_records(&Hasher, first.case_id, &selections, &evidence).is_err());
    }
}
