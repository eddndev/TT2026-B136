use super::{audit, decode, inconsistent, port, write};
use application::{
    measure_corrections::*, precautionary_measures::MeasureDecisionRecordHistoryEvidence,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    precautionary_measures::{MeasureCorrectionOperationId, MeasureSupervision},
};
use postgres::{Row, Transaction};

const BOUNDS:&str="octet_length(a.action)<=24 AND octet_length(a.reason) BETWEEN 1 AND 4000
 AND COALESCE(octet_length(a.correction_canonical),38) BETWEEN 38 AND 20040
 AND COALESCE(octet_length(a.correction_view::text),0)<=32768 AND COALESCE(octet_length(a.correction_digest),32)=32
 AND COALESCE(octet_length(a.replacement_subject_values_digest),32)=32
 AND octet_length(a.target_capture_digest)=32 AND octet_length(a.observed_context_digest)=32
 AND octet_length(a.support_format)<=4 AND octet_length(a.support_policy)<=11
 AND octet_length(a.recorded_by_email) BETWEEN 1 AND 1280 AND octet_length(a.recorded_by_role)<=9
 AND octet_length(a.submission_digest)=32 AND octet_length(a.review_digest)=32 AND octet_length(a.capture_digest)=32
 AND octet_length(o.owner_digest)=32 AND octet_length(o.family)<=2";
