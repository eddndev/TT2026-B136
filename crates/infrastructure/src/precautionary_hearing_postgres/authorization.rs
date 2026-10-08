use super::port;
use application::{identity::Principal, ApplicationError};
use domain::{cases::CaseId, identity::Role};
use postgres::Transaction;

pub(crate) fn authorize(
    tx: &mut Transaction<'_>,
    actor: &Principal,
    case: CaseId,
    write: bool,
) -> Result<(), ApplicationError> {
    let current = crate::postgres_actor::active_actor(tx, actor.id)?;
    if current != *actor {
        return Err(ApplicationError::InvalidSession);
    }
    let allowed = matches!(current.role, Role::Owner | Role::Litigator)
        || (!write && current.role == Role::Paralegal);
    if !allowed {
        return Err(ApplicationError::PermissionDenied);
    }
    let visible = if current.role == Role::Owner {
        tx.query_opt("SELECT id FROM cases WHERE id=$1", &[&case.as_uuid()])
    } else {
        tx.query_opt(
            "SELECT case_id FROM case_memberships WHERE case_id=$1 AND user_id=$2 FOR SHARE",
            &[&case.as_uuid(), &current.id.as_uuid()],
        )
    }
    .map_err(port)?;
    visible.ok_or(ApplicationError::CaseNotFound)?;
    Ok(())
}
