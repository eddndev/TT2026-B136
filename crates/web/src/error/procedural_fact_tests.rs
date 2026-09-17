use super::ApiError;
use application::{procedural_facts::ProceduralFactError as F, ApplicationError as A};
use axum::http::StatusCode as S;
use domain::DomainError as D;

#[test]
fn fact_errors_preserve_actionable_status_codes() {
    let cases = [
        (F::NotFound, S::NOT_FOUND, "procedural_fact_not_found"),
        (
            F::ReferenceNotFound,
            S::NOT_FOUND,
            "procedural_fact_reference_not_found",
        ),
        (
            F::InvalidReference,
            S::UNPROCESSABLE_ENTITY,
            "procedural_fact_invalid_reference",
        ),
        (
            F::RevisionConflict,
            S::CONFLICT,
            "procedural_fact_revision_conflict",
        ),
        (
            F::RevisionExhausted,
            S::CONFLICT,
            "procedural_fact_revision_exhausted",
        ),
        (
            F::AlreadyWithdrawn,
            S::CONFLICT,
            "procedural_fact_already_withdrawn",
        ),
        (
            F::OperationConflict,
            S::CONFLICT,
            "procedural_fact_operation_conflict",
        ),
        (
            F::SubmissionMismatch,
            S::CONFLICT,
            "procedural_fact_submission_mismatch",
        ),
        (
            F::SupportChanged,
            S::CONFLICT,
            "procedural_fact_support_changed",
        ),
        (
            F::SupportTooLarge,
            S::UNPROCESSABLE_ENTITY,
            "procedural_fact_support_too_large",
        ),
        (
            F::SupportFormatRejected,
            S::UNPROCESSABLE_ENTITY,
            "procedural_fact_support_format_rejected",
        ),
        (
            F::SupportValidationLimit,
            S::UNPROCESSABLE_ENTITY,
            "procedural_fact_support_validation_limit",
        ),
        (
            F::SupportDigestMismatch,
            S::UNPROCESSABLE_ENTITY,
            "procedural_fact_support_digest_mismatch",
        ),
    ];
    for (error, status, code) in cases {
        let response = ApiError::from(A::from(error));
        assert_eq!(response.status, status, "{code}");
        assert_eq!(response.code, code);
    }
}
#[test]
fn fact_value_failures_are_validation_errors_without_a_future_time_policy() {
    for (error, code) in [
        (
            D::InvalidProceduralFact("invalid declaration"),
            "invalid_procedural_fact",
        ),
        (
            D::InvalidDeclaredProceduralTime,
            "invalid_declared_procedural_time",
        ),
    ] {
        let response = ApiError::from(A::from(error));
        assert_eq!(response.status, S::UNPROCESSABLE_ENTITY);
        assert_eq!(response.code, code);
    }
}
#[test]
fn stored_fact_errors_do_not_expose_internal_material() {
    let response = ApiError::from(A::from(F::StoredInconsistent(
        "private source and SQL".into(),
    )));
    assert_eq!(response.status, S::INTERNAL_SERVER_ERROR);
    assert_eq!(response.code, "internal_error");
    assert!(!response.message.contains("private"));
    assert!(!response.message.contains("SQL"));
}
