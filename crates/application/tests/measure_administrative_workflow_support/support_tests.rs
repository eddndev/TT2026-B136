use super::*;
use domain::crypto::Sha256Digest;
use std::sync::Arc;

#[test]
fn retained_support_identity_and_digest_must_match_before_plaintext_admission() {
    for mismatch in 0..3 {
        let mut fixture = Fixture::single();
        match mismatch {
            0 => fixture.material.support_record.id = DocumentId::from_uuid(Uuid::from_u128(9191)),
            1 => fixture.material.support_record.version = DocumentVersion::new(2).unwrap(),
            _ => fixture.material.support_record.digest = Sha256Digest::from_array([99; 32]),
        }
        let harness = harness(fixture.store(), identity(fixture.actor));
        assert!(harness
            .service
            .prepare("session", fixture.case_id, fixture.command)
            .is_err());
        assert_eq!(harness.validator.calls(), 0);
        assert!(harness.events().is_empty());
    }
}

#[test]
fn coherent_historical_digest_claim_still_requires_matching_decrypted_plaintext() {
    let mut record = Fixture::single().material.support_record;
    record.digest = Sha256Digest::from_array([99; 32]);
    let fixture = Fixture::with_support_record(record);
    let harness = harness(fixture.store(), identity(fixture.actor));
    let error = harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .unwrap_err();
    assert!(matches!(
        error,
        ApplicationError::StoredDocumentInconsistent(_)
    ));
    assert!(harness.events().contains(&"open"));
    assert!(harness.events().contains(&"hash"));
    assert_eq!(harness.validator.calls(), 0);
}

#[test]
fn format_admission_failure_rejects_correction_and_mark_without_commit() {
    for fixture in [Fixture::single(), Fixture::mark()] {
        let mut validator = Validator::default();
        validator.failure = true;
        let harness = harness_with(
            fixture.store(),
            identity(fixture.actor),
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
}
