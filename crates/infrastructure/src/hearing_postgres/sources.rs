use super::{inconsistent, port};
use application::case_stages::{CaseStageEntry, CaseStageQuery};
use application::cases::CaseAdministrationSnapshot;
use application::participants::{
    ParticipantDetail, ParticipantOverview, ParticipantRevisionSnapshot,
};
use application::{hearings::*, ApplicationError};
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    cases::CaseId,
    crypto::DocumentHasher,
};
use postgres::Transaction;

pub(super) fn context(
    tx: &mut Transaction<'_>,
    case: CaseId,
    hasher: &dyn DocumentHasher,
) -> Result<HearingCaseContext, ApplicationError> {
    let detail = crate::cases::storage::detail(tx, case, hasher)?;
    Ok(HearingCaseContext {
        case_id: case,
        administration: detail.administration,
        stage: crate::case_stages::query::current(tx, case, hasher)?,
    })
}
pub(super) fn administration(
    tx: &mut Transaction<'_>,
    case: CaseId,
    revision: CaseRevision,
    hasher: &dyn DocumentHasher,
) -> Result<CaseAdministrationSnapshot, ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT * FROM case_administration_revisions WHERE case_id=$1 AND revision=$2",
            &[&case.as_uuid(), &i64::from(revision.get())],
        )
        .map_err(port)?
        .ok_or_else(|| inconsistent("hearing administration source is absent"))?;
    crate::cases::values::decode(&row, hasher)
}
pub(super) fn stage(
    tx: &mut Transaction<'_>,
    case: CaseId,
    revision: CaseStageRevision,
    hasher: &dyn DocumentHasher,
) -> Result<CaseStageEntry, ApplicationError> {
    let before = revision.get().checked_add(1);
    let page =
        crate::case_stages::query::history(tx, case, &CaseStageQuery::new(1, before)?, hasher)?;
    page.entries
        .into_iter()
        .find(|entry| entry.stage_revision() == revision)
        .ok_or_else(|| inconsistent("hearing stage source is absent"))
}
pub(super) fn participant(
    detail: ParticipantDetail,
) -> Result<HearingParticipantSnapshot, ApplicationError> {
    let values_digest = detail.values_digest();
    let overview = match &detail.revision {
        ParticipantRevisionSnapshot::Manual(value) => ParticipantOverview::from(value.as_ref()),
        ParticipantRevisionSnapshot::Typed(value) => {
            let subject = detail
                .bound_subject
                .as_ref()
                .ok_or_else(|| inconsistent("typed hearing participant has no identity source"))?;
            ParticipantOverview {
                case_id: value.case_id,
                id: value.id,
                revision: value.revision,
                display_name: subject.values.display_name().into(),
                procedural_role: value.values.kind().as_str().into(),
                organization: value.values.role().organization().map(str::to_owned),
                directory_status: value.values.directory_status(),
                kind: Some(value.values.kind()),
                subject: Some(value.values.subject()),
            }
        }
    };
    Ok(HearingParticipantSnapshot {
        overview,
        values_digest,
    })
}
pub(super) fn exact_participants(
    tx: &mut Transaction<'_>,
    case: CaseId,
    values: &HearingValues,
    hasher: &dyn DocumentHasher,
) -> Result<Vec<HearingParticipantSnapshot>, ApplicationError> {
    values
        .participants()
        .iter()
        .map(|reference| {
            let detail = crate::participant_postgres::storage::exact(
                tx,
                case,
                reference.id(),
                reference.revision(),
                hasher,
            )
            .map_err(|error| match error {
                ApplicationError::ParticipantNotFound => {
                    inconsistent("hearing participant source is absent")
                }
                other => other,
            })?;
            participant(detail)
        })
        .collect()
}
