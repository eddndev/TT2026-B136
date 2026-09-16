use super::port;
use application::{identity::Principal, ApplicationError};
use domain::{
    cases::CaseId,
    identity::{Permission, Role, UserId},
};
use postgres::Transaction;

pub(super) fn actor(
    tx: &mut Transaction<'_>,
    id: UserId,
    write: bool,
) -> Result<Principal, ApplicationError> {
    let principal = crate::postgres_actor::active_actor(tx, id)?;
    let permission = if write {
        Permission::ManageHearingResult
    } else {
        Permission::ReadHearingResult
    };
    if !principal.role.allows(permission) {
        return Err(ApplicationError::PermissionDenied);
    }
    Ok(principal)
}
pub(super) fn scope(
    tx: &mut Transaction<'_>,
    principal: &Principal,
    case: CaseId,
) -> Result<(), ApplicationError> {
    let visible = if principal.role == Role::Owner {
        tx.query_opt("SELECT id FROM cases WHERE id=$1", &[&case.as_uuid()])
    } else {
        tx.query_opt(
            "SELECT case_id FROM case_memberships WHERE case_id=$1 AND user_id=$2 FOR SHARE",
            &[&case.as_uuid(), &principal.id.as_uuid()],
        )
    }
    .map_err(port)?;
    visible.ok_or(ApplicationError::CaseNotFound)?;
    Ok(())
}
pub(super) fn authorize(
    tx: &mut Transaction<'_>,
    id: UserId,
    case: CaseId,
    write: bool,
) -> Result<Principal, ApplicationError> {
    let principal = actor(tx, id, write)?;
    scope(tx, &principal, case)?;
    Ok(principal)
}
