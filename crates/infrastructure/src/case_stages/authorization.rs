use super::{decode::digest, inconsistent, port};
use application::case_stages::*;
use application::identity::Principal;
use application::ApplicationError;
use domain::case_administration::CaseRevision;
use domain::cases::CaseId;
use domain::crypto::Sha256Digest;
use domain::identity::{Role, UserId};
use postgres::Transaction;

pub(super) fn authorize(
    tx: &mut Transaction<'_>,
    actor: UserId,
    case: CaseId,
    action: CaseStageAction,
) -> Result<Principal, ApplicationError> {
    let principal = crate::postgres_actor::active_actor(tx, actor)?;
    if !principal.role.allows(action.permission()) {
        return Err(ApplicationError::PermissionDenied);
    }
    let visible = if principal.role == Role::Owner {
        tx.query_opt("SELECT id FROM cases WHERE id=$1", &[&case.as_uuid()])
    } else {
        tx.query_opt(
            "SELECT case_id FROM case_memberships WHERE case_id=$1 AND user_id=$2 FOR SHARE",
            &[&case.as_uuid(), &actor.as_uuid()],
        )
    }
    .map_err(port)?;
    visible.ok_or(ApplicationError::CaseNotFound)?;
    Ok(principal)
}

pub(super) fn context(
    tx: &mut Transaction<'_>,
    case: CaseId,
) -> Result<(CaseRevision, Sha256Digest), ApplicationError> {
    let row=tx.query_opt("SELECT revision,nuc,administrative_status,values_digest FROM case_administration_revisions WHERE case_id=$1 ORDER BY revision DESC LIMIT 1",&[&case.as_uuid()]).map_err(port)?.ok_or(ApplicationError::CaseStageProfileIncomplete)?;
    if row.get::<_, &str>("administrative_status") == "closed" {
        return Err(ApplicationError::CaseClosed);
    }
    if row.get::<_, Option<&str>>("nuc").is_none() {
        return Err(ApplicationError::CaseStageProfileIncomplete);
    }
    let revision = u32::try_from(row.get::<_, i64>("revision")).map_err(inconsistent)?;
    Ok((
        CaseRevision::new(revision).map_err(inconsistent)?,
        digest(row.get("values_digest"))?,
    ))
}

pub(super) fn check_head(
    current: &CurrentCaseStage,
    expected: CaseStageExpectation,
    change: &CaseStageChange,
) -> Result<CaseStageRevision, ApplicationError> {
    match change {
        CaseStageChange::Adopt(_) => {
            if expected != CaseStageExpectation::Unregistered {
                return Err(ApplicationError::InvalidInput(
                    "adoption expects an unregistered stage".into(),
                ));
            }
            if current.entry().is_some() {
                return Err(ApplicationError::CaseStageConflict);
            }
            Ok(CaseStageRevision::FIRST)
        }
        CaseStageChange::Transition(_) => {
            let entry = current.entry().ok_or(ApplicationError::CaseStageRequired)?;
            if expected != CaseStageExpectation::Revision(entry.stage_revision()) {
                return Err(ApplicationError::CaseStageConflict);
            }
            if !entry.stage().permits(change.stage()) {
                return Err(ApplicationError::CaseStageTransitionRejected);
            }
            entry
                .stage_revision()
                .next()
                .ok_or(ApplicationError::CaseStageRevisionExhausted)
        }
    }
}
