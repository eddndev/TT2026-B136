use application::identity::invitations::{
    InvitationDeliveryState, InvitationEnrollment, InvitationPage, InvitationReceipt,
    InvitationStatus, InvitationSummary,
};
use domain::clock::OffsetDateTime;
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, UtcOffset};

use super::error::InvitationError;

pub(super) fn summary(row: InvitationSummary) -> Result<Value, InvitationError> {
    let status = match row.status {
        InvitationStatus::Pending => "pending",
        InvitationStatus::PendingMfa => "pending_mfa",
        InvitationStatus::Accepted => "accepted",
        InvitationStatus::Expired => "expired",
    };
    let delivery = match row.delivery {
        InvitationDeliveryState::Pending => "pending",
        InvitationDeliveryState::Sent => "sent",
        InvitationDeliveryState::Uncertain => "uncertain",
        InvitationDeliveryState::Rejected => "rejected",
    };
    Ok(
        json!({"id":row.id.to_string(),"revision":revision(row.revision)?,
        "operation_id":row.operation_id,"email":row.email,"role":row.role.as_str(),
        "status":status,"delivery":delivery,"issued_at":timestamp(row.issued_at)?,
        "expires_at":timestamp(row.expires_at)?,"accepted_at":row.accepted_at.map(timestamp).transpose()?}),
    )
}
pub(super) fn page(row: InvitationPage) -> Result<Value, InvitationError> {
    let items = row
        .items
        .into_iter()
        .map(summary)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({"items":items,"next":row.next.map(|id| id.to_string())}))
}
pub(super) fn enrollment(row: InvitationEnrollment) -> Result<Value, InvitationError> {
    Ok(
        json!({"id":row.id.to_string(),"revision":revision(row.revision)?,
        "operation_id":row.operation_id,"email":row.email,"role":row.role.as_str(),
        "expires_at":timestamp(row.expires_at)?,"totp_secret_base32":row.totp_secret_base32.as_str(),
        "otpauth_uri":row.otpauth_uri.as_str(),
        "recovery_codes":row.recovery_codes.iter().map(|value| value.as_str()).collect::<Vec<_>>()}),
    )
}
pub(super) fn receipt(row: InvitationReceipt) -> Result<Value, InvitationError> {
    Ok(
        json!({"id":row.id.to_string(),"revision":revision(row.revision)?,
        "operation_id":row.operation_id,"user_id":row.user_id.to_string(),"email":row.email,
        "role":row.role.as_str(),"accepted_at":timestamp(row.accepted_at)?}),
    )
}
fn timestamp(value: OffsetDateTime) -> Result<String, InvitationError> {
    value
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|_| InvitationError::Uncertain)
}
fn revision(value: u64) -> Result<String, InvitationError> {
    if !(1..=i64::MAX as u64).contains(&value) {
        return Err(InvitationError::Uncertain);
    }
    Ok(value.to_string())
}
