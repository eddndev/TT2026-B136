use super::*;
use domain::crypto::Sha256Digest;
use std::sync::Arc;

#[test]
fn exact_document_reference_and_digest_are_checked_before_decryption() {
    let prior = Fixture::schedule().operation(at());
    for original in [Fixture::schedule(), Fixture::replace(&prior)] {
        for mutation in 0..3 {
            let mut fixture = original.clone();
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
            let h = harness(fixture.store(), identity(fixture.actor));
            assert!(h
                .service
                .prepare("session", fixture.case_id, fixture.command)
                .is_err());
            assert!(h.events().is_empty());
            assert_eq!(h.validator.calls(), 0);
        }
    }
}

#[test]
fn matching_declared_digest_does_not_replace_plaintext_integrity_or_format_admission() {
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
    let h = harness(fixture.store(), identity(fixture.actor));
    assert!(matches!(
        h.service
            .prepare("session", fixture.case_id, fixture.command),
        Err(ApplicationError::StoredDocumentInconsistent(_))
    ));
    assert_eq!(h.events(), ["unwrap", "open", "hash"]);
    assert_eq!(h.validator.calls(), 0);

    let fixture = Fixture::schedule();
    let mut validator = Validator::default();
    validator.failure = true;
    let h = harness_with(
        fixture.store(),
        identity(fixture.actor),
        validator,
        Arc::new(FixedClock(now())),
    );
    assert!(matches!(
        h.service
            .prepare("session", fixture.case_id, fixture.command),
        Err(ApplicationError::StageSupportFormatRejected)
    ));
    assert_eq!(h.validator.calls(), 1);
}

#[test]
fn cancellation_retains_historical_sources_without_decrypting_or_readmitting_them() {
    let prior = Fixture::schedule().operation(at());
    let fixture = Fixture::cancel(&prior);
    let review = fixture.review();
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(|_, _, prepared| prepared.into_operation(now()));
    let h = harness(store, identity(fixture.actor));
    let result = h
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&review),
        )
        .unwrap();
    assert_eq!(result.capture.review.sources, prior.capture.review.sources);
    assert_eq!(h.validator.calls(), 0);
    assert!(h.events().is_empty());
}

#[test]
fn confirmation_binds_source_provenance_without_changing_the_instruction_digest() {
    let mut fixture = Fixture::schedule();
    let original = fixture.review();
    let selected = fixture.material.selected_sources.as_mut().unwrap();
    crate::participant_support::manual_mut(&mut selected.participants[0])
        .changed_by
        .email = "another-recorder@example.test".into();
    let changed = fixture.review();
    assert_eq!(original.submission_digest, changed.submission_digest);
    assert_ne!(original.review_digest, changed.review_digest);
    let h = harness(fixture.store(), identity(fixture.actor));
    assert!(h
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&original)
        )
        .is_err());
}
