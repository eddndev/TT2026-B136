use super::*;
use domain::crypto::Sha256Digest;
use domain::precautionary_hearings::MeasureId;
use std::sync::Arc;

#[path = "preparation_results.rs"]
mod results;

fn reject_preparation(fixture: Fixture) {
    let harness = harness(fixture.store(), identity(fixture.actor.clone()));
    assert!(harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
}

#[test]
fn exact_decision_support_selection_is_checked_before_any_cryptography() {
    for base in [Fixture::single(), Fixture::no_change()] {
        for mutation in 0..3 {
            let mut fixture = base.clone();
            let record = &mut fixture.material.support_record;
            match mutation {
                0 => record.id = DocumentId::from_uuid(Uuid::from_u128(999)),
                1 => record.version = DocumentVersion::new(2).unwrap(),
                _ => record.digest = Sha256Digest::from_array([99; 32]),
            }
            let harness = harness(fixture.store(), identity(fixture.actor.clone()));
            assert!(harness
                .service
                .prepare("session", fixture.case_id, fixture.command)
                .is_err());
            assert!(harness.events().is_empty());
            assert_eq!(harness.validator.calls(), 0);
        }
    }
}

#[test]
fn aligned_decision_metadata_does_not_bypass_plaintext_digest_verification() {
    let mut fixture = Fixture::single();
    fixture.material.support_record.digest = Sha256Digest::from_array([99; 32]);
    let record = &fixture.material.support_record;
    let mut input = crate::measure_decision_fixtures::decision_input(&fixture.command.values);
    input.support = HearingSupportRef::new(
        DocumentVersionRef {
            id: record.id,
            version: record.version,
        },
        record.digest,
    );
    fixture.command.values = MeasureDecisionValues::new(input);
    let harness = harness(fixture.store(), identity(fixture.actor.clone()));
    assert!(matches!(
        harness
            .service
            .prepare("session", fixture.case_id, fixture.command),
        Err(ApplicationError::StoredDocumentInconsistent(_))
    ));
    assert_eq!(harness.events(), ["unwrap", "open", "hash"]);
    assert_eq!(harness.validator.calls(), 0);
}

#[test]
fn decision_format_failure_propagates_without_retry_or_commit() {
    let fixture = Fixture::single();
    let mut validator = Validator::default();
    validator.failure = true;
    let harness = harness_with(
        fixture.store(),
        identity(fixture.actor.clone()),
        validator,
        Arc::new(FixedClock(now())),
    );
    assert!(matches!(
        harness
            .service
            .prepare("session", fixture.case_id, fixture.command),
        Err(ApplicationError::StageSupportFormatRejected)
    ));
    assert_eq!(harness.validator.calls(), 1);
}

#[test]
fn ready_decision_scope_and_context_must_match_the_command() {
    let mut foreign = Fixture::single();
    foreign.case_id = CaseId::from_uuid(Uuid::from_u128(999));
    reject_preparation(foreign);
    let mut stale = Fixture::single();
    stale.command.context.context_digest = Sha256Digest::from_array([99; 32]);
    reject_preparation(stale);
}

#[test]
fn ready_result_sources_require_exact_complete_unique_rows() {
    for mutation in 0..5 {
        let mut fixture = Fixture::single();
        match mutation {
            0 => fixture.material.result_sources.clear(),
            1 => fixture
                .material
                .result_sources
                .push(fixture.material.result_sources[0].clone()),
            2 => fixture.material.result_sources[0].id = MeasureId::from_uuid(Uuid::from_u128(999)),
            3 => {
                fixture.material.result_sources[0].sources.subject.case_id =
                    CaseId::from_uuid(Uuid::from_u128(999))
            }
            _ => {
                fixture.material.result_sources[0]
                    .sources
                    .subject
                    .values_digest = Sha256Digest::from_array([99; 32])
            }
        }
        reject_preparation(fixture);
    }
    let mut no_change = Fixture::no_change();
    no_change.material.result_sources = Fixture::single().material.result_sources;
    reject_preparation(no_change);
}

#[test]
fn ready_predecessors_require_the_exact_complete_owned_captures() {
    let previous = Fixture::single().operation(at());
    for mutation in 0..4 {
        let mut fixture = Fixture::confirm(&previous);
        match mutation {
            0 => fixture.material.predecessors.clear(),
            1 => fixture
                .material
                .predecessors
                .push(fixture.material.predecessors[0].clone()),
            2 => {
                fixture.material.predecessors[0].owner.group_digest =
                    Sha256Digest::from_array([99; 32])
            }
            _ => {
                fixture.material.predecessors[0].capture.capture_digest =
                    Sha256Digest::from_array([99; 32])
            }
        }
        reject_preparation(fixture);
    }
    let mut initial = Fixture::single();
    initial.material.predecessors = Fixture::confirm(&previous).material.predecessors;
    reject_preparation(initial);
}

#[test]
fn ready_requires_complete_origin_bound_ancestor_groups() {
    let previous = Fixture::single().operation(at());
    for mutation in 0..5 {
        let mut fixture = Fixture::confirm(&previous);
        let groups = &mut fixture.material.measure_history.groups;
        match mutation {
            0 => groups.clear(),
            1 => groups[0].origin.review_digest = Sha256Digest::from_array([99; 32]),
            2 => groups[0].capture.measures.clear(),
            3 => groups.push(groups[0].clone()),
            _ => {
                groups[0].capture.measures[0]
                    .result
                    .projection
                    .subject
                    .display_name = "Invented retained subject".into()
            }
        }
        reject_preparation(fixture);
    }
    let mut unrelated = Fixture::no_change();
    unrelated.command.operation_id = MeasureDecisionOperationId::from_uuid(Uuid::from_u128(999));
    unrelated.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(998));
    unrelated.material.measure_history = Fixture::confirm(&previous).material.measure_history;
    reject_preparation(unrelated);
}

