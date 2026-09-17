use super::{inconsistent, port, storage};
use application::{
    case_stages::StageSupportSnapshot, hearing_results::*, hearings::HearingDetail,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    hearings::{HearingId, HearingRevision},
};
use postgres::Transaction;

pub(super) fn anchor(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: HearingId,
    revision: HearingRevision,
    hasher: &dyn DocumentHasher,
) -> Result<HearingDetail, ApplicationError> {
    crate::hearing_postgres::storage::detail(tx, case, id, Some(revision), hasher).map_err(|e| {
        match e {
            ApplicationError::Hearing(application::hearings::HearingError::NotFound) => {
                HearingResultError::ReferenceNotFound.into()
            }
            other => other,
        }
    })
}
pub(super) fn continuation(
    tx: &mut Transaction<'_>,
    case: CaseId,
    reference: HearingResultContinuationRef,
    hasher: &dyn DocumentHasher,
) -> Result<HearingResultSnapshot, ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT hearing_id FROM case_hearing_results WHERE id=$1 AND case_id=$2",
            &[&reference.id().as_uuid(), &case.as_uuid()],
        )
        .map_err(port)?
        .ok_or(HearingResultError::ReferenceNotFound)?;
    let hearing = HearingId::from_uuid(row.get(0));
    storage::snapshot(
        tx,
        case,
        hearing,
        reference.id(),
        reference.revision(),
        hasher,
    )
    .map_err(|e| match e {
        ApplicationError::HearingResult(HearingResultError::NotFound) => {
            HearingResultError::ReferenceNotFound.into()
        }
        other => other,
    })
}
pub(super) fn attendees(
    tx: &mut Transaction<'_>,
    case: CaseId,
    values: &HearingResultValues,
    hasher: &dyn DocumentHasher,
) -> Result<Vec<HearingResultAttendeeSnapshot>, ApplicationError> {
    values
        .attendees()
        .iter()
        .map(|reference| {
            let detail = crate::participant_postgres::storage::exact(
                tx,
                case,
                reference.participant_id(),
                reference.revision(),
                hasher,
            )
            .map_err(|e| match e {
                ApplicationError::ParticipantNotFound => {
                    HearingResultError::ReferenceNotFound.into()
                }
                other => other,
            })?;
            let subject_digest = detail
                .bound_subject
                .as_ref()
                .map(|subject| subject.values_digest);
            let participant = crate::hearing_postgres::participant(detail)?;
            if subject_digest
                != participant
                    .overview
                    .subject
                    .map(|subject| subject.values_digest)
            {
                return Err(inconsistent("result attendee identity digest differs"));
            }
            Ok(HearingResultAttendeeSnapshot {
                participant,
                subject_digest,
            })
        })
        .collect()
}
pub(super) fn support(
    tx: &mut Transaction<'_>,
    case: CaseId,
    captured: &Option<StageSupportSnapshot>,
) -> Result<(), ApplicationError> {
    if let Some(support) = captured {
        let row=tx.query_opt("SELECT name,digest FROM documents WHERE case_id=$1 AND id=$2 AND version=$3 AND octet_length(name)<=128 AND octet_length(digest)=32",&[&case.as_uuid(),&support.reference.id.as_uuid(),&i64::from(support.reference.version.get())]).map_err(port)?.ok_or_else(||inconsistent("result support source is absent or unbounded"))?;
        if row.get::<_, String>(0) != support.name
            || super::decode::digest(row.get(1))? != support.digest
        {
            return Err(inconsistent("result support source differs"));
        }
    }
    Ok(())
}
