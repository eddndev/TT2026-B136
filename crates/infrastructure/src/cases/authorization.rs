use application::identity::Principal;
use application::ApplicationError;
use domain::cases::CaseId;
use domain::identity::{Permission, Role, UserId};
use postgres::Transaction;

use super::storage::port;

pub(super) fn actor(
    tx: &mut Transaction<'_>,
    id: UserId,
    permission: Option<Permission>,
) -> Result<Principal, ApplicationError> {
    let principal = crate::postgres_actor::active_actor(tx, id)?;
    if permission.is_some_and(|p| !principal.role.allows(p)) {
        return Err(ApplicationError::PermissionDenied);
    }
    Ok(principal)
}
pub(super) fn require_scope(
    tx: &mut Transaction<'_>,
    principal: &Principal,
    id: CaseId,
) -> Result<(), ApplicationError> {
    let visible = if principal.role == Role::Owner {
        tx.query_opt("SELECT id FROM cases WHERE id=$1", &[&id.as_uuid()])
    } else {
        tx.query_opt(
            "SELECT case_id FROM case_memberships WHERE case_id=$1 AND user_id=$2 FOR SHARE",
            &[&id.as_uuid(), &principal.id.as_uuid()],
        )
    }
    .map_err(port)?;
    visible.ok_or(ApplicationError::CaseNotFound)?;
    Ok(())
}
