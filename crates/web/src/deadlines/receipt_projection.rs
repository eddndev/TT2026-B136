use crate::error::ApiError;
use application::{
    deadline_reevaluation::{
        DependencyFamily, SourceEventReference, TechnicalCause, TechnicalService,
    },
    deadlines::{DeadlineActorSnapshot, DeadlineReceiptVersion},
};
use domain::cases::CaseId;
use serde_json::{json, Value};

pub(super) fn author(value: &DeadlineActorSnapshot) -> Result<Value, ApiError> {
    match value {
        DeadlineActorSnapshot::User { id, email } => {
            if email.is_empty()
                || email.trim() != email
                || email.chars().count() > 320
                || email.chars().any(char::is_control)
            {
                return Err(ApiError::internal());
            }
            Ok(json!({"kind": "user", "id": id.to_string(), "email": email}))
        }
        DeadlineActorSnapshot::Technical {
            service,
            policy_version,
        } => {
            if *policy_version != 1 {
                return Err(ApiError::internal());
            }
            let service = match service {
                TechnicalService::DeadlineReevaluator => "deadline_reevaluator",
            };
            Ok(json!({
                "kind": "technical", "service": service,
                "policy_version": policy_version
            }))
        }
    }
}

/// Project metadata after the caller verifies its enclosing receipt and evidence.
pub(super) fn version(value: &DeadlineReceiptVersion, case_id: CaseId) -> Result<Value, ApiError> {
    match value {
        DeadlineReceiptVersion::Legacy => Ok(json!({"kind": "v1"})),
        DeadlineReceiptVersion::Tracked(value) => {
            let predecessor = value.predecessor.map(|previous| {
                json!({
                    "submission_digest": previous.submission_digest.to_hex(),
                    "capture_digest": previous.capture_digest.to_hex()
                })
            });
            let cause = value
                .cause
                .map(|cause| technical_cause(cause, case_id))
                .transpose()?;
            Ok(json!({
                "kind": "v2", "observations_digest": value.observations_digest.to_hex(),
                "predecessor": predecessor, "cause": cause
            }))
        }
    }
}

fn technical_cause(value: TechnicalCause, case_id: CaseId) -> Result<Value, ApiError> {
    match value {
        TechnicalCause::LegacyBootstrap {
            job_id,
            policy_version,
        } => {
            if policy_version != 1 {
                return Err(ApiError::internal());
            }
            Ok(json!({
                "kind": "legacy_bootstrap", "job_id": job_id.to_string(),
                "policy_version": policy_version
            }))
        }
        TechnicalCause::SourceEvent { job_id, event } => Ok(json!({
            "kind": "source_event", "job_id": job_id.to_string(),
            "event": source_event(event, case_id)?
        })),
    }
}

fn source_event(value: SourceEventReference, case_id: CaseId) -> Result<Value, ApiError> {
    let valid_scope = match value.family {
        DependencyFamily::Profile => {
            value.case_id.is_none_or(|id| id == case_id) && value.hearing_id.is_none()
        }
        DependencyFamily::Calendar => value.case_id.is_none() && value.hearing_id.is_none(),
        DependencyFamily::Resolution | DependencyFamily::Notification => {
            value.case_id == Some(case_id) && value.hearing_id.is_none()
        }
        DependencyFamily::HearingResult => {
            value.case_id == Some(case_id) && value.hearing_id.is_some()
        }
    };
    if value.sequence == 0
        || value.sequence > i64::MAX as u64
        || value.revision == 0
        || !valid_scope
    {
        return Err(ApiError::internal());
    }
    Ok(json!({
        "sequence": value.sequence.to_string(), "family": family(value.family),
        "source_id": value.source_id.to_string(), "revision": value.revision,
        "case_id": value.case_id.map(|id| id.to_string()),
        "hearing_id": value.hearing_id.map(|id| id.to_string()),
        "operation_id": value.operation_id.to_string()
    }))
}

fn family(value: DependencyFamily) -> &'static str {
    match value {
        DependencyFamily::Resolution => "resolution",
        DependencyFamily::Notification => "notification",
        DependencyFamily::HearingResult => "hearing_result",
        DependencyFamily::Calendar => "calendar",
        DependencyFamily::Profile => "profile",
    }
}
