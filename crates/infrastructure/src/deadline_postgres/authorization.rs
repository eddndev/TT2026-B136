use super::port;
use application::{
    cases::CaseAdministrativeStatus, deadlines::*, identity::Principal, ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    identity::{Permission, Role, UserId},
};
use postgres::Transaction;

pub(super) fn actor(
    tx: &mut Transaction<'_>,
    id: UserId,
    case_id: CaseId,
    write: bool,
    hasher: &dyn DocumentHasher,
) -> Result<Principal, ApplicationError> {
    let principal = crate::postgres_actor::active_actor(tx, id)?;
    let permission = if write {
        Permission::ManageDeadline
    } else {
        Permission::ReadDeadline
    };
    if !principal.role.allows(permission) {
        return Err(ApplicationError::PermissionDenied);
    }
    require_case(tx, &principal, case_id)?;
    if write
        && crate::cases::storage::detail(tx, case_id, hasher)?
            .administration
            .values()
            .status()
            == CaseAdministrativeStatus::Closed
    {
        return Err(ApplicationError::CaseClosed);
    }
    Ok(principal)
}
fn require_case(
    tx: &mut Transaction<'_>,
    principal: &Principal,
    case: CaseId,
) -> Result<(), ApplicationError> {
    let row = if principal.role == Role::Owner {
        tx.query_opt("SELECT id FROM cases WHERE id=$1", &[&case.as_uuid()])
    } else {
        tx.query_opt(
            "SELECT case_id FROM case_memberships WHERE case_id=$1 AND user_id=$2 FOR SHARE",
            &[&case.as_uuid(), &principal.id.as_uuid()],
        )
    }
    .map_err(port)?;
    row.ok_or(ApplicationError::CaseNotFound)?;
    Ok(())
}
pub(super) fn responsible(
    tx: &mut Transaction<'_>,
    id: UserId,
    case: CaseId,
) -> Result<DeadlineResponsibleSnapshot, ApplicationError> {
    let principal = crate::postgres_actor::active_actor(tx, id).map_err(|error| match error {
        ApplicationError::InvalidSession => {
            ApplicationError::Deadline(DeadlineError::ResponsibleUnavailable)
        }
        other => other,
    })?;
    if !principal.role.allows(Permission::ReadDeadline) {
        return Err(DeadlineError::ResponsibleUnavailable.into());
    }
    require_case(tx, &principal, case).map_err(|error| match error {
        ApplicationError::CaseNotFound => {
            ApplicationError::Deadline(DeadlineError::ResponsibleUnavailable)
        }
        other => other,
    })?;
    Ok(DeadlineResponsibleSnapshot {
        id,
        email: principal.email,
        role: principal.role,
    })
}
/// Inspect scope before loading private calculation evidence.
pub(super) fn visible(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: DeadlineId,
) -> Result<bool, ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT case_id FROM case_deadlines WHERE id=$1",
            &[&id.as_uuid()],
        )
        .map_err(port)?;
    match row {
        None => Ok(false),
        Some(row)
            if row
                .try_get::<_, uuid::Uuid>(0)
                .map_err(super::inconsistent)?
                == case.as_uuid() =>
        {
            Ok(true)
        }
        Some(_) => Err(DeadlineError::NotFound.into()),
    }
}
