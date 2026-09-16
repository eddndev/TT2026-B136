use super::*;
use domain::identity::{Permission, Role};
pub(super) fn authorize(
    tx: &mut Transaction<'_>,
    id: UserId,
    case: CaseId,
    manage: bool,
) -> Result<Principal, ApplicationError> {
    let principal = crate::postgres_actor::active_actor(tx, id)?;
    let permission = if manage {
        Permission::ManageParticipant
    } else {
        Permission::ReadParticipant
    };
    if !principal.role.allows(permission) {
        return Err(ApplicationError::PermissionDenied);
    }
    let visible = if principal.role == Role::Owner {
        tx.query_opt("SELECT id FROM cases WHERE id=$1", &[&case.as_uuid()])
    } else {
        tx.query_opt(
            "SELECT case_id FROM case_memberships WHERE case_id=$1 AND user_id=$2 FOR SHARE",
            &[&case.as_uuid(), &id.as_uuid()],
        )
    }
    .map_err(port)?;
    visible.ok_or(ApplicationError::CaseNotFound)?;
    if manage {
        crate::postgres_case_status::require_active(tx, case)?;
    }
    Ok(principal)
}
