use super::ApiError;
use application::{deadlines::DeadlineError as D, ApplicationError as A};
use axum::http::StatusCode as S;

#[test]
fn deadline_errors_preserve_actionable_status_codes() {
    for (error, status, code) in [
        (D::NotFound, S::NOT_FOUND, "deadline_not_found"),
        (
            D::RevisionConflict,
            S::CONFLICT,
            "deadline_revision_conflict",
        ),
        (
            D::OperationConflict,
            S::CONFLICT,
            "deadline_operation_conflict",
        ),
        (
            D::RevisionExhausted,
            S::CONFLICT,
            "deadline_revision_exhausted",
        ),
        (D::Retired, S::CONFLICT, "deadline_retired"),
        (
            D::ProfileUnavailable,
            S::CONFLICT,
            "deadline_profile_unavailable",
        ),
        (
            D::ResponsibleUnavailable,
            S::CONFLICT,
            "deadline_responsible_unavailable",
        ),
        (
            D::SubmissionMismatch,
            S::CONFLICT,
            "deadline_submission_mismatch",
        ),
        (
            D::Invalid("revision"),
            S::UNPROCESSABLE_ENTITY,
            "invalid_deadline",
        ),
    ] {
        let response = ApiError::from(A::Deadline(error));
        assert_eq!(response.status, status);
        assert_eq!(response.code, code);
    }
}

#[test]
fn stored_deadline_diagnostics_do_not_leave_the_server() {
    let response = ApiError::from(A::Deadline(D::StoredInconsistent(
        "private source details".into(),
    )));
    assert_eq!(response.status, S::INTERNAL_SERVER_ERROR);
    assert_eq!(response.code, "internal_error");
    assert_eq!(response.message, "internal application error");
}
