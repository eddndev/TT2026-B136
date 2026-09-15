use super::ApiError;
use application::ApplicationError;
use axum::http::StatusCode;
use domain::DomainError;

#[test]
fn invalid_declarations_are_distinct_from_changed_stage_state() {
    for (error, code) in [
        (
            DomainError::InvalidDeclaredStageTime,
            "invalid_declared_stage_time",
        ),
        (DomainError::InvalidStageActOrder, "invalid_stage_act_order"),
        (DomainError::StageActInFuture, "stage_act_in_future"),
        (
            DomainError::ConflictingStageSupport,
            "conflicting_stage_support",
        ),
    ] {
        let mapped = ApiError::from(ApplicationError::from(error));
        assert_eq!(mapped.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(mapped.code, code);
    }
    for (error, code) in [
        (ApplicationError::CaseStageConflict, "case_stage_conflict"),
        (ApplicationError::CaseStageRequired, "case_stage_required"),
        (
            ApplicationError::CaseStageTransitionRejected,
            "case_stage_transition_rejected",
        ),
        (
            ApplicationError::StageSupportChanged,
            "stage_support_changed",
        ),
        (
            ApplicationError::StageSupportDigestMismatch,
            "stage_support_digest_mismatch",
        ),
    ] {
        let mapped = ApiError::from(error);
        assert_eq!(mapped.status, StatusCode::CONFLICT);
        assert_eq!(mapped.code, code);
    }
}

#[test]
fn support_admission_distinguishes_size_format_and_resource_exhaustion() {
    for (error, code) in [
        (
            ApplicationError::StageSupportTooLarge,
            "stage_support_too_large",
        ),
        (
            ApplicationError::StageSupportFormatRejected,
            "stage_support_format_rejected",
        ),
        (
            ApplicationError::StageSupportValidationLimit,
            "stage_support_validation_limit",
        ),
    ] {
        let mapped = ApiError::from(error);
        assert_eq!(mapped.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(mapped.code, code);
    }
}

#[test]
fn corrupted_stage_storage_remains_an_opaque_server_error() {
    let error = ApplicationError::StoredCaseStageInconsistent("private evidence".into());
    let mapped = ApiError::from(error);
    assert_eq!(mapped.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(mapped.code, "internal_error");
    assert!(!mapped.message.contains("private evidence"));
}