pub(crate) fn raw(
    tx: &mut Transaction<'_>,
    case: CaseId,
    op: MeasureCorrectionOperationId,
) -> Result<Row, ApplicationError> {
    tx.query_opt(&format!("SELECT a.*,o.family,o.owner_digest,o.audit_sequence FROM case_measure_administrations a
        JOIN case_measure_operations o ON o.operation_id=a.operation_id AND o.case_id=a.case_id
        WHERE a.case_id=$1 AND a.operation_id=$2 AND {BOUNDS}"),&[&case.as_uuid(),&op.as_uuid()])
        .map_err(port)?.ok_or_else(||inconsistent("exact administrative payload is absent or oversized"))
}
pub(crate) fn reconstruct(
    tx: &mut Transaction<'_>,
    row: &Row,
    history: MeasureDecisionRecordHistoryEvidence,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureAdministrativeStoredOperation, ApplicationError> {
    let operation = decode::capture(tx, row, history, hasher)?;
    let id = operation.origin.operation_id.as_uuid();
    let counts=tx.query_one("SELECT
        (SELECT count(*) FROM (SELECT 1 FROM case_measure_revisions WHERE owner_operation=$1 LIMIT 3) rows),
        (SELECT count(*) FROM (SELECT 1 FROM case_measures WHERE root_operation=$1 LIMIT 2) roots)",&[&id]).map_err(port)?;
    let expected_roots = i64::from(operation.capture.review.replacement.is_some());
    if counts.get::<_, i64>(0) != operation.capture.records.len() as i64
        || counts.get::<_, i64>(1) != expected_roots
    {
        return Err(inconsistent(
            "administrative ownership is incomplete or has extra roots",
        ));
    }
    let rows=tx.query("SELECT r.*,root.initial_revision,root.root_operation FROM case_measure_revisions r
        JOIN case_measures root ON root.id=r.measure_id AND root.case_id=r.case_id WHERE r.owner_operation=$1
        AND octet_length(r.values_canonical) BETWEEN 91 AND 20113 AND octet_length(r.values_view::text)<=32768
        AND octet_length(r.values_digest)=32 AND octet_length(r.capture_digest)=32 AND octet_length(r.subject_values_digest)=32
        AND octet_length(r.family)<=2 AND octet_length(r.action)<=14 AND octet_length(r.validity)<=16
        ORDER BY r.measure_id,r.revision LIMIT 3",
        &[&id]).map_err(port)?;
    if rows.len() != operation.capture.records.len() {
        return Err(inconsistent(
            "administrative record source is absent or oversized",
        ));
    }
    for (row, capture) in rows.iter().zip(&operation.capture.records) {
        member(row, capture, hasher)?;
    }
    audit::verify(tx, &operation.capture, hasher)?;
    Ok(operation)
}
fn member(
    row: &Row,
    capture: &MeasureAdministrativeRecordCapture,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let result = &capture.result;
    let subject = result.values.subject();
    if row.get::<_, uuid::Uuid>("measure_id") != result.id.as_uuid()
        || row.get::<_, i64>("revision") != i64::from(result.revision.get())
        || row.get::<_, uuid::Uuid>("case_id") != capture.case_id.as_uuid()
        || row.get::<_, uuid::Uuid>("owner_operation") != capture.operation_id.as_uuid()
        || row.get::<_, String>("family") != "c1"
        || row.get::<_, String>("action") != write::retained_action(result.last_action)
        || row.get::<_, String>("validity") != write::validity(result.validity)
    {
        return Err(inconsistent(
            "administrative member identity, owner or action differs",
        ));
    }
    let bytes: Vec<u8> = row.get("values_canonical");
    if crate::measure_decision_codec::measure_values(&bytes, &row.get("values_view"))?
        != result.values
        || hasher.hash_bytes(&bytes) != decode::digest(row.get("values_digest"))?
        || capture.capture_digest != decode::digest(row.get("capture_digest"))?
    {
        return Err(inconsistent(
            "administrative member differs from reconstructed values",
        ));
    }
    if row.get::<_, uuid::Uuid>("subject_id") != subject.id.as_uuid()
        || row.get::<_, i64>("subject_revision") != i64::from(subject.revision.get())
        || decode::digest(row.get("subject_values_digest"))? != subject.values_digest
    {
        return Err(inconsistent("administrative subject selectors differ"));
    }
    let actual = (
        row.get::<_, Option<uuid::Uuid>>("supervisor_id"),
        row.get::<_, Option<i64>>("supervisor_revision"),
    );
    let expected = match result.values.supervision() {
        MeasureSupervision::Known { participant, .. } => (
            Some(participant.id().as_uuid()),
            Some(i64::from(participant.revision().get())),
        ),
        MeasureSupervision::Unknown { .. } => (None, None),
    };
    let root = match &result.record_root {
        MeasureRecordRoot::Judicial(origin) => origin.operation_id.as_uuid(),
        MeasureRecordRoot::Administrative { operation_id, .. } => operation_id.as_uuid(),
    };
    if actual != expected
        || row.get::<_, i64>("initial_revision") != 1
        || row.get::<_, uuid::Uuid>("root_operation") != root
    {
        return Err(inconsistent(
            "administrative retained supervisor or root differs",
        ));
    }
    Ok(())
}
pub(super) fn operation(
    tx: &mut Transaction<'_>,
    case: CaseId,
    op: MeasureCorrectionOperationId,
    hasher: &dyn DocumentHasher,
) -> Result<Option<MeasureAdministrativeStoredOperation>, ApplicationError> {
    crate::measure_decision_postgres::audit::inventory_intact(tx)?;
    let owner = tx
        .query_opt(
            "SELECT case_id,family FROM case_measure_operations WHERE operation_id=$1
        AND octet_length(family)<=2",
            &[&op.as_uuid()],
        )
        .map_err(port)?;
    let Some(owner) = owner else {
        audit::operation_absent(tx, op)?;
        return Ok(None);
    };
    if owner.get::<_, uuid::Uuid>("case_id") != case.as_uuid()
        || owner.get::<_, String>("family") != "a1"
    {
        return Err(MeasureAdministrativeError::OperationConflict.into());
    }
    let loaded = crate::measure_decision_postgres::load_precautionary_history(
        tx,
        case,
        &[crate::measure_decision_postgres::HistoryRoot::Administrative(op)],
        crate::measure_decision_postgres::HistoryReserve::default(),
        hasher,
    )?;
    loaded.into_administrative_operation(op).map(Some)
}
