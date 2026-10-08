use super::{
    audit, inconsistent, load_precautionary_history, port, record_decode, storage, HistoryReserve,
    HistoryRoot,
};
use application::{precautionary_measures::*, ApplicationError};
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_measures::MeasureDecisionOperationId,
};
use postgres::{Row, Transaction};

pub(super) fn reconstruct(
    tx: &mut Transaction<'_>,
    row: &Row,
    history: MeasureDecisionRecordHistoryEvidence,
    anchor: Option<MeasureDecisionAnchorMaterial>,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureDecisionRecordStoredOperation, ApplicationError> {
    let result = record_decode::group(tx, row, history, anchor, hasher)?;
    let rows = storage::member_rows(
        tx,
        result.origin.operation_id.as_uuid(),
        result.group.measures.len(),
        result
            .group
            .measures
            .iter()
            .filter(|m| m.result.previous.is_none())
            .count(),
    )?;
    for (row, capture) in rows.iter().zip(&result.group.measures) {
        record_decode::member(row, capture, hasher)?;
    }
    audit::verify_record(tx, &result.group, hasher)?;
    Ok(result)
}

pub(super) fn operation(
    tx: &mut Transaction<'_>,
    case: CaseId,
    operation: MeasureDecisionOperationId,
    hasher: &dyn DocumentHasher,
) -> Result<Option<MeasureDecisionRecordReceipt>, ApplicationError> {
    audit::inventory_intact(tx)?;
    let row = tx.query_opt("SELECT o.case_id,o.family,d.decision_id FROM case_measure_operations o LEFT JOIN case_measure_decisions d ON d.operation_id=o.operation_id AND d.case_id=o.case_id WHERE o.operation_id=$1", &[&operation.as_uuid()]).map_err(port)?;
    let Some(row) = row else {
        audit::operation_absent(tx, operation)?;
        return Ok(None);
    };
    let family: String = row.get("family");
    if row.get::<_, uuid::Uuid>("case_id") != case.as_uuid()
        || !matches!(family.as_str(), "g1" | "g2")
    {
        return Err(MeasureDecisionError::OperationConflict.into());
    }
    if row.get::<_, Option<uuid::Uuid>>("decision_id").is_none() {
        return Err(inconsistent("judicial record owner has no payload"));
    }
    let loaded = load_precautionary_history(
        tx,
        case,
        &[HistoryRoot::Decision(operation)],
        HistoryReserve::default(),
        hasher,
    )?;
    Ok(Some(match family.as_str() {
        "g1" => {
            MeasureDecisionRecordReceipt::V1(Box::new(loaded.into_operation(operation.as_uuid())?))
        }
        "g2" => {
            MeasureDecisionRecordReceipt::V2(Box::new(loaded.into_record_operation(operation)?))
        }
        _ => return Err(inconsistent("unsupported judicial record family")),
    }))
}

pub(super) fn detail(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: domain::precautionary_measures::MeasureDecisionId,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureDecisionRecordReceipt, ApplicationError> {
    audit::inventory_intact(tx)?;
    let row = tx
        .query_opt(
            "SELECT operation_id FROM case_measure_decisions WHERE case_id=$1 AND decision_id=$2",
            &[&case.as_uuid(), &id.as_uuid()],
        )
        .map_err(port)?;
    let Some(row) = row else {
        audit::decision_absent(tx, id)?;
        return Err(MeasureDecisionError::NotFound.into());
    };
    operation(
        tx,
        case,
        MeasureDecisionOperationId::from_uuid(row.get(0)),
        hasher,
    )?
    .ok_or_else(|| inconsistent("selected decision owner is absent"))
}
