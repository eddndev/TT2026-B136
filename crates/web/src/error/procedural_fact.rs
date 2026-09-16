use super::ApiError;
use application::{procedural_facts::ProceduralFactError as H, ApplicationError as A};
use axum::http::StatusCode;
use domain::DomainError as D;
pub(super) fn map(error: A) -> Result<ApiError, A> {
    let (status, code) = match &error {
        A::ProceduralFact(H::NotFound) => (StatusCode::NOT_FOUND, "procedural_fact_not_found"),
        A::ProceduralFact(H::ReferenceNotFound) => {
            (StatusCode::NOT_FOUND, "procedural_fact_reference_not_found")
        }
        A::ProceduralFact(H::RevisionConflict) => {
            (StatusCode::CONFLICT, "procedural_fact_revision_conflict")
        }
        A::ProceduralFact(H::AlreadyWithdrawn) => {
            (StatusCode::CONFLICT, "procedural_fact_already_withdrawn")
        }
        A::ProceduralFact(H::OperationConflict) => {
            (StatusCode::CONFLICT, "procedural_fact_operation_conflict")
        }
        A::ProceduralFact(H::SubmissionMismatch) => {
            (StatusCode::CONFLICT, "procedural_fact_submission_mismatch")
        }
        A::ProceduralFact(H::SupportChanged) => {
            (StatusCode::CONFLICT, "procedural_fact_support_changed")
        }
        A::ProceduralFact(H::RevisionExhausted) => {
            (StatusCode::CONFLICT, "procedural_fact_revision_exhausted")
        }
        A::ProceduralFact(H::InvalidReference) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "procedural_fact_invalid_reference",
        ),
        A::ProceduralFact(H::SupportTooLarge) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "procedural_fact_support_too_large",
        ),
        A::ProceduralFact(H::SupportFormatRejected) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "procedural_fact_support_format_rejected",
        ),
        A::ProceduralFact(H::SupportValidationLimit) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "procedural_fact_support_validation_limit",
        ),
        A::ProceduralFact(H::SupportDigestMismatch) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "procedural_fact_support_digest_mismatch",
        ),
        A::Domain(D::InvalidProceduralFact(_)) => {
            (StatusCode::UNPROCESSABLE_ENTITY, "invalid_procedural_fact")
        }
        A::Domain(D::InvalidDeclaredProceduralTime) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_declared_procedural_time",
        ),
        _ => return Err(error),
    };
    Ok(ApiError {
        status,
        code,
        message: error.to_string(),
    })
}
