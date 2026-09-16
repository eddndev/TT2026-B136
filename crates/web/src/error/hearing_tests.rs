use super::ApiError;
use application::{hearings::HearingError, ApplicationError};
use axum::http::StatusCode;
use domain::DomainError;

#[test]
fn hearing_conflicts_are_actionable_without_exposing_storage() {
    for (value, code) in [
        (HearingError::RevisionConflict, "hearing_revision_conflict"),
        (HearingError::ContextConflict, "hearing_context_conflict"),
        (
            HearingError::ParticipantChanged,
            "hearing_participant_changed",
        ),
        (HearingError::AlreadyCancelled, "hearing_already_cancelled"),
        (
            HearingError::OperationConflict,
            "hearing_operation_conflict",
        ),
        (
            HearingError::SubmissionMismatch,
            "hearing_submission_mismatch",
        ),
        (HearingError::SupportChanged, "hearing_support_changed"),
    ] {
        let error = ApiError::from(ApplicationError::from(value));
        assert_eq!(error.status, StatusCode::CONFLICT);
        assert_eq!(error.code, code);
    }
}
#[test]
fn invalid_hearing_values_and_context_have_public_validation_codes() {
    for (value, code) in [
        (
            HearingError::RevisionExhausted,
            "hearing_revision_exhausted",
        ),
        (HearingError::ContextRequired, "hearing_context_required"),
        (
            HearingError::StageIncompatible,
            "hearing_stage_incompatible",
        ),
        (HearingError::ImmutableKind, "hearing_immutable_kind"),
    ] {
        let error = ApiError::from(ApplicationError::from(value));
        assert_eq!(error.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error.code, code);
    }
    for value in [
        DomainError::InvalidHearingValue("scheduled_at"),
        DomainError::InvalidHearingRevision,
    ] {
        assert_eq!(
            ApiError::from(ApplicationError::from(value)).status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
}
#[test]
fn missing_hearing_and_corrupt_storage_do_not_leak_private_details() {
    let missing = ApiError::from(ApplicationError::from(HearingError::NotFound));
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert_eq!(missing.code, "hearing_not_found");
    let corrupt = ApiError::from(ApplicationError::from(HearingError::StoredInconsistent(
        "secret participant and DSN".into(),
    )));
    assert_eq!(corrupt.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(corrupt.code, "internal_error");
    assert!(!corrupt.message.contains("secret"));
}

#[test]
fn hearing_support_errors_preserve_admission_reasons() {
    for (value, code) in [
        (HearingError::SupportTooLarge, "hearing_support_too_large"),
        (
            HearingError::SupportFormatRejected,
            "hearing_support_format_rejected",
        ),
        (
            HearingError::SupportValidationLimit,
            "hearing_support_validation_limit",
        ),
        (
            HearingError::SupportDigestMismatch,
            "hearing_support_digest_mismatch",
        ),
    ] {
        let error = ApiError::from(ApplicationError::from(value));
        assert_eq!(error.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error.code, code);
    }
}
