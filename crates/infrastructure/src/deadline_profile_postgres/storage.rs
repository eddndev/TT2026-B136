use super::{decode, inconsistent, port};
use application::{deadline_profiles::*, ApplicationError};
use domain::crypto::DocumentHasher;
use postgres::Transaction;

pub(crate) fn detail(
    tx: &mut Transaction<'_>,
    id: DeadlineProfileId,
    revision: Option<DeadlineProfileRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<DeadlineProfileDetail, ApplicationError> {
    let selected = raw(tx, id, revision, hasher)?;
    let initial = if selected.revision.get() == 1 {
        selected.clone()
    } else {
        raw(
            tx,
            id,
            Some(DeadlineProfileRevision::new(1).map_err(inconsistent)?),
            hasher,
        )
        .map_err(source_error)?
    };
    if initial.receipt.action != DeadlineProfileAction::Publish
        || selected.definition.scope() != initial.definition.scope()
    {
        return Err(inconsistent(
            "profile scope differs from its initial revision",
        ));
    }
    if selected.revision.get() > 1 {
        let prior = raw(
            tx,
            id,
            Some(DeadlineProfileRevision::new(selected.revision.get() - 1).map_err(inconsistent)?),
            hasher,
        )
        .map_err(source_error)?;
        if prior.status != DeadlineProfileStatus::Published
            || prior.definition.scope() != initial.definition.scope()
            || (selected.status == DeadlineProfileStatus::Retired
                && (selected.definition != prior.definition
                    || selected.definition_digest != prior.definition_digest
                    || selected.algorithm != prior.algorithm))
        {
            return Err(inconsistent("profile successor or retired values differ"));
        }
    }
    Ok(selected)
}
fn raw(
    tx: &mut Transaction<'_>,
    id: DeadlineProfileId,
    revision: Option<DeadlineProfileRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<DeadlineProfileDetail, ApplicationError> {
    let number = revision.map(|v| i64::from(v.get()));
    let row = tx
        .query_opt(
            "SELECT r.*,c.initial_revision,c.case_id AS scope_case_id FROM deadline_profile_revisions r
        JOIN deadline_profiles c ON c.id=r.profile_id WHERE r.profile_id=$1
        AND ($2::bigint IS NULL OR r.revision=$2) ORDER BY r.revision DESC LIMIT 1",
            &[&id.as_uuid(), &number],
        )
        .map_err(port)?;
    match row {
        Some(row) => decode::row(&row, hasher),
        None => {
            if revision.is_none()
                && tx
                    .query_opt(
                        "SELECT id FROM deadline_profiles WHERE id=$1",
                        &[&id.as_uuid()],
                    )
                    .map_err(port)?
                    .is_some()
            {
                return Err(inconsistent("profile root has no initial revision"));
            }
            Err(DeadlineProfileError::NotFound.into())
        }
    }
}
fn source_error(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::DeadlineProfile(DeadlineProfileError::NotFound) => {
            inconsistent("profile historical revision is missing")
        }
        other => other,
    }
}

pub(super) use super::summary::load as summary;
