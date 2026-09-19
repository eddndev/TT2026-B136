use super::port;
use application::{alerts::AlertSubject, identity::Principal, ApplicationError};
use domain::identity::{Permission, Role, UserId};
use postgres::Transaction;

pub(super) fn actor(tx: &mut Transaction<'_>, user: UserId) -> Result<Principal, ApplicationError> {
    let actor = crate::postgres_actor::active_actor(tx, user)?;
    if !actor.role.allows(Permission::ReadHearing) || !actor.role.allows(Permission::ReadDeadline) {
        return Err(ApplicationError::PermissionDenied);
    }
    Ok(actor)
}
pub(super) fn recipient(
    tx: &mut Transaction<'_>,
    subject: AlertSubject,
    user: UserId,
    responsible: Option<UserId>,
) -> Result<Option<Principal>, ApplicationError> {
    let actor = match actor(tx, user) {
        Ok(actor) => actor,
        Err(ApplicationError::InvalidSession | ApplicationError::PermissionDenied) => {
            return Ok(None)
        }
        Err(error) => return Err(error),
    };
    if matches!(subject, AlertSubject::Deadline { .. }) && responsible != Some(user) {
        return Ok(None);
    }
    if matches!(subject, AlertSubject::Deadline { .. }) && actor.role == Role::Owner {
        return Ok(Some(actor));
    }
    let member = tx
        .query_opt(
            "SELECT case_id FROM case_memberships WHERE case_id=$1 AND user_id=$2 FOR SHARE",
            &[&subject.case_id().as_uuid(), &user.as_uuid()],
        )
        .map_err(port)?;
    Ok(member.map(|_| actor))
}
