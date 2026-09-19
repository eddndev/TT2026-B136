use crate::error::ApiError;
use application::alerts::*;
use domain::identity::UserId;
use serde_json::{json, Value};
use time::OffsetDateTime;

pub(super) fn preferences(value: AlertPreferences) -> Result<Value, ApiError> {
    value
        .validate(value.user_id)
        .map_err(|_| ApiError::internal())?;
    Ok(json!({"preferences":{
        "user_id":value.user_id.to_string(), "revision":value.revision,
        "values":{
            "hearing_upcoming":family(&value.values.hearing_upcoming),
            "deadline_upcoming":family(&value.values.deadline_upcoming),
            "overdue_unattended":channels(value.values.overdue_unattended),
            "review_required":channels(value.values.review_required),
            "due_changed_soon":channels(value.values.due_changed_soon)
        },
        "updated_at":value.updated_at.map(instant),
        "receipt":value.receipt.map(|receipt|json!({"operation_id":receipt.operation_id.to_string(),"expected_revision":receipt.expected_revision})),
        "email_transport":match value.email_transport { AlertEmailTransport::Ready=>"ready", AlertEmailTransport::Disabled=>"disabled" }
    }}))
}
fn channels(value: AlertChannels) -> Value {
    json!({"internal":value.internal,"email":value.email})
}
fn family(value: &AlertFamilyPreferences) -> Value {
    json!({"lead_hours":value.anticipations.hours().iter().map(|lead|lead.get()).collect::<Vec<_>>(),"channels":channels(value.channels)})
}
pub(super) fn page(value: AlertPage, query: AlertQuery) -> Result<Value, ApiError> {
    let recipient = value
        .alerts
        .first()
        .map(|alert| alert.recipient_id)
        .unwrap_or_else(|| UserId::from_uuid(uuid::Uuid::nil()));
    value
        .validate(recipient, &query)
        .map_err(|_| ApiError::internal())?;
    let alerts = value
        .alerts
        .into_iter()
        .map(|alert| record(alert, value.checked_at))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(
        json!({"checked_at":instant(value.checked_at),"alerts":alerts,"has_more":value.has_more,"next_cursor":value.next_cursor.map(|cursor|cursor.encode())}),
    )
}
pub(super) fn detail(value: AlertDetail) -> Result<Value, ApiError> {
    Ok(
        json!({"checked_at":instant(value.checked_at),"alert":record(value.alert,value.checked_at)?}),
    )
}
pub(super) fn receipt(value: AlertReadReceipt) -> Result<Value, ApiError> {
    Ok(
        json!({"operation_id":value.operation_id.to_string(),"checked_at":instant(value.checked_at),"alert":record(value.alert,value.checked_at)?}),
    )
}
fn record(value: AlertRecord, checked_at: OffsetDateTime) -> Result<Value, ApiError> {
    value
        .validate(value.recipient_id, checked_at)
        .map_err(|_| ApiError::internal())?;
    let subject = match value.subject {
        AlertSubject::Hearing { case_id, id } => {
            json!({"kind":"hearing","case_id":case_id.to_string(),"id":id.to_string()})
        }
        AlertSubject::Deadline { case_id, id } => {
            json!({"kind":"deadline","case_id":case_id.to_string(),"id":id.to_string()})
        }
    };
    let kind = match value.kind {
        AlertKind::Upcoming {
            lead_hours,
            activity_at,
        } => {
            json!({"kind":"upcoming","lead_hours":lead_hours.get(),"activity_at":instant(activity_at)})
        }
        AlertKind::OverdueUnattended { due_at } => {
            json!({"kind":"overdue_unattended","due_at":instant(due_at)})
        }
        AlertKind::ReviewRequired => json!({"kind":"review_required"}),
        AlertKind::DueChangedSoon {
            previous_due_at,
            current_due_at,
        } => {
            json!({"kind":"due_changed_soon","previous_due_at":instant(previous_due_at),"current_due_at":instant(current_due_at)})
        }
    };
    let state = match value.state {
        AlertState::Active => json!({"kind":"active"}),
        AlertState::Resolved { at, reason } => {
            json!({"kind":"resolved","at":instant(at),"reason":match reason {
                AlertResolutionReason::Superseded=>"superseded", AlertResolutionReason::AttentionRecorded=>"attention_recorded",
                AlertResolutionReason::TargetRetired=>"target_retired", AlertResolutionReason::CancelledHearing=>"cancelled_hearing",
                AlertResolutionReason::NoLongerEligible=>"no_longer_eligible"
            }})
        }
    };
    let email = match value.email {
        AlertEmailStatus::Accepted { accepted_at } => {
            json!({"kind":"accepted","accepted_at":instant(accepted_at)})
        }
        AlertEmailStatus::Disabled => json!({"kind":"disabled"}),
        AlertEmailStatus::Pending => json!({"kind":"pending"}),
        AlertEmailStatus::Sending => json!({"kind":"sending"}),
        AlertEmailStatus::Failed => json!({"kind":"failed"}),
        AlertEmailStatus::Unknown => json!({"kind":"unknown"}),
        AlertEmailStatus::Cancelled => json!({"kind":"cancelled"}),
    };
    Ok(
        json!({"id":value.id.to_string(),"recipient_id":value.recipient_id.to_string(),"occurrence_id":value.occurrence_id.to_string(),
            "subject":subject,"subject_title":value.subject_title,"case_title":value.case_title,"case_reference":value.case_reference,
            "kind":kind,"origin":{"revision":value.origin.revision,"evidence_digest":value.origin.evidence_digest.to_hex()},
            "trigger_at":instant(value.trigger_at),"created_at":instant(value.created_at),"read_at":value.read_at.map(instant),"state":state,"email":email
        }),
    )
}
fn instant(value: OffsetDateTime) -> Value {
    json!({"unix_seconds":value.unix_timestamp(),"nanosecond":value.nanosecond(),"offset_seconds":0})
}
