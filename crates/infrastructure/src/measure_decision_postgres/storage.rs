use super::{audit, decode, inconsistent, port};
use application::{precautionary_measures::*, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    precautionary_measures::{MeasureDecisionId, MeasureDecisionOperationId},
};
use postgres::Transaction;
const DECISION_BOUNDS:&str="octet_length(d.values_canonical) BETWEEN 80 AND 16076 AND octet_length(d.values_view::text)<=32768
 AND octet_length(d.outcome_canonical) BETWEEN 11 AND 645450 AND octet_length(d.outcome_view::text)<=1048576
 AND octet_length(d.values_digest)=32 AND octet_length(d.outcome_digest)=32 AND octet_length(d.observed_context_digest)=32
 AND octet_length(d.support_format)<=4 AND octet_length(d.support_policy)<=11
 AND octet_length(d.recorded_by_email) BETWEEN 1 AND 1280 AND octet_length(d.recorded_by_role)<=9
 AND octet_length(d.submission_digest)=32 AND octet_length(d.review_digest)=32 AND octet_length(d.decision_digest)=32
 AND octet_length(d.group_digest)=32 AND octet_length(o.owner_digest)=32 AND octet_length(o.family)<=2";
const MEMBER_BOUNDS:&str="octet_length(r.values_canonical) BETWEEN 91 AND 20113 AND octet_length(r.values_view::text)<=32768
 AND octet_length(r.values_digest)=32 AND octet_length(r.capture_digest)=32 AND octet_length(r.subject_values_digest)=32
 AND octet_length(r.family)<=2 AND octet_length(r.action)<=6";

pub(super) fn detail(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: MeasureDecisionId,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureDecisionStoredOperation, ApplicationError> {
    audit::inventory_intact(tx)?;
    let row=tx.query_opt(&format!("SELECT d.*,o.family,o.owner_digest,o.audit_sequence FROM case_measure_decisions d JOIN case_measure_operations o ON o.operation_id=d.operation_id AND o.case_id=d.case_id WHERE d.case_id=$1 AND d.decision_id=$2 AND {DECISION_BOUNDS}"),&[&case.as_uuid(),&id.as_uuid()]).map_err(port)?;
    let Some(row) = row else {
        let exists:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM case_measure_decisions WHERE decision_id=$1 AND case_id=$2)",&[&id.as_uuid(),&case.as_uuid()]).map_err(port)?.get(0);
        if exists {
            return Err(inconsistent("decision payload exceeds storage bounds"));
        }
        audit::decision_absent(tx, id)?;
        return Err(MeasureDecisionError::NotFound.into());
    };
    let result = decode::group(tx, &row, hasher)?;
    let op = result.group.review.command.operation_id.as_uuid();
    let expected = result.group.measures.len();
    let counts=tx.query_one("SELECT (SELECT count(*) FROM (SELECT 1 FROM case_measure_revisions WHERE owner_operation=$1 LIMIT 33) r),(SELECT count(*) FROM (SELECT 1 FROM case_measures WHERE root_operation=$1 LIMIT 33) h)",&[&op]).map_err(port)?;
    if counts.get::<_, i64>(0) != expected as i64 || counts.get::<_, i64>(1) != expected as i64 {
        return Err(inconsistent("group sibling or root count differs"));
    }
    let rows=tx.query(&format!("SELECT r.*,h.initial_revision,h.root_operation FROM case_measure_revisions r JOIN case_measures h ON h.id=r.measure_id AND h.case_id=r.case_id WHERE r.owner_operation=$1 AND {MEMBER_BOUNDS} ORDER BY r.measure_id,r.revision LIMIT 33"),&[&op]).map_err(port)?;
    if rows.len() != expected {
        return Err(inconsistent("group sibling source is missing or oversized"));
    }
    for (row, capture) in rows.iter().zip(&result.group.measures) {
        decode::member(row, capture, hasher)?;
    }
    audit::verify(tx, &result.group, hasher)?;
    Ok(result)
}
pub(super) fn operation(
    tx: &mut Transaction<'_>,
    case: CaseId,
    op: MeasureDecisionOperationId,
    hasher: &dyn DocumentHasher,
) -> Result<Option<MeasureDecisionStoredOperation>, ApplicationError> {
    audit::inventory_intact(tx)?;
    let row=tx.query_opt("SELECT o.case_id,d.decision_id FROM case_measure_operations o JOIN case_measure_decisions d ON d.operation_id=o.operation_id AND d.case_id=o.case_id WHERE o.operation_id=$1",&[&op.as_uuid()]).map_err(port)?;
    let Some(row) = row else {
        audit::operation_absent(tx, op)?;
        return Ok(None);
    };
    if row.get::<_, uuid::Uuid>("case_id") != case.as_uuid() {
        return Err(MeasureDecisionError::OperationConflict.into());
    }
    detail(
        tx,
        case,
        MeasureDecisionId::from_uuid(row.get("decision_id")),
        hasher,
    )
    .map(Some)
}
