use super::*;
use crate::{identity::Principal, ApplicationError};
use domain::{clock::OffsetDateTime, identity::Role};

pub(super) fn inconsistent(message: &str) -> ApplicationError {
    CaseReportError::StoredInconsistent(message.into()).into()
}
pub(super) fn scope(actor: &Principal) -> Result<CaseReportScope, ApplicationError> {
    match actor.role {
        Role::Owner => Ok(CaseReportScope::Office),
        Role::Litigator => Ok(CaseReportScope::AssignedCases),
        _ => Err(ApplicationError::PermissionDenied),
    }
}
pub(super) fn email(value: &str) -> Result<(), ApplicationError> {
    if value.len() > 254 {
        return Err(inconsistent("report account email exceeds its limit"));
    }
    let normalized = crate::identity::normalize_email(value)
        .map_err(|_| inconsistent("report account email is invalid"))?;
    if normalized != value {
        return Err(inconsistent("report account email is not canonical"));
    }

    Ok(())
}
pub(super) fn principal(
    actor: &Principal,
    expected: CaseReportScope,
) -> Result<(), ApplicationError> {
    if scope(actor).ok() != Some(expected) || actor.id.as_uuid().is_nil() {
        return Err(inconsistent("report requester and scope differ"));
    }
    email(&actor.email)
}
pub(super) fn requester(
    value: &CaseReportRequester,
    expected: CaseReportScope,
) -> Result<(), ApplicationError> {
    principal(&value.principal, expected)?;
    if value.account_revision > i64::MAX as u64 || value.auth_generation > i64::MAX as u64 {
        return Err(inconsistent(
            "report account authorization stamp is invalid",
        ));
    }
    Ok(())
}
pub(super) fn time(value: OffsetDateTime) -> Result<(), ApplicationError> {
    if value.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&value.year()) {
        return Err(inconsistent("report timestamp must be representable UTC"));
    }
    Ok(())
}
pub(super) fn window(
    started: OffsetDateTime,
    returned: OffsetDateTime,
) -> Result<(), ApplicationError> {
    time(started)?;
    time(returned)?;
    if returned < started {
        return Err(inconsistent("report clock moved backwards"));
    }
    Ok(())
}
pub(super) fn filters(value: &CaseReportFilters) -> Result<(), ApplicationError> {
    if time(value.period_from).is_err()
        || time(value.period_before).is_err()
        || value.period_from >= value.period_before
        || value.period_before - value.period_from > time::Duration::days(366)
        || value.litigator.is_some_and(|id| id.as_uuid().is_nil())
    {
        return Err(ApplicationError::InvalidInput(
            "report period must be UTC, nonempty and at most 366 days".into(),
        ));
    }
    Ok(())
}
pub(super) fn stored_filters(value: &CaseReportFilters) -> Result<(), ApplicationError> {
    filters(value).map_err(|_| inconsistent("stored report filters are invalid"))
}
pub(super) fn query(value: CaseReportQuery) -> Result<(), ApplicationError> {
    if !(1..=100).contains(&value.limit) || value.after_id.is_some_and(|id| id.as_uuid().is_nil()) {
        return Err(ApplicationError::InvalidInput(
            "report page limit must be between 1 and 100 with a valid cursor".into(),
        ));
    }
    Ok(())
}
pub(super) fn lease(value: &CaseReportLease, at: OffsetDateTime) -> Result<(), ApplicationError> {
    time(at)?;
    time(value.expires_at)?;
    if value.report_id.as_uuid().is_nil()
        || value.attempt_id.as_uuid().is_nil()
        || value.token.as_uuid().is_nil()
        || value.generation == 0
        || value.generation > i64::MAX as u64
    {
        return Err(inconsistent("report lease identity is invalid"));
    }
    if at >= value.expires_at {
        return Err(CaseReportError::LeaseLost.into());
    }
    Ok(())
}
