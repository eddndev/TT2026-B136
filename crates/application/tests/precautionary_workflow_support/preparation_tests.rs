use super::*;
use domain::crypto::Sha256Digest;
use std::sync::Arc;

fn reject_preparation(fixture: Fixture) {
    let harness = harness(fixture.store(), identity(fixture.actor.clone()));
    assert!(harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
}

#[test]
fn selected_encrypted_record_must_match_the_exact_scheduling_reference_before_crypto() {
    let initial = Fixture::schedule().operation(at());
    for (mut fixture, mutation) in [Fixture::schedule(), Fixture::replace(&initial)]
        .into_iter()
        .flat_map(|fixture| (0..3).map(move |mutation| (fixture.clone(), mutation)))
    {
        let record = &mut fixture
            .material
            .selected_sources
            .as_mut()
            .unwrap()
            .support_record;
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

#[test]
fn matching_selected_metadata_cannot_bypass_plaintext_integrity() {
    let mut fixture = Fixture::schedule();
    let record = &mut fixture
        .material
        .selected_sources
        .as_mut()
        .unwrap()
        .support_record;
    record.digest = Sha256Digest::from_array([99; 32]);
    let support = HearingSupportRef::new(
        DocumentVersionRef {
            id: record.id,
            version: record.version,
        },
        record.digest,
    );
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut fixture.command.change else {
        unreachable!()
    };
    let mut input = values_input(values);
    input.scheduling_basis = PrecautionaryHearingSchedulingBasis::new(
        input.scheduling_basis.statement().clone(),
        support,
        input.scheduling_basis.locator().clone(),
    );
    *values = PrecautionaryHearingValues::new(input).unwrap();
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
fn scheduling_format_rejection_is_returned_without_retry_or_commit() {
    let fixture = Fixture::schedule();
    let mut validator = Validator::default();
    validator.failure = true;
    let harness = harness_with(
        fixture.store(),
        identity(fixture.actor.clone()),
        validator,
        Arc::new(FixedClock(at())),
    );
    assert!(matches!(
        harness
            .service
            .prepare("session", fixture.case_id, fixture.command),
        Err(ApplicationError::StageSupportFormatRejected)
    ));
    assert_eq!(harness.validator.calls(), 1);
    assert_eq!(harness.events(), ["unwrap", "open", "hash"]);
}

#[test]
fn ready_schedule_requires_selected_sources_and_matching_case_scope() {
    let mut missing = Fixture::schedule();
    missing.material.selected_sources = None;
    reject_preparation(missing);
    let mut foreign = Fixture::schedule();
    foreign.case_id = CaseId::from_uuid(Uuid::from_u128(999));
    reject_preparation(foreign);
}

#[test]
fn confirmation_binds_source_provenance_even_when_submission_digest_is_unchanged() {
    let mut fixture = Fixture::schedule();
    let original = fixture.review();
    crate::participant_support::manual_mut(
        &mut fixture
            .material
            .selected_sources
            .as_mut()
            .unwrap()
            .participants[0],
    )
    .changed_by
    .email = "another historical recorder".into();
    let changed = fixture.review();
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

#[test]
fn replacement_readmits_the_exact_encrypted_document_once() {
    let initial = Fixture::schedule().operation(at());
    let fixture = Fixture::replace(&initial);
    let expected = fixture.review();
    let harness = harness(fixture.store(), identity(fixture.actor.clone()));
    assert_eq!(
        harness
            .service
            .prepare("session", fixture.case_id, fixture.command)
            .unwrap(),
        expected,
    );
    assert_eq!(harness.events(), ["unwrap", "open", "hash"]);
    assert_eq!(harness.validator.calls(), 1);
}

#[test]
fn cancellation_and_replay_do_not_decrypt_or_parse_historical_support() {
    let initial_fixture = Fixture::schedule();
    let initial = initial_fixture.operation(at());
    let cancellation = Fixture::cancel(&initial);
    let harness = harness(cancellation.store(), identity(cancellation.actor.clone()));
    harness
        .service
        .prepare("session", cancellation.case_id, cancellation.command)
        .unwrap();
    assert!(harness.events().is_empty());
    assert_eq!(harness.validator.calls(), 0);
    for submit in [false, true] {
        let harness = super::harness(
            initial_fixture.replay_store(initial.clone()),
            identity(initial_fixture.actor.clone()),
        );
        if submit {
            harness
                .service
                .submit(
                    "session",
                    initial_fixture.case_id,
                    initial_fixture.command.clone(),
                    confirmation(&initial.capture.review),
                )
                .unwrap();
        } else {
            harness
                .service
                .prepare(
                    "session",
                    initial_fixture.case_id,
                    initial_fixture.command.clone(),
                )
                .unwrap();
        }
        assert!(harness.events().is_empty());
        assert_eq!(harness.validator.calls(), 0);
    }
}

#[test]
fn ready_history_and_selected_sources_presence_must_match_the_action() {
    let initial = Fixture::schedule().operation(at());
    let mut schedule = Fixture::schedule();
    schedule.material.history = Some(initial.history.clone());
    reject_preparation(schedule);
    for mut fixture in [Fixture::replace(&initial), Fixture::cancel(&initial)] {
        fixture.material.history = None;
        reject_preparation(fixture);
    }
    let mut replacement = Fixture::replace(&initial);
    replacement.material.selected_sources = None;
    reject_preparation(replacement);
    let mut cancellation = Fixture::cancel(&initial);
    cancellation.material.selected_sources = Fixture::schedule().material.selected_sources;
    reject_preparation(cancellation);
}

#[test]
fn ready_requires_a_valid_complete_origin_bound_predecessor_prefix() {
    let initial = Fixture::schedule().operation(at());
    let replaced = Fixture::replace(&initial).operation(at() + Duration::seconds(2));
    for mutation in 0..5 {
        let mut fixture = Fixture::cancel(&replaced);
        let history = fixture.material.history.as_mut().unwrap();
        match mutation {
            0 => history.captures.clear(),
            1 => history.origin.capture_digest = Sha256Digest::from_array([99; 32]),
            2 => history.captures.reverse(),
            3 => {
                history.captures.remove(0);
            }
            _ => history.captures[0].capture_digest = Sha256Digest::from_array([99; 32]),
        };
        reject_preparation(fixture);
    }
}

#[test]
fn ready_predecessor_must_match_the_exact_command_and_remain_scheduled() {
    let initial = Fixture::schedule().operation(at());
    for mutation in 0..3 {
        let mut fixture = Fixture::replace(&initial);
        let PrecautionaryHearingChange::Replace {
            expected_revision,
            expected_capture_digest,
            ..
        } = &mut fixture.command.change
        else {
            unreachable!()
        };
        match mutation {
            0 => *expected_revision = PrecautionaryHearingRevision::new(2).unwrap(),
            1 => *expected_capture_digest = Sha256Digest::from_array([99; 32]),
            _ => {
                fixture.command.hearing_id =
                    PrecautionaryHearingId::from_uuid(Uuid::from_u128(999));
            }
        }
        reject_preparation(fixture);
    }
    let cancelled = Fixture::cancel(&initial).operation(at() + Duration::seconds(2));
    reject_preparation(Fixture::replace(&cancelled));
}

#[test]
fn review_preparation_requires_the_real_selected_measure_history() {
    let mut fixture = Fixture::schedule();
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut fixture.command.change else {
        unreachable!()
    };
    let mut input = values_input(values);
    input.purpose = PrecautionaryHearingPurpose::Review;
    input.review_targets = vec![PrecautionaryMeasureRef::new(
        MeasureId::from_uuid(Uuid::from_u128(999)),
        MeasureRevision::initial(),
        Sha256Digest::from_array([99; 32]),
    )];
    *values = PrecautionaryHearingValues::new(input).unwrap();
    reject_preparation(fixture);
}

#[test]
fn cancellation_confirmation_binds_the_observed_context_without_a_new_submission() {
    let initial = Fixture::schedule().operation(at());
    let mut fixture = Fixture::cancel(&initial);
    let original = fixture.review();
    let mut context = fixture.material.observed_context.material().clone();
    context.administration.revision = context.administration.revision.next().unwrap();
    context.administration.changed_at = at() + Duration::seconds(1);
    fixture.material.observed_context = PrecautionaryContext::new(&Hasher, context).unwrap();
    let changed = fixture.review();
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
    assert!(harness.events().is_empty());
    assert_eq!(harness.validator.calls(), 0);
}

#[test]
fn committed_operation_must_preserve_its_exact_capture_and_history_proof() {
    for mutation in 0..3 {
        let fixture = Fixture::schedule();
        let review = fixture.review();
        let mut returned = fixture.operation(at());
        match mutation {
            0 => returned.capture.capture_digest = Sha256Digest::from_array([99; 32]),
            1 => returned.history.captures.clear(),
            _ => returned.history.origin.review_digest = Sha256Digest::from_array([99; 32]),
        }
        let mut store = fixture.store();
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, _| Ok(returned));
        let harness = harness(store, identity(fixture.actor));
        assert!(harness
            .service
            .submit(
                "session",
                fixture.case_id,
                fixture.command,
                confirmation(&review),
            )
            .is_err());
    }
}

#[test]
fn a_valid_commit_result_cannot_silently_replace_the_confirmed_review() {
    let fixture = Fixture::schedule();
    let review = fixture.review();
    let mut substituted = fixture.clone();
    crate::participant_support::manual_mut(
        &mut substituted
            .material
            .selected_sources
            .as_mut()
            .unwrap()
            .participants[0],
    )
    .changed_by
    .email = "different retained recorder".into();
    let returned = substituted.operation(at());
    assert_eq!(
        returned.capture.review.submission_digest,
        review.submission_digest
    );
    assert_ne!(returned.capture.review.review_digest, review.review_digest);
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, _| Ok(returned));
    let harness = harness(store, identity(fixture.actor));
    assert!(harness
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&review),
        )
        .is_err());
}
