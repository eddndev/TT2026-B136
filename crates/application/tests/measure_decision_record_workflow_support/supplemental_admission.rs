use super::*;
use application::documents::DocumentFormatBatch;

struct RejectedFormat;
impl DocumentFormatBatchValidator for RejectedFormat {
    fn validate_batch(
        &self,
        _: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        Err(ApplicationError::StageSupportFormatRejected)
    }
}

#[test]
fn selected_support_identity_version_and_digest_are_checked_before_crypto() {
    for mutation in 0..3 {
        let mut fixture = Fixture::corrected();
        let record = &mut fixture.material.support_record;
        match mutation {
            0 => record.id = DocumentId::new(),
            1 => record.version = DocumentVersion::new(2).unwrap(),
            _ => record.digest = Sha256Digest::from_array([55; 32]),
        }
        let harness = harness(fixture.store(), identity(fixture.actor));
        assert!(harness
            .service
            .prepare("session", fixture.case_id, fixture.command)
            .is_err());
        assert!(harness.events().is_empty());
        assert_eq!(harness.validator.calls(), 0);
    }
}

#[test]
fn aligned_metadata_cannot_bypass_plaintext_hash_or_format_rejection() {
    let mut fixture = Fixture::corrected();
    fixture.material.support_record.digest = Sha256Digest::from_array([55; 32]);
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
    let harness = harness(fixture.store(), identity(fixture.actor));
    assert!(matches!(
        harness
            .service
            .prepare("session", fixture.case_id, fixture.command),
        Err(ApplicationError::StoredDocumentInconsistent(_))
    ));
    assert_eq!(harness.events(), ["unwrap", "open", "hash"]);
    assert_eq!(harness.validator.calls(), 0);
    let fixture = Fixture::corrected();
    let (service, observations) = custom_service(
        fixture.store(),
        identity(fixture.actor),
        Arc::new(RejectedFormat),
        Arc::new(FixedClock(now())),
        Arc::new(Hasher),
    );
    assert!(matches!(
        service.prepare("session", fixture.case_id, fixture.command),
        Err(ApplicationError::StageSupportFormatRejected)
    ));
    assert!(observations.lock().unwrap().events.contains(&"open"));
}

#[test]
fn same_instruction_with_changed_valid_source_requires_new_review_confirmation() {
    let mut fixture = initial();
    let original = fixture.review();
    fixture.material.result_sources[0]
        .sources
        .subject
        .changed_by
        .email = "another-historical-actor@example.test".into();
    let changed = fixture.review();
    assert_eq!(original.submission_digest, changed.submission_digest);
    assert_ne!(original.review_digest, changed.review_digest);
    let harness = harness(fixture.store(), identity(fixture.actor));
    assert!(matches!(
        harness.service.submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&original)
        ),
        Err(ApplicationError::MeasureDecision(
            MeasureDecisionError::ReviewMismatch
        ))
    ));
}

#[test]
fn valid_but_different_committed_review_or_corrupt_group_is_rejected() {
    for mutation in 0..3 {
        let fixture = initial();
        let expected = confirmation(&fixture.review());
        let mut changed = fixture.clone();
        changed.material.result_sources[0]
            .sources
            .subject
            .changed_by
            .email = "another-historical-actor@example.test".into();
        let mut returned = if mutation == 0 {
            changed.operation(now())
        } else {
            fixture.operation(now())
        };
        match mutation {
            1 => returned.group.measures[0].capture_digest = Sha256Digest::from_array([66; 32]),
            2 => returned.origin.group_digest = Sha256Digest::from_array([77; 32]),
            _ => {}
        }
        let mut store = fixture.store();
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, _| Ok(returned));
        let harness = harness(store, identity(fixture.actor));
        assert!(harness
            .service
            .submit("session", fixture.case_id, fixture.command, expected)
            .is_err());
    }
}
