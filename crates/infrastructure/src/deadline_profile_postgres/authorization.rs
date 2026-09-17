use super::port;
use application::{deadline_profiles::*, identity::Principal, ApplicationError};
use domain::{
    crypto::DocumentHasher,
    identity::{Permission, Role, UserId},
};
use postgres::Transaction;

pub(super) fn actor(
    tx: &mut Transaction<'_>,
    id: UserId,
    collection: DeadlineProfileCollection,
    write: bool,
    hasher: &dyn DocumentHasher,
) -> Result<Principal, ApplicationError> {
    let principal = crate::postgres_actor::active_actor(tx, id)?;
    let permission = if write {
        Permission::ManageDeadlineProfile
    } else {
        Permission::ReadDeadlineProfile
    };
    if !principal.role.allows(permission) {
        return Err(ApplicationError::PermissionDenied);
    }
    if let DeadlineProfileCollection::ForCase(case) = collection {
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
        if write
            && crate::cases::storage::detail(tx, case, hasher)?
                .administration
                .values()
                .status()
                == application::cases::CaseAdministrativeStatus::Closed
        {
            return Err(ApplicationError::CaseClosed);
        }
    }
    Ok(principal)
}

/// Check the lightweight root before loading any private definition.
pub(super) fn visible(
    tx: &mut Transaction<'_>,
    collection: DeadlineProfileCollection,
    id: DeadlineProfileId,
    write: bool,
) -> Result<bool, ApplicationError> {
    let Some(row) = tx
        .query_opt(
            "SELECT case_id FROM deadline_profiles WHERE id=$1",
            &[&id.as_uuid()],
        )
        .map_err(port)?
    else {
        return Ok(false);
    };
    let case: Option<uuid::Uuid> = row.try_get(0).map_err(super::inconsistent)?;
    let visible = match collection {
        DeadlineProfileCollection::Global => case.is_none(),
        DeadlineProfileCollection::ForCase(id) => case == Some(id.as_uuid()) || case.is_none(),
    };
    if !visible {
        return Err(DeadlineProfileError::NotFound.into());
    }
    if write && matches!(collection, DeadlineProfileCollection::ForCase(_)) && case.is_none() {
        return Err(DeadlineProfileError::ScopeChangeForbidden.into());
    }
    Ok(true)
}
