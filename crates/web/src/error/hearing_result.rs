use super::ApiError;
use application::{hearing_results::HearingResultError as H, ApplicationError as A};
use axum::http::StatusCode;
use domain::DomainError as D;
pub(super) fn map(error: A) -> Result<ApiError, A> {
    let (status, code) = match &error {
        A::HearingResult(H::NotFound) => (StatusCode::NOT_FOUND, "hearing_result_not_found"),
        A::HearingResult(H::ReferenceNotFound) => {
            (StatusCode::NOT_FOUND, "hearing_result_reference_not_found")
        }
        A::HearingResult(H::RevisionConflict) => {
            (StatusCode::CONFLICT, "hearing_result_revision_conflict")
        }
        A::HearingResult(H::AlreadyWithdrawn) => {
            (StatusCode::CONFLICT, "hearing_result_already_withdrawn")
        }
        A::HearingResult(H::OperationConflict) => {
            (StatusCode::CONFLICT, "hearing_result_operation_conflict")
        }
        A::HearingResult(H::SubmissionMismatch) => {
            (StatusCode::CONFLICT, "hearing_result_submission_mismatch")
        }
        A::HearingResult(H::SupportChanged) => {
            (StatusCode::CONFLICT, "hearing_result_support_changed")
        }
        A::HearingResult(H::RevisionExhausted) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "hearing_result_revision_exhausted",
        ),
        A::HearingResult(H::FutureTime) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "hearing_result_future_time",
        ),
        A::HearingResult(H::InvalidReference) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "hearing_result_invalid_reference",
        ),
        A::HearingResult(H::SupportTooLarge) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "hearing_result_support_too_large",
        ),
        A::HearingResult(H::SupportFormatRejected) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "hearing_result_support_format_rejected",
        ),
        A::HearingResult(H::SupportValidationLimit) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "hearing_result_support_validation_limit",
        ),
        A::HearingResult(H::SupportDigestMismatch) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "hearing_result_support_digest_mismatch",
        ),
        A::Domain(D::InvalidHearingResultValue(_)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_hearing_result_value",
        ),
        A::Domain(D::InvalidHearingResultRevision) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_hearing_result_revision",
        ),
        _ => return Err(error),
    };
    Ok(ApiError {
        status,
        code,
        message: error.to_string(),
    })
}