#[test]
fn ready_anchor_requires_matching_family_material_and_its_measure_dependencies() {
    let mut missing = Fixture::initial_anchor();
    missing.material.anchor = None;
    reject_preparation(missing);
    let mut unselected = Fixture::initial_anchor();
    unselected.command.anchor = None;
    reject_preparation(unselected);
    let mut wrong_family = Fixture::precautionary_anchor();
    wrong_family.material.anchor = Fixture::initial_anchor().material.anchor;
    reject_preparation(wrong_family);
    let mut corrupted = Fixture::precautionary_anchor();
    let Some(MeasureDecisionAnchorMaterial::Precautionary(capture)) =
        &mut corrupted.material.anchor
    else {
        unreachable!()
    };
    capture.capture_digest = Sha256Digest::from_array([99; 32]);
    let Some(MeasureDecisionAnchorRef::Precautionary { capture_digest, .. }) =
        &mut corrupted.command.anchor
    else {
        unreachable!()
    };
    *capture_digest = capture.capture_digest;
    reject_preparation(corrupted);
    let previous = Fixture::single().operation(at());
    let mut missing_closure = Fixture::review_anchor_no_change(&previous);
    missing_closure.material.measure_history.groups.clear();
    reject_preparation(missing_closure);
}

#[test]
fn retained_measure_sources_cannot_change_under_the_same_historical_identity() {
    let previous = Fixture::single().operation(at());
    let mut fixture = Fixture::confirm(&previous);
    fixture.material.result_sources[0]
        .sources
        .subject
        .changed_by
        .email = "different retained subject recorder".into();
    reject_preparation(fixture);
}

#[test]
fn full_review_confirmation_detects_source_provenance_changes_under_one_command() {
    for subject in [false, true] {
        let mut fixture = Fixture::single();
        let original = fixture.review();
        let sources = &mut fixture.material.result_sources[0].sources;
        if subject {
            sources.subject.changed_by.email = "different subject recorder".into();
        } else {
            sources
                .supervisor
                .as_mut()
                .unwrap()
                .bound_subject
                .as_mut()
                .unwrap()
                .changed_by
                .email = "different supervisor subject recorder".into();
        }
        let changed = fixture.review();
        assert_eq!(original.command, changed.command);
        assert_eq!(original.submission_digest, changed.submission_digest);
        assert_ne!(original.review_digest, changed.review_digest);
        let harness = harness(fixture.store(), identity(fixture.actor.clone()));
        assert!(harness
            .service
            .submit(
                "session",
                fixture.case_id,
                fixture.command,
                confirmation(&original),
            )
            .is_err());
    }
}

#[test]
fn oversized_ready_collections_are_rejected_before_document_admission() {
    let previous = Fixture::single().operation(at());
    for mutation in 0..3 {
        let mut fixture = Fixture::confirm(&previous);
        match mutation {
            0 => {
                fixture.material.result_sources =
                    vec![fixture.material.result_sources[0].clone(); 33]
            }
            1 => fixture.material.predecessors = vec![fixture.material.predecessors[0].clone(); 33],
            _ => {
                fixture.material.measure_history.groups =
                    vec![fixture.material.measure_history.groups[0].clone(); 256]
            }
        }
        let harness = harness(fixture.store(), identity(fixture.actor.clone()));
        assert!(harness
            .service
            .prepare("session", fixture.case_id, fixture.command)
            .is_err());
        assert!(harness.events().is_empty());
        assert_eq!(harness.validator.calls(), 0);
    }
}
