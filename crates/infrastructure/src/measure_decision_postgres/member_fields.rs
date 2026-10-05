use super::{decode::digest, inconsistent};
use application::{precautionary_measures::MeasureCaptureAction, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    precautionary_hearings::{MeasureId, MeasureRevision},
    precautionary_measures::{MeasureDecisionOperationId, MeasureSupervision, MeasureValues},
};
use postgres::Row;

pub(super) struct MemberFields<'a> {
    pub id: MeasureId,
    pub revision: MeasureRevision,
    pub case: CaseId,
    pub operation: MeasureDecisionOperationId,
    pub family: &'static str,
    pub action: MeasureCaptureAction,
    pub values: &'a MeasureValues,
    pub digest: Sha256Digest,
    pub root_operation: uuid::Uuid,
}
pub(super) fn validate(
    row: &Row,
    expected: MemberFields<'_>,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    if row.get::<_, uuid::Uuid>("measure_id") != expected.id.as_uuid()
        || row.get::<_, i64>("revision") != i64::from(expected.revision.get())
        || row.get::<_, uuid::Uuid>("case_id") != expected.case.as_uuid()
        || row.get::<_, uuid::Uuid>("owner_operation") != expected.operation.as_uuid()
        || row.get::<_, String>("family") != expected.family
        || row.get::<_, String>("validity") != "valid"
        || row.get::<_, String>("action") != super::decode::action_name(expected.action)
    {
        return Err(inconsistent("member identity or owner differs"));
    }
    let bytes: Vec<u8> = row.get("values_canonical");
    let values = crate::measure_decision_codec::measure_values(&bytes, &row.get("values_view"))?;
    if values != *expected.values
        || hasher.hash_bytes(&bytes) != digest(row.get("values_digest"))?
        || expected.digest != digest(row.get("capture_digest"))?
    {
        return Err(inconsistent("member values or commitment differ"));
    }
    let subject = values.subject();
    if row.get::<_, uuid::Uuid>("subject_id") != subject.id.as_uuid()
        || row.get::<_, i64>("subject_revision") != i64::from(subject.revision.get())
        || digest(row.get("subject_values_digest"))? != subject.values_digest
    {
        return Err(inconsistent("member subject selectors differ"));
    }
    let pair = (
        row.get::<_, Option<uuid::Uuid>>("supervisor_id"),
        row.get::<_, Option<i64>>("supervisor_revision"),
    );
    let expected_supervisor = match values.supervision() {
        MeasureSupervision::Unknown { .. } => (None, None),
        MeasureSupervision::Known { participant, .. } => (
            Some(participant.id().as_uuid()),
            Some(i64::from(participant.revision().get())),
        ),
    };
    if pair != expected_supervisor {
        return Err(inconsistent("member supervisor selectors differ"));
    }
    if row.get::<_, i64>("initial_revision") != 1
        || row.get::<_, uuid::Uuid>("root_operation") != expected.root_operation
    {
        return Err(inconsistent(
            "measure root does not belong to original member",
        ));
    }
    Ok(())
}
