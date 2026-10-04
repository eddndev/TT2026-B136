use domain::crypto::{DocumentId, Sha256Digest};
use domain::hearings::HearingSupportRef;
use domain::participants::DirectoryStatus;
use uuid::Uuid;

use crate::correction_support::*;
use crate::effect_support::{empty_history, substitution, values_input, LaterFixture};
use crate::measure_decision_fixtures::{decision_input, Fixture};

fn assert_retained(fixture: &CorrectionFixture) {
    let prior = fixture.previous();
    let group = fixture.previous_group();
    let capture = fixture.capture();
    let result = &capture.review.result;
    assert_eq!(result.previous, reference(&prior));
    assert_eq!(result.id, prior.result.id);
    assert_eq!(result.revision.get(), prior.result.revision.get() + 1);
    assert_eq!(
        result.record_root,
        MeasureRecordRoot::Judicial(prior.result.origin)
    );
    assert_eq!(result.judicial_origin, prior.result.origin);
    assert_eq!(result.last_action, prior.result.action);
    assert_eq!(result.last_judicial.owner, owned(&group).owner);
    assert_eq!(result.last_judicial.reference, reference(&prior));
    assert_eq!(result.validity, MeasureCaptureValidity::Valid);
    assert_eq!(result.sources, prior.result.sources);
    assert_eq!(result.projection, prior.result.projection);
    assert_eq!(result.values.subject(), prior.result.values.subject());
    assert_eq!(result.values.kind(), prior.result.values.kind());
    assert_eq!(result.values.validity(), prior.result.values.validity());
    assert_eq!(
        result.values.supervision(),
        prior.result.values.supervision()
    );
    assert_eq!(capture.review.support, group.decision.support);
    assert_eq!(capture.records[0].support, capture.review.support);
    measure_administrative_capture_matches(&Hasher, &capture, &fixture.history).unwrap();
}

#[test]
fn correction_after_confirm_or_modify_retains_original_origin_and_latest_judicial_terms() {
    let initial = Fixture::single().capture();
    for modify in [false, true] {
        let mut later = LaterFixture::confirm(&initial);
        if modify {
            let mut input = values_input(&initial.measures[0].result.values);
            input.conditions = note("Judicially modified conditions");
            later.effects(vec![MeasureEffect::Modify {
                previous: reference(&initial.measures[0]),
                values: MeasureValues::new(input),
            }]);
        }
        let ancestors = later.evidence.clone();
        let group = later.capture();
        let mut fixture = CorrectionFixture::from_group(&group, &ancestors, id(70));
        fixture.history.groups.reverse();
        assert_retained(&fixture);
        let result = fixture.capture().review.result;
        assert_eq!(result.judicial_origin, initial.measures[0].result.origin);
        assert_ne!(
            result.last_judicial.owner.decision_id,
            result.judicial_origin.decision_id
        );
    }
}

#[test]
fn correction_of_revoke_and_cease_preserves_terminality_without_inventing_an_end() {
    let initial = Fixture::single().capture();
    for cease in [false, true] {
        let mut later = LaterFixture::confirm(&initial);
        let previous = reference(&initial.measures[0]);
        later.effects(vec![if cease {
            MeasureEffect::Cease { previous }
        } else {
            MeasureEffect::Revoke { previous }
        }]);
        let ancestors = later.evidence.clone();
        let group = later.capture();
        let fixture = CorrectionFixture::from_group(&group, &ancestors, id(70));
        assert_retained(&fixture);
        let result = fixture.capture().review.result;
        assert!(result.values.validity().end().is_none());
        assert_eq!(
            result.last_action,
            if cease {
                MeasureCaptureAction::Cease
            } else {
                MeasureCaptureAction::Revoke
            }
        );
    }
}

#[test]
fn substitution_out_and_in_keep_their_own_origins_and_full_selected_group() {
    let initial = Fixture::multiple().capture();
    let later = substitution(&initial, &[90, 100]);
    let ancestors = later.evidence.clone();
    let group = later.capture();
    for measure_id in [id(70), id(80), id(90), id(100)] {
        let fixture = CorrectionFixture::from_group(&group, &ancestors, measure_id);
        assert_retained(&fixture);
        let capture = fixture.capture();
        assert_eq!(capture.records.len(), 1);
        assert_eq!(capture.review.result.id, measure_id);
        assert_eq!(fixture.previous_group().measures.len(), 4);
        assert_eq!(fixture.previous_group().substitutions.len(), 1);
    }
}

