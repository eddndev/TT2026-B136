use super::{header, inconsistent, port, projection};
use application::{deadline_profiles::*, ApplicationError};
use domain::{crypto::DocumentHasher, procedural_facts::FactLabel};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    id: DeadlineProfileId,
    revision: Option<DeadlineProfileRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<(DeadlineProfileHistoryEntry, FactLabel), ApplicationError> {
    let selected = raw(tx, id, revision, hasher)?;
    let initial = if selected.0.revision.get() == 1 {
        selected.clone()
    } else {
        raw(
            tx,
            id,
            Some(DeadlineProfileRevision::new(1).map_err(inconsistent)?),
            hasher,
        )
        .map_err(missing)?
    };
    if initial.0.receipt.action != DeadlineProfileAction::Publish
        || selected.0.scope != initial.0.scope
    {
        return Err(inconsistent(
            "profile summary scope differs from initial revision",
        ));
    }
    if selected.0.revision.get() > 1 {
        let prior = raw(
            tx,
            id,
            Some(
                DeadlineProfileRevision::new(selected.0.revision.get() - 1)
                    .map_err(inconsistent)?,
            ),
            hasher,
        )
        .map_err(missing)?
        .0;
        if prior.status != DeadlineProfileStatus::Published
            || prior.scope != initial.0.scope
            || (selected.0.status == DeadlineProfileStatus::Retired
                && (selected.0.definition_digest != prior.definition_digest
                    || selected.0.algorithm != prior.algorithm))
        {
            return Err(inconsistent(
                "profile summary successor or retirement differs",
            ));
        }
    }
    Ok(selected)
}
fn raw(
    tx: &mut Transaction<'_>,
    id: DeadlineProfileId,
    revision: Option<DeadlineProfileRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<(DeadlineProfileHistoryEntry, FactLabel), ApplicationError> {
    let revision = revision.map(|r| i64::from(r.get()));
    let row=tx.query_opt("SELECT r.profile_id,r.revision,r.definition_view,r.definition_digest,r.algorithm,r.operation_id,r.action,r.status,r.reason,
        r.submission_canonical,r.submission_digest,r.submission_view,r.recorded_at_seconds,r.recorded_at_nanoseconds,r.recorded_by,r.recorded_by_email,
        c.initial_revision,c.case_id AS scope_case_id FROM deadline_profile_revisions r JOIN deadline_profiles c ON c.id=r.profile_id
        WHERE r.profile_id=$1 AND ($2::bigint IS NULL OR r.revision=$2) ORDER BY r.revision DESC LIMIT 1", &[&id.as_uuid(),&revision]).map_err(port)?.ok_or(DeadlineProfileError::NotFound)?;
    let entry = header::row(&row, hasher)?;
    let (title, _) = projection::read(&row.try_get("definition_view").map_err(inconsistent)?)?;
    Ok((entry, title))
}
fn missing(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::DeadlineProfile(DeadlineProfileError::NotFound) => {
            inconsistent("profile historical summary is missing")
        }
        other => other,
    }
}
