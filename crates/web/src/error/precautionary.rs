use super::ApiError;
use application::{
    measure_corrections::{
        MeasureAdministrativeError as Administrative, MeasureRecordReadError as RecordRead,
    },
    precautionary_hearings::PrecautionaryHearingError as Hearing,
    precautionary_measures::MeasureDecisionError as Decision,
    ApplicationError,
};
use axum::http::StatusCode;

pub(super) fn map(error: ApplicationError) -> Result<ApiError, ApplicationError> {
    let (status, code) = match &error {
        ApplicationError::PrecautionaryHearing(error) => match error {
            Hearing::NotFound => (StatusCode::NOT_FOUND, "precautionary_hearing_not_found"),
            Hearing::IncompleteHistory => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "precautionary_hearing_incomplete_history",
            ),
            Hearing::OperationConflict => (
                StatusCode::CONFLICT,
                "precautionary_hearing_operation_conflict",
            ),
            Hearing::SubmissionMismatch => (
                StatusCode::CONFLICT,
                "precautionary_hearing_submission_mismatch",
            ),
            Hearing::ReviewMismatch => (
                StatusCode::CONFLICT,
                "precautionary_hearing_review_mismatch",
            ),
            Hearing::StoredInconsistent(_) => return Ok(ApiError::internal()),
        },
        ApplicationError::MeasureDecision(error) => match error {
            Decision::NotFound => (StatusCode::NOT_FOUND, "measure_decision_not_found"),
            Decision::IncompleteHistory => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "measure_decision_incomplete_history",
            ),
            Decision::OperationConflict => {
                (StatusCode::CONFLICT, "measure_decision_operation_conflict")
            }
            Decision::SubmissionMismatch => {
                (StatusCode::CONFLICT, "measure_decision_submission_mismatch")
            }
            Decision::ReviewMismatch => (StatusCode::CONFLICT, "measure_decision_review_mismatch"),
            Decision::StoredInconsistent(_) => return Ok(ApiError::internal()),
        },
        ApplicationError::MeasureRecordRead(error) => match error {
            RecordRead::NotFound => (StatusCode::NOT_FOUND, "measure_record_not_found"),
            RecordRead::IncompleteHistory => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "measure_record_incomplete_history",
            ),
            RecordRead::StoredInconsistent(_) => return Ok(ApiError::internal()),
        },
        ApplicationError::MeasureAdministrative(error) => match error {
            Administrative::NotFound => (StatusCode::NOT_FOUND, "measure_administrative_not_found"),
            Administrative::IncompleteHistory => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "measure_administrative_incomplete_history",
            ),
            Administrative::OperationConflict => (
                StatusCode::CONFLICT,
                "measure_administrative_operation_conflict",
            ),
            Administrative::SubmissionMismatch => (
                StatusCode::CONFLICT,
                "measure_administrative_submission_mismatch",
            ),
            Administrative::ReviewMismatch => (
                StatusCode::CONFLICT,
                "measure_administrative_review_mismatch",
            ),
            Administrative::StaleHead => {
                (StatusCode::CONFLICT, "measure_administrative_stale_head")
            }
            Administrative::KnownDependants => (
                StatusCode::CONFLICT,
                "measure_administrative_known_dependants",
            ),
            Administrative::StoredInconsistent(_) => return Ok(ApiError::internal()),
        },
        _ => return Err(error),
    };
    Ok(ApiError {
        status,
        code,
        message: error.to_string(),
    })
}
