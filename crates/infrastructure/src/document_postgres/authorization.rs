use application::documents::DocumentAction;
use application::identity::Principal;
use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::DocumentId;
use domain::identity::{Role, UserId};
use postgres::Transaction;

use super::storage;
use crate::postgres_actor::active_actor;

pub(super) fn authorize(
    transaction: &mut Transaction<'_>,
    actor: UserId,
    case: CaseId,
    action: DocumentAction,
) -> Result<Principal, ApplicationError> {
    let principal = active_actor(transaction, actor)?;
    if !principal.role.allows(action.permission()) {
        return Err(ApplicationError::PermissionDenied);
    }
    let visible = if principal.role == Role::Owner {
        transaction.query_opt("SELECT id FROM cases WHERE id=$1", &[&case.as_uuid()])
    } else {
        transaction.query_opt(
            "SELECT m.case_id FROM case_memberships m WHERE m.case_id=$1 AND m.user_id=$2 FOR SHARE",
            &[&case.as_uuid(), &actor.as_uuid()],
        )
    }.map_err(storage::port_error)?;
    visible.ok_or(ApplicationError::CaseNotFound)?;
    Ok(principal)
}

pub(super) fn authorize_document(
    transaction: &mut Transaction<'_>,
    actor: UserId,
    case: CaseId,
    id: DocumentId,
    action: DocumentAction,
) -> Result<Principal, ApplicationError> {
    authorize(transaction, actor, case, action).map_err(|error| match error {
        ApplicationError::CaseNotFound => ApplicationError::DocumentNotFound(id.to_string()),
        other => other,
    })
}
