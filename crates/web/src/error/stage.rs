use super::ApiError;
use application::ApplicationError;
use axum::http::StatusCode;
use domain::DomainError;

pub(super) fn map(error: ApplicationError) -> Result<ApiError, ApplicationError> {
    use ApplicationError as A;
    use DomainError as D;
    let (status, code) = match &error {
        A::CaseStageConflict => (StatusCode::CONFLICT, "case_stage_conflict"),
        A::CaseStageRevisionExhausted => (StatusCode::CONFLICT, "case_stage_revision_exhausted"),
        A::CaseStageRequired => (StatusCode::CONFLICT, "case_stage_required"),
        A::CaseStageTransitionRejected => (StatusCode::CONFLICT, "case_stage_transition_rejected"),
        A::CaseStageProfileIncomplete => (StatusCode::CONFLICT, "case_stage_profile_incomplete"),
        A::StageSupportDigestMismatch => (StatusCode::CONFLICT, "stage_support_digest_mismatch"),
        A::StageSupportChanged => (StatusCode::CONFLICT, "stage_support_changed"),
        A::Domain(value) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            match value {
                D::InvalidCaseStage => "invalid_case_stage",
                D::InvalidDeclaredStageTime => "invalid_declared_stage_time",
                D::InvalidStageNote => "invalid_stage_note",
                D::InvalidStageCourt => "invalid_stage_court",
                D::InvalidStageReceiptReference => "invalid_stage_receipt_reference",
                D::InvalidStageActOrder => "invalid_stage_act_order",
                D::ConflictingStageSupport => "conflicting_stage_support",
                D::StageActInFuture => "stage_act_in_future",
                _ => return Err(error),
            },
        ),
        _ => return Err(error),
    };
    Ok(ApiError {
        status,
        code,
        message: error.to_string(),
    })
}
