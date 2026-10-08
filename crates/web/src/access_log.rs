//! Allowlisted access metadata; never format requests or application errors.

use application::identity::{MfaAttempt, MfaReason};
use axum::{
    extract::Request,
    http::{HeaderValue, Method},
    response::Response,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct AccessAttempt {
    request_id: Uuid,
    method: &'static str,
    reported: Arc<AtomicBool>,
    dispatch: tracing::Dispatch,
}

impl AccessAttempt {
    pub(crate) fn begin(request: &mut Request) -> Option<Self> {
        if request.method() != Method::POST {
            return None;
        }
        let method = match request.uri().path() {
            "/api/v1/auth/mfa/totp" => "totp",
            "/api/v1/auth/mfa/recovery" => "recovery",
            _ => return None,
        };
        let attempt = Self {
            request_id: Uuid::new_v4(),
            method,
            reported: Arc::new(AtomicBool::new(false)),
            dispatch: tracing::dispatcher::get_default(Clone::clone),
        };
        request.extensions_mut().insert(attempt.clone());
        Some(attempt)
    }

    pub(crate) fn record(&self, attempt: &MfaAttempt) {
        let result = match attempt.reason {
            MfaReason::Accepted => "accepted",
            MfaReason::OperationalError => "error",
            _ => "rejected",
        };
        let user = attempt.user_id.map(|id| id.to_string());
        self.emit(
            user.as_deref().unwrap_or("unavailable"),
            result,
            attempt.reason.as_str(),
        );
    }

    pub(crate) fn finish(&self, response: &mut Response) {
        let status = response.status().as_u16();
        let (result, reason) = match status {
            503 => ("error", "request_capacity_exceeded"),
            500..=599 => ("error", "operational_error"),
            400..=499 => ("rejected", "invalid_request"),
            _ => ("error", "outcome_unavailable"),
        };
        self.emit("unavailable", result, reason);
        response.headers_mut().insert(
            "x-request-id",
            HeaderValue::from_str(&self.request_id.to_string()).expect("UUID is a valid header"),
        );
    }

    fn emit(&self, user_id: &str, result: &str, reason: &str) {
        if self.reported.swap(true, Ordering::AcqRel) {
            return;
        }
        let timestamp_unix_ms =
            (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64;
        tracing::dispatcher::with_default(&self.dispatch, || {
            tracing::info!(target: "qadra::access", timestamp_unix_ms,
                request_id = %self.request_id, method = self.method,
                user_id, result, reason, "MFA access attempt");
        });
    }
}
