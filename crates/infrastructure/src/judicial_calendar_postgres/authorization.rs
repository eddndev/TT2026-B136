use application::{identity::Principal, ApplicationError};
use domain::identity::{Permission, UserId};
use postgres::Transaction;

pub(super) fn actor(
    tx: &mut Transaction<'_>,
    id: UserId,
    write: bool,
) -> Result<Principal, ApplicationError> {
    let principal = crate::postgres_actor::active_actor(tx, id)?;
    let permission = if write {
        Permission::ManageJudicialCalendar
    } else {
        Permission::ReadJudicialCalendar
    };
    if !principal.role.allows(permission) {
        return Err(ApplicationError::PermissionDenied);
    }
    Ok(principal)
}
