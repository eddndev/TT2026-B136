use super::{inconsistent, port};
pub(super) use crate::precautionary_hearing_postgres::sources::{current_context, exact_context};
use application::{
    case_stages::{StageDocumentFormat, StageSupportSnapshot},
    precautionary_measures::*,
    typed_participants::SubjectSnapshot,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    hearings::HearingSupportRef,
    precautionary_measures::{MeasureSupervision, MeasureValues},
    typed_participants::SubjectRevisionRef,
};
use postgres::Transaction;

pub(super) fn result_sources(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &MeasureDecisionCommand,
    hasher: &dyn DocumentHasher,
) -> Result<Vec<MeasureResultSources>, ApplicationError> {
    let mut result = Vec::new();
    if let Some(effects) = command.outcome.changes() {
        for effect in effects {
            let domain::precautionary_measures::MeasureEffect::Impose(proposal) = effect else {
                return Err(inconsistent(
                    "durable predecessor effects are not available",
                ));
            };
            result.push(MeasureResultSources {
                id: proposal.id,
                sources: selected(tx, case, &proposal.values, hasher)?,
            });
        }
    }
    Ok(result)
}
fn selected(
    tx: &mut Transaction<'_>,
    case: CaseId,
    values: &MeasureValues,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureSources, ApplicationError> {
    let sources = MeasureSources {
        subject: subject(tx, case, values.subject(), hasher)?,
        supervisor: match values.supervision() {
            MeasureSupervision::Unknown { .. } => None,
            MeasureSupervision::Known { participant, .. } => Some(
                crate::precautionary_hearing_postgres::sources::exact_participant(
                    tx,
                    case,
                    *participant,
                    hasher,
                )
                .map_err(inconsistent)?,
            ),
        },
    };
    resolve_measure_sources(hasher, case, values, &sources).map_err(inconsistent)?;
    Ok(sources)
}
fn subject(
    tx: &mut Transaction<'_>,
    case: CaseId,
    reference: SubjectRevisionRef,
    hasher: &dyn DocumentHasher,
) -> Result<SubjectSnapshot, ApplicationError> {
    let row=tx.query_opt("SELECT s.case_id,r.* FROM case_subjects s JOIN case_subject_revisions r ON r.subject_id=s.id WHERE s.case_id=$1 AND s.id=$2 AND r.revision=$3
 AND octet_length(r.values_canonical)<=5676 AND octet_length(r.values_view::text)<=65536
 AND octet_length(r.values_digest)=32 AND octet_length(r.identity_document_digest)=32
 AND octet_length(r.changed_at)<=64 AND octet_length(r.changed_by_email) BETWEEN 1 AND 1280
 AND octet_length(r.subject_kind)<=32 AND octet_length(r.display_name)<=800
 AND COALESCE(octet_length(r.declared_identifier),0)<=800 AND octet_length(r.identity_document_locator)<=2000",&[&case.as_uuid(),&reference.id.as_uuid(),&i64::from(reference.revision.get())]).map_err(port)?.ok_or_else(||inconsistent("exact subject is foreign, absent or oversized"))?;
    let snapshot = crate::typed_participant_postgres::storage::decode_subject(&row, hasher)
        .map_err(inconsistent)?;
    if snapshot.values_digest != reference.values_digest {
        return Err(inconsistent("selected subject digest differs"));
    }
    Ok(snapshot)
}
pub(super) fn support(
    tx: &mut Transaction<'_>,
    case: CaseId,
    reference: HearingSupportRef,
    format: StageDocumentFormat,
) -> Result<StageSupportSnapshot, ApplicationError> {
    crate::precautionary_hearing_postgres::sources::document(tx, case, reference, format)
        .map_err(inconsistent)
}
