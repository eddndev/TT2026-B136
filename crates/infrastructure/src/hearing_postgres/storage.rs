use super::{decode, inconsistent, port, sources};
use application::case_stages::CaseStageEntry;
use application::{hearings::*, ApplicationError};
use domain::{
    case_administration::CaseAdministrativeStatus, cases::CaseId, crypto::DocumentHasher,
};
use postgres::Transaction;

pub(crate) fn detail(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: HearingId,
    revision: Option<HearingRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<HearingDetail, ApplicationError> {
    let mut detail = raw(tx, case, id, revision, hasher)?;
    let snapshot = &detail.snapshot;
    let scheduling = sources::administration(
        tx,
        case,
        snapshot.scheduling_context.administration_revision,
        hasher,
    )?;
    let recorded =
        sources::administration(tx, case, snapshot.recorded_administration_revision, hasher)?;
    if scheduling.values_digest != snapshot.scheduling_context.administration_digest
        || scheduling.values.profile().is_none()
        || scheduling.values.status() != CaseAdministrativeStatus::Active
        || recorded.values_digest != snapshot.recorded_administration_digest
        || recorded.values.status() != CaseAdministrativeStatus::Active
    {
        return Err(inconsistent("hearing administrative sources differ"));
    }
    let stage = sources::stage(tx, case, snapshot.scheduling_context.stage_revision, hasher)?;
    let stage_digest = match &stage {
        CaseStageEntry::Initial(_) => None,
        CaseStageEntry::Changed(value) => Some(value.values_digest),
    };
    if stage.stage() != snapshot.scheduling_context.stage
        || stage_digest != snapshot.scheduling_context.stage_digest
    {
        return Err(inconsistent("hearing stage source differs"));
    }
    detail.participants = sources::exact_participants(tx, case, &snapshot.values, hasher)?;
    if detail.participants.iter().any(|participant| {
        participant.overview.directory_status != application::participants::DirectoryStatus::Active
    }) {
        return Err(inconsistent(
            "hearing selected an archived historical snapshot",
        ));
    }
    if let Some(support) = &detail.support {
        let row=tx.query_opt("SELECT name,digest FROM documents WHERE case_id=$1 AND id=$2 AND version=$3 AND octet_length(name)<=128 AND octet_length(digest)=32",
            &[&case.as_uuid(),&support.reference.id.as_uuid(),&i64::from(support.reference.version.get())]).map_err(port)?
            .ok_or_else(||inconsistent("hearing support source is absent or unbounded"))?;
        if row.get::<_, String>(0) != support.name || decode::digest(row.get(1))? != support.digest
        {
            return Err(inconsistent("hearing support source differs"));
        }
    }
    hearing_receipt_matches(hasher, &detail)?;
    if snapshot.revision.get() > 1 {
        let previous = raw(
            tx,
            case,
            id,
            Some(HearingRevision::new(snapshot.revision.get() - 1).map_err(inconsistent)?),
            hasher,
        )?;
        if previous.snapshot.status != HearingStatus::Scheduled
            || previous.snapshot.values.kind() != snapshot.values.kind()
        {
            return Err(inconsistent(
                "hearing history does not preserve kind and scheduled predecessor",
            ));
        }
        if snapshot.receipt.action == HearingAction::Cancel
            && (snapshot.values != previous.snapshot.values
                || snapshot.scheduling_context != previous.snapshot.scheduling_context
                || detail.support != previous.support)
        {
            return Err(inconsistent(
                "hearing cancellation changed its historical programming",
            ));
        }
    }
    Ok(detail)
}
fn raw(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: HearingId,
    revision: Option<HearingRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<HearingDetail, ApplicationError> {
    let revision = revision.map(|value| i64::from(value.get()));
    let probe=tx.query_opt("SELECT revision,octet_length(values_canonical) BETWEEN 27 AND 10726
        AND octet_length(submission_canonical) BETWEEN 108 AND 4120
        AND octet_length(values_view::text)<=65536 AND octet_length(submission_view::text)<=32768
        AND octet_length(recorded_by_email)<=1280 AND COALESCE(octet_length(reason),0)<=4000
        AND COALESCE(octet_length(support_name),0)<=128 AS bounded
        FROM case_hearing_revisions WHERE hearing_id=$1 AND case_id=$2 AND ($3::bigint IS NULL OR revision=$3)
        ORDER BY revision DESC LIMIT 1", &[&id.as_uuid(),&case.as_uuid(),&revision]).map_err(port)?
        .ok_or(HearingError::NotFound)?;
    if !probe.get::<_, bool>("bounded") {
        return Err(inconsistent("hearing persisted fields exceed bounds"));
    }
    let selected: i64 = probe.get("revision");
    let row=tx.query_opt("SELECT * FROM case_hearing_revisions WHERE hearing_id=$1 AND case_id=$2 AND revision=$3
        AND octet_length(values_canonical) BETWEEN 27 AND 10726 AND octet_length(submission_canonical) BETWEEN 108 AND 4120
        AND octet_length(values_view::text)<=65536 AND octet_length(submission_view::text)<=32768
        AND octet_length(recorded_by_email)<=1280 AND COALESCE(octet_length(reason),0)<=4000
        AND COALESCE(octet_length(support_name),0)<=128",&[&id.as_uuid(),&case.as_uuid(),&selected]).map_err(port)?
        .ok_or_else(||inconsistent("hearing changed while loading an immutable revision"))?;
    decode::row(&row, hasher)
}
