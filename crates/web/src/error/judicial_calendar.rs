use super::ApiError;
use application::{judicial_calendars::JudicialCalendarError as C, ApplicationError as A};
use axum::http::StatusCode;
use domain::DomainError as D;
pub(super) fn map(error: A) -> Result<ApiError, A> {
    let (status, code) = match &error {
        A::JudicialCalendar(C::NotFound) => (StatusCode::NOT_FOUND, "judicial_calendar_not_found"),
        A::JudicialCalendar(C::RevisionConflict) => {
            (StatusCode::CONFLICT, "judicial_calendar_revision_conflict")
        }
        A::JudicialCalendar(C::OperationConflict) => {
            (StatusCode::CONFLICT, "judicial_calendar_operation_conflict")
        }
        A::JudicialCalendar(C::Retired) => (StatusCode::CONFLICT, "judicial_calendar_retired"),
        A::JudicialCalendar(C::RevisionExhausted)
        | A::Domain(D::JudicialCalendarRevisionExhausted) => {
            (StatusCode::CONFLICT, "judicial_calendar_revision_exhausted")
        }
        A::JudicialCalendar(C::ScopeChangeForbidden) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "judicial_calendar_scope_change_forbidden",
        ),
        A::JudicialCalendar(C::SubmissionMismatch) => (
            StatusCode::CONFLICT,
            "judicial_calendar_submission_mismatch",
        ),
        A::Domain(D::InvalidJudicialCalendarValue(_)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_judicial_calendar_value",
        ),
        A::Domain(D::InvalidJudicialCalendarRevision) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_judicial_calendar_revision",
        ),
        _ => return Err(error),
    };
    Ok(ApiError {
        status,
        code,
        message: error.to_string(),
    })
}
