use application::identity::Principal;
use application::participants::ParticipantAction;
use application::ApplicationError;
use domain::cases::CaseId;
use domain::identity::{Role, UserId};
use postgres::Transaction;

pub(super) fn authorize(
    transaction: &mut Transaction<'_>,
    actor: UserId,
    case: CaseId,
    action: ParticipantAction,
    item: bool,
) -> Result<Principal, ApplicationError> {
    let principal = crate::postgres_actor::active_actor(transaction, actor)?;
    if !principal.role.allows(action.permission()) {
        return Err(ApplicationError::PermissionDenied);
    }
    let visible = if principal.role == Role::Owner {
        transaction.query_opt("SELECT id FROM cases WHERE id=$1", &[&case.as_uuid()])
    } else {
        transaction.query_opt(
            "SELECT case_id FROM case_memberships WHERE case_id=$1 AND user_id=$2 FOR SHARE",
            &[&case.as_uuid(), &actor.as_uuid()],
        )
    }
    .map_err(super::storage::port)?;
    visible.ok_or(if item {
        ApplicationError::ParticipantNotFound
    } else {
        ApplicationError::CaseNotFound
    })?;
    Ok(principal)
}
