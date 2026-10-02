use super::ApiError;
use application::{case_reports::CaseReportError as R, ApplicationError as A};
use axum::http::StatusCode;
pub(super) fn map(error: A) -> Result<ApiError, A> {
    let (status, code) = match &error {
        A::CaseReport(R::NotFound) => (StatusCode::NOT_FOUND, "case_report_not_found"),
        A::CaseReport(R::OperationConflict) => {
            (StatusCode::CONFLICT, "case_report_operation_conflict")
        }
        A::CaseReport(R::NotReady) => (StatusCode::CONFLICT, "case_report_not_ready"),
        A::CaseReport(R::CapacityExceeded) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "case_report_capacity_exceeded",
        ),
        A::CaseReport(R::AccessRevoked) => (StatusCode::FORBIDDEN, "case_report_access_revoked"),
        A::CaseReport(R::RenderUnavailable) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "case_report_render_unavailable",
        ),
        A::CaseReport(R::RenderFailed) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "case_report_render_failed",
        ),
        A::CaseReport(R::StoredInconsistent(_) | R::LeaseLost) => return Ok(ApiError::internal()),
        _ => return Err(error),
    };
    Ok(ApiError {
        status,
        code,
        message: error.to_string(),
    })
}
