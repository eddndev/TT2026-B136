use super::port;
use application::{identity::Principal, ApplicationError};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use postgres::Transaction;

pub(crate) fn authorize(
    tx: &mut Transaction<'_>,
    actor: UserId,
    case: CaseId,
    write: bool,
) -> Result<Principal, ApplicationError> {
    let principal = crate::postgres_actor::active_actor(tx, actor)?;
    if principal.role == Role::Client || (write && principal.role == Role::Paralegal) {
        return Err(ApplicationError::PermissionDenied);
    }
    let found = if principal.role == Role::Owner {
        tx.query_opt("SELECT id FROM cases WHERE id=$1", &[&case.as_uuid()])
    } else {
        tx.query_opt(
            "SELECT case_id FROM case_memberships WHERE case_id=$1 AND user_id=$2 FOR SHARE",
            &[&case.as_uuid(), &actor.as_uuid()],
        )
    }
    .map_err(port)?;
    found.ok_or(ApplicationError::CaseNotFound)?;
    Ok(principal)
}
