use super::{authorization, port, storage};
use application::{deadline_profiles::*, ApplicationError};
use domain::crypto::DocumentHasher;
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    collection: DeadlineProfileCollection,
    command: &DeadlineProfileCommand,
    hasher: &dyn DocumentHasher,
) -> Result<DeadlineProfilePreparation, ApplicationError> {
    command.result_revision()?;
    let exists = authorization::visible(tx, collection, command.profile_id, true)?;
    let base = if exists {
        Some(storage::detail(tx, command.profile_id, None, hasher)?)
    } else {
        None
    };
    let used: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM deadline_profile_revisions WHERE operation_id=$1)",
            &[&command.operation_id.as_uuid()],
        )
        .map_err(port)?
        .get(0);
    if used {
        return Err(DeadlineProfileError::OperationConflict.into());
    }
    match (&command.change, &base) {
        (DeadlineProfileChange::Publish { .. }, Some(_)) => {
            return Err(DeadlineProfileError::RevisionConflict.into())
        }
        (DeadlineProfileChange::Replace { .. } | DeadlineProfileChange::Retire { .. }, None) => {
            return Err(DeadlineProfileError::NotFound.into())
        }
        _ => {}
    }
    if let Some(base) = &base {
        if base.revision.get() != command.expected_revision() {
            return Err(DeadlineProfileError::RevisionConflict.into());
        }
        if base.status != DeadlineProfileStatus::Published {
            return Err(DeadlineProfileError::Retired.into());
        }
    }
    let definition = match &command.change {
        DeadlineProfileChange::Publish { definition }
        | DeadlineProfileChange::Replace { definition, .. } => definition,
        DeadlineProfileChange::Retire { .. } => {
            &base
                .as_ref()
                .ok_or(DeadlineProfileError::NotFound)?
                .definition
        }
    };
    let scoped = match (collection, definition.scope()) {
        (DeadlineProfileCollection::Global, DeadlineProfileScope::Global(_)) => true,
        (DeadlineProfileCollection::ForCase(a), DeadlineProfileScope::Case(b)) => a == *b,
        _ => false,
    };
    if !scoped
        || base
            .as_ref()
            .is_some_and(|b| b.definition.scope() != definition.scope())
    {
        return Err(DeadlineProfileError::ScopeChangeForbidden.into());
    }
    Ok(DeadlineProfilePreparation {
        collection,
        profile_id: command.profile_id,
        initial_scope: base.as_ref().map(|b| b.definition.scope().clone()),
        base,
    })
}
