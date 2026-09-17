use super::{receipt::inconsistent, *};
use crate::ApplicationError;
use domain::crypto::DocumentHasher;

pub(super) fn validate_preparation(
    hasher: &dyn DocumentHasher,
    collection: DeadlineProfileCollection,
    command: &DeadlineProfileCommand,
    preparation: &DeadlineProfilePreparation,
) -> Result<(DeadlineProfileDefinition, DeadlineProfileAlgorithm), ApplicationError> {
    if preparation.collection != collection || preparation.profile_id != command.profile_id {
        return Err(inconsistent("prepared collection or root differs"));
    }
    match (&preparation.base, &preparation.initial_scope) {
        (None, None) => {}
        (Some(base), Some(scope)) => {
            deadline_profile_receipt_matches(hasher, base)?;
            if base.id != command.profile_id || base.definition.scope() != scope {
                return Err(inconsistent("prepared base or initial scope differs"));
            }
            if !collection.permits_mutation(scope) {
                return Err(DeadlineProfileError::ScopeChangeForbidden.into());
            }
        }
        _ => return Err(inconsistent("base and initial scope must occur together")),
    }
    if let DeadlineProfileChange::Publish { definition } = &command.change {
        if preparation.base.is_some() {
            return Err(DeadlineProfileError::RevisionConflict.into());
        }
        return Ok((definition.clone(), DeadlineProfileAlgorithm::V1));
    }
    let base = preparation
        .base
        .as_ref()
        .ok_or(DeadlineProfileError::NotFound)?;
    if base.revision.get() != command.expected_revision() {
        return Err(DeadlineProfileError::RevisionConflict.into());
    }
    if base.status == DeadlineProfileStatus::Retired {
        return Err(DeadlineProfileError::Retired.into());
    }
    match &command.change {
        DeadlineProfileChange::Replace { definition, .. } => {
            if Some(definition.scope()) != preparation.initial_scope.as_ref() {
                return Err(DeadlineProfileError::ScopeChangeForbidden.into());
            }
            Ok((definition.clone(), DeadlineProfileAlgorithm::V1))
        }
        DeadlineProfileChange::Retire { .. } => Ok((base.definition.clone(), base.algorithm)),
        DeadlineProfileChange::Publish { .. } => unreachable!(),
    }
}

pub(super) fn validate_page(
    collection: DeadlineProfileCollection,
    page: &DeadlineProfilePage,
    query: &DeadlineProfileQuery,
) -> Result<(), ApplicationError> {
    if page.profiles.len() > query.limit() as usize
        || page.has_more && page.profiles.len() != query.limit() as usize
        || page.next_after_id
            != if page.has_more {
                page.profiles.last().map(|e| e.id)
            } else {
                None
            }
    {
        return Err(inconsistent("profile page size or cursor differs"));
    }
    let mut after = query.after_id();
    for e in &page.profiles {
        if !collection.includes(&e.scope)
            || after.is_some_and(|id| e.id.as_uuid() <= id.as_uuid())
            || query.status().status().is_some_and(|s| e.status != s)
        {
            return Err(inconsistent("profile page scope, order or filter differs"));
        }
        after = Some(e.id);
    }
    Ok(())
}

pub(super) fn validate_history(
    hasher: &dyn DocumentHasher,
    collection: DeadlineProfileCollection,
    id: DeadlineProfileId,
    page: &DeadlineProfileHistoryPage,
    query: DeadlineProfileHistoryQuery,
) -> Result<(), ApplicationError> {
    if page.revisions.is_empty() && query.before_revision().map(|v| v.get()) != Some(1) {
        return Err(inconsistent("profile history omits its first revision"));
    }
    if page.revisions.len() > query.limit() as usize
        || page.has_more && page.revisions.len() != query.limit() as usize
        || page.next_before_revision
            != if page.has_more {
                page.revisions.last().map(|e| e.revision)
            } else {
                None
            }
        || page.has_more && page.revisions.last().is_some_and(|e| e.revision.get() == 1)
    {
        return Err(inconsistent("profile history size or cursor differs"));
    }
    let mut operations = std::collections::HashSet::new();
    for e in &page.revisions {
        deadline_profile_history_receipt_matches(hasher, e)?;
        if !operations.insert(e.receipt.operation_id)
            || e.id != id
            || !collection.includes(&e.scope)
            || query
                .before_revision()
                .is_some_and(|before| e.revision >= before)
        {
            return Err(inconsistent(
                "profile history identity, scope or bound differs",
            ));
        }
    }
    for pair in page.revisions.windows(2) {
        if Some(pair[0].revision.get()) != pair[1].revision.get().checked_add(1)
            || pair[0].scope != pair[1].scope
            || pair[1].status == DeadlineProfileStatus::Retired
            || pair[0].status == DeadlineProfileStatus::Retired
                && (pair[0].definition_digest != pair[1].definition_digest
                    || pair[0].algorithm != pair[1].algorithm)
        {
            return Err(inconsistent(
                "profile history sequence, scope or retirement differs",
            ));
        }
    }
    if !page.has_more && page.revisions.last().is_some_and(|e| e.revision.get() != 1) {
        return Err(inconsistent(
            "profile history ends before its first revision",
        ));
    }
    Ok(())
}