#[test]
fn correction_retains_last_judicial_support_instead_of_initial_imposition_support() {
    let initial = Fixture::single().capture();
    let mut later = LaterFixture::confirm(&initial);
    later.request.material.support.reference.id = DocumentId::from_uuid(Uuid::from_u128(92));
    later.request.material.support.digest = Sha256Digest::from_array([22; 32]);
    later.request.material.support.name = "confirmation.docx".into();
    later.request.material.support.format = application::case_stages::StageDocumentFormat::Docx;
    let mut values = decision_input(&later.request.command.values);
    values.support = HearingSupportRef::new(
        later.request.material.support.reference,
        later.request.material.support.digest,
    );
    later.request.command.values = MeasureDecisionValues::new(values);
    let ancestors = later.evidence.clone();
    let group = later.capture();
    let fixture = CorrectionFixture::from_group(&group, &ancestors, id(70));
    let capture = fixture.capture();
    assert_eq!(capture.review.support, group.decision.support);
    assert_ne!(capture.review.support, initial.decision.support);
    assert_retained(&fixture);
}

#[test]
fn archived_manual_typed_and_unknown_supervision_sources_remain_exact() {
    for source in [
        crate::measure_source_support::Fixture::manual(DirectoryStatus::Archived),
        crate::measure_source_support::Fixture::typed(false, DirectoryStatus::Archived),
        crate::measure_source_support::Fixture::unknown(true),
    ] {
        let mut initial = Fixture::single();
        initial.replace_values(id(70), source.values);
        initial.material.result_sources[0].sources = source.sources;
        let group = initial.capture();
        let fixture = CorrectionFixture::from_group(&group, &empty_history(), id(70));
        assert_retained(&fixture);
    }
}

#[test]
fn correcting_one_member_preserves_its_whole_owning_group_without_creating_sibling_rows() {
    let group = Fixture::multiple().capture();
    let before = group.clone();
    let fixture = CorrectionFixture::from_group(&group, &empty_history(), id(80));
    let capture = fixture.capture();
    assert_eq!(capture.records.len(), 1);
    assert_eq!(capture.review.result.id, id(80));
    assert_eq!(fixture.history.groups.len(), 1);
    assert_eq!(fixture.history.groups[0].capture, before);
    assert_retained(&fixture);
}

#[test]
fn administrative_origin_binds_the_exact_review_and_owning_receipt() {
    let fixture = CorrectionFixture::initial();
    let capture = fixture.capture();
    let origin = measure_administrative_origin(&Hasher, &capture, &fixture.history).unwrap();
    assert_eq!(origin.case_id, fixture.case_id);
    assert_eq!(origin.operation_id, fixture.command.operation_id);
    assert_eq!(origin.submission_digest, capture.review.submission_digest);
    assert_eq!(origin.review_digest, capture.review.review_digest);
    assert_eq!(origin.capture_digest, capture.capture_digest);
    assert_eq!(capture.review.command, fixture.command);
    assert_eq!(capture.review.context, fixture.context);
    assert_eq!(capture.records[0].context, fixture.context);
}

#[test]
fn corrected_validity_and_supervision_text_keep_exact_sources_and_projection() {
    use domain::procedural_time::DeclaredProceduralTime;

    let mut fixture = CorrectionFixture::initial();
    let prior = fixture.previous();
    let start = MeasureTime::new(
        DeclaredProceduralTime::date("2026-10-04".parse().unwrap(), None).unwrap(),
        None,
    )
    .unwrap();
    let validity = MeasureValidity::new(start, note("Corrected declared validity"), None).unwrap();
    let correction = MeasureCorrectionValues::new(
        note("Corrected conditions"),
        validity.clone(),
        note("Corrected supervisor statement"),
    );
    let expected_values = prior.result.values.correct_record(&correction).unwrap();
    fixture.command.action = MeasureAdministrativeAction::Correct(correction);
    let capture = fixture.capture();
    assert_eq!(capture.review.result.values, expected_values);
    assert_eq!(capture.review.result.values.validity(), &validity);
    assert_eq!(capture.review.result.sources, prior.result.sources);
    assert_eq!(capture.review.result.projection, prior.result.projection);
    assert_eq!(capture.records[0].result, capture.review.result);
    measure_administrative_capture_matches(&Hasher, &capture, &fixture.history).unwrap();
}
