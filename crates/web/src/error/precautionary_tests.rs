use super::ApiError;
use application::{
    measure_corrections::MeasureAdministrativeError as Administrative,
    precautionary_hearings::PrecautionaryHearingError as Hearing,
    precautionary_measures::MeasureDecisionError as Decision, ApplicationError,
};
use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
use serde_json::json;

async fn assert_response(error: ApplicationError, status: StatusCode, code: &str) {
    let message = error.to_string();
    let response = ApiError::from(error).into_response();
    assert_eq!(response.status(), status, "{code}");
    assert_eq!(response.headers()["content-type"], "application/json");
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        json!({"error": {"code": code, "message": message}}),
        "{code}"
    );
}

#[tokio::test]
async fn precautionary_hearing_errors_preserve_actionable_http_codes() {
    for (error, status, code) in [
        (
            Hearing::NotFound,
            StatusCode::NOT_FOUND,
            "precautionary_hearing_not_found",
        ),
        (
            Hearing::IncompleteHistory,
            StatusCode::UNPROCESSABLE_ENTITY,
            "precautionary_hearing_incomplete_history",
        ),
        (
            Hearing::OperationConflict,
            StatusCode::CONFLICT,
            "precautionary_hearing_operation_conflict",
        ),
        (
            Hearing::SubmissionMismatch,
            StatusCode::CONFLICT,
            "precautionary_hearing_submission_mismatch",
        ),
        (
            Hearing::ReviewMismatch,
            StatusCode::CONFLICT,
            "precautionary_hearing_review_mismatch",
        ),
    ] {
        assert_response(ApplicationError::PrecautionaryHearing(error), status, code).await;
    }
}

#[tokio::test]
async fn measure_decision_errors_preserve_actionable_http_codes() {
    for (error, status, code) in [
        (
            Decision::NotFound,
            StatusCode::NOT_FOUND,
            "measure_decision_not_found",
        ),
        (
            Decision::IncompleteHistory,
            StatusCode::UNPROCESSABLE_ENTITY,
            "measure_decision_incomplete_history",
        ),
        (
            Decision::OperationConflict,
            StatusCode::CONFLICT,
            "measure_decision_operation_conflict",
        ),
        (
            Decision::SubmissionMismatch,
            StatusCode::CONFLICT,
            "measure_decision_submission_mismatch",
        ),
        (
            Decision::ReviewMismatch,
            StatusCode::CONFLICT,
            "measure_decision_review_mismatch",
        ),
    ] {
        assert_response(ApplicationError::MeasureDecision(error), status, code).await;
    }
}

#[tokio::test]
async fn measure_administrative_errors_preserve_actionable_http_codes() {
    for (error, status, code) in [
        (
            Administrative::NotFound,
            StatusCode::NOT_FOUND,
            "measure_administrative_not_found",
        ),
        (
            Administrative::IncompleteHistory,
            StatusCode::UNPROCESSABLE_ENTITY,
            "measure_administrative_incomplete_history",
        ),
        (
            Administrative::OperationConflict,
            StatusCode::CONFLICT,
            "measure_administrative_operation_conflict",
        ),
        (
            Administrative::SubmissionMismatch,
            StatusCode::CONFLICT,
            "measure_administrative_submission_mismatch",
        ),
        (
            Administrative::ReviewMismatch,
            StatusCode::CONFLICT,
            "measure_administrative_review_mismatch",
        ),
        (
            Administrative::StaleHead,
            StatusCode::CONFLICT,
            "measure_administrative_stale_head",
        ),
        (
            Administrative::KnownDependants,
            StatusCode::CONFLICT,
            "measure_administrative_known_dependants",
        ),
    ] {
        assert_response(ApplicationError::MeasureAdministrative(error), status, code).await;
    }
}

#[tokio::test]
async fn precautionary_stored_failures_keep_the_exact_opaque_http_envelope() {
    let private = "private subject, historical actor, SQL and connection credentials";
    for error in [
        ApplicationError::PrecautionaryHearing(Hearing::StoredInconsistent(private.into())),
        ApplicationError::MeasureDecision(Decision::StoredInconsistent(private.into())),
        ApplicationError::MeasureAdministrative(Administrative::StoredInconsistent(private.into())),
    ] {
        let response = ApiError::from(error).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.headers()["content-type"], "application/json");
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(
            &body[..],
            br#"{"error":{"code":"internal_error","message":"internal application error"}}"#
        );
    }
}
