use domain::{clock::OffsetDateTime, identity::Role};

use super::super::*;
use crate::{identity::normalize_email, ApplicationError};

pub(super) fn stored() -> ApplicationError {
    ApplicationError::Port("stored invitation material is inconsistent".into())
}

pub(super) fn operation(id: Uuid) -> Result<(), ApplicationError> {
    if id.is_nil() {
        return Err(ApplicationError::InvalidInput(
            "invitation operation is required".into(),
        ));
    }
    Ok(())
}

pub(super) fn identifier(id: InvitationId) -> Result<(), ApplicationError> {
    operation(id.as_uuid())
}

pub(super) fn revision(value: u64) -> bool {
    (1..=i64::MAX as u64).contains(&value)
}

pub(super) fn role(value: Role) -> Result<(), ApplicationError> {
    if !matches!(value, Role::Litigator | Role::Paralegal) {
        return Err(ApplicationError::InvalidInput(
            "invitation role must be litigator or paralegal".into(),
        ));
    }
    Ok(())
}

fn identity(
    id: InvitationId,
    counter: u64,
    email: &str,
    assigned: Role,
) -> Result<(), ApplicationError> {
    identifier(id).map_err(|_| stored())?;
    if !revision(counter) || normalize_email(email).map_err(|_| stored())? != email {
        return Err(stored());
    }
    role(assigned).map_err(|_| stored())
}

pub(super) fn candidate(
    value: &InvitationCandidate,
    now: OffsetDateTime,
) -> Result<(), ApplicationError> {
    identity(value.id, value.revision, &value.email, value.role)?;
    if now.unix_timestamp() < 0 || value.expires_at <= now {
        return Err(ApplicationError::InvalidCredentials);
    }
    if value.expires_at - now > time::Duration::seconds(INVITATION_TTL_SECONDS) {
        return Err(stored());
    }
    Ok(())
}

pub(super) fn summary(value: &InvitationSummary) -> Result<(), ApplicationError> {
    identity(value.id, value.revision, &value.email, value.role)?;
    operation(value.operation_id).map_err(|_| stored())?;
    if value.issued_at.unix_timestamp() < 0
        || value.expires_at - value.issued_at != time::Duration::seconds(INVITATION_TTL_SECONDS)
    {
        return Err(stored());
    }
    match (value.status, value.accepted_at) {
        (InvitationStatus::Accepted, Some(at))
            if at >= value.issued_at && at < value.expires_at =>
        {
            Ok(())
        }
        (
            InvitationStatus::Pending | InvitationStatus::PendingMfa | InvitationStatus::Expired,
            None,
        ) => Ok(()),
        _ => Err(stored()),
    }
}

pub(super) fn receipt(
    value: &InvitationReceipt,
    operation_id: Uuid,
    now: OffsetDateTime,
) -> Result<(), ApplicationError> {
    identity(value.id, value.revision, &value.email, value.role)?;
    if value.operation_id != operation_id
        || value.user_id.as_uuid().is_nil()
        || value.accepted_at.unix_timestamp() < 0
        || value.accepted_at > now
    {
        return Err(stored());
    }
    Ok(())
}

pub(super) fn ciphertext(value: &[u8]) -> Result<(), ApplicationError> {
    if value.is_empty() || value.len() > MAX_INVITATION_PAYLOAD_BYTES + 28 {
        return Err(stored());
    }
    Ok(())
}
