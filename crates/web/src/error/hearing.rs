use super::ApiError;
use application::{hearings::HearingError as H, ApplicationError as A};
use axum::http::StatusCode;
use domain::DomainError as D;

pub(super) fn map(error: A) -> Result<ApiError, A> {
    let (status, code) = match &error {
        A::Hearing(H::NotFound) => (StatusCode::NOT_FOUND, "hearing_not_found"),
        A::Hearing(H::RevisionConflict) => (StatusCode::CONFLICT, "hearing_revision_conflict"),
        A::Hearing(H::ContextConflict) => (StatusCode::CONFLICT, "hearing_context_conflict"),
        A::Hearing(H::ParticipantChanged) => (StatusCode::CONFLICT, "hearing_participant_changed"),
        A::Hearing(H::AlreadyCancelled) => (StatusCode::CONFLICT, "hearing_already_cancelled"),
        A::Hearing(H::OperationConflict) => (StatusCode::CONFLICT, "hearing_operation_conflict"),
        A::Hearing(H::SubmissionMismatch) => (StatusCode::CONFLICT, "hearing_submission_mismatch"),
        A::Hearing(H::SupportTooLarge) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "hearing_support_too_large",
        ),
        A::Hearing(H::SupportFormatRejected) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "hearing_support_format_rejected",
        ),
        A::Hearing(H::SupportValidationLimit) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "hearing_support_validation_limit",
        ),
        A::Hearing(H::SupportDigestMismatch) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "hearing_support_digest_mismatch",
        ),
        A::Hearing(H::SupportChanged) => (StatusCode::CONFLICT, "hearing_support_changed"),
        A::Hearing(H::RevisionExhausted) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "hearing_revision_exhausted",
        ),
        A::Hearing(H::ContextRequired) => {
            (StatusCode::UNPROCESSABLE_ENTITY, "hearing_context_required")
        }
        A::Hearing(H::StageIncompatible) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "hearing_stage_incompatible",
        ),
        A::Hearing(H::ImmutableKind) => {
            (StatusCode::UNPROCESSABLE_ENTITY, "hearing_immutable_kind")
        }
        A::Domain(D::InvalidHearingValue(_)) => {
            (StatusCode::UNPROCESSABLE_ENTITY, "invalid_hearing_value")
        }
        A::Domain(D::InvalidHearingRevision) => {
            (StatusCode::UNPROCESSABLE_ENTITY, "invalid_hearing_revision")
        }
        _ => return Err(error),
    };
    Ok(ApiError {
        status,
        code,
        message: error.to_string(),
    })
}
