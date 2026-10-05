use super::{audit, inconsistent, port};
use crate::measure_decision_postgres::{
    load_precautionary_history, HearingProofRef, HistoryReserve, HistoryRoot,
};
use application::{precautionary_hearings::*, ApplicationError};
use domain::{cases::CaseId, crypto::DocumentHasher, precautionary_hearings::*};
use postgres::{Row, Transaction};

const BOUNDS:&str="octet_length(action)<=8 AND COALESCE(octet_length(reason),0)<=4000
 AND COALESCE(octet_length(values_canonical),0)<=16395 AND COALESCE(octet_length(values_view::text),0)<=65536
 AND COALESCE(octet_length(previous_capture_digest),32)=32 AND COALESCE(octet_length(values_digest),32)=32
 AND octet_length(observed_context_digest)=32 AND octet_length(submission_digest)=32 AND octet_length(review_digest)=32 AND octet_length(capture_digest)=32
 AND octet_length(recorded_by_email) BETWEEN 1 AND 1280 AND octet_length(recorded_by_role)<=9
 AND COALESCE(octet_length(support_format),0)<=4 AND COALESCE(octet_length(support_policy),0)<=11";

pub(crate) struct Prefix {
    pub(crate) rows: Vec<Row>,
    pub(crate) targets: Vec<Vec<PrecautionaryMeasureRef>>,
}

pub(super) fn detail(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: PrecautionaryHearingId,
    selected: Option<PrecautionaryHearingRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<PrecautionaryHearingStoredOperation, ApplicationError> {
    let reference = selection(tx, case, id, selected)?;
    let loaded = load_precautionary_history(
        tx,
        case,
        &[HistoryRoot::Hearing(reference)],
        HistoryReserve::default(),
        hasher,
    )?;
    loaded.hearing_operation(reference, hasher)
}

pub(crate) fn prefix(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: PrecautionaryHearingId,
    selected: Option<PrecautionaryHearingRevision>,
) -> Result<Prefix, ApplicationError> {
    audit::hearing_intact(tx, id)?;
    let root = tx
        .query_opt(
            "SELECT initial_revision FROM case_precautionary_hearings WHERE id=$1 AND case_id=$2",
            &[&id.as_uuid(), &case.as_uuid()],
        )
        .map_err(port)?
        .ok_or(PrecautionaryHearingError::NotFound)?;
    if root.get::<_, i64>(0) != 1 {
        return Err(inconsistent("hearing origin revision differs"));
    }
    let head=tx.query_one("SELECT max(revision) FROM case_precautionary_hearing_revisions WHERE hearing_id=$1 AND case_id=$2",&[&id.as_uuid(),&case.as_uuid()]).map_err(port)?.get::<_,Option<i64>>(0).ok_or_else(||inconsistent("hearing root lacks revisions"))?;
    if !(1..=256).contains(&head) {
        return Err(inconsistent("hearing prefix exceeds bounds"));
    }
    let target = selected.map(|v| i64::from(v.get())).unwrap_or(head);
    if target > head {
        return Err(PrecautionaryHearingError::NotFound.into());
    }
    let rows=tx.query(&format!("SELECT * FROM case_precautionary_hearing_revisions WHERE hearing_id=$1 AND case_id=$2 AND revision<=$3 AND {BOUNDS} ORDER BY revision LIMIT 257"),&[&id.as_uuid(),&case.as_uuid(),&target]).map_err(port)?;
    if rows.len() != target as usize {
        return Err(inconsistent("hearing prefix is absent or oversized"));
    }
    let mut resolved: Vec<Vec<PrecautionaryMeasureRef>> = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        if row.get::<_, i64>("revision") != index as i64 + 1 {
            return Err(inconsistent("hearing prefix is not contiguous"));
        }
        let selected = if row.get::<_, String>("action") == "cancel" {
            resolved
                .last()
                .ok_or_else(|| inconsistent("cancellation has no predecessor"))?
                .clone()
        } else {
            let bytes = row
                .get::<_, Option<Vec<u8>>>("values_canonical")
                .ok_or_else(|| inconsistent("selected hearing values are absent"))?;
            let view = row
                .get::<_, Option<serde_json::Value>>("values_view")
                .ok_or_else(|| inconsistent("selected hearing projection is absent"))?;
            crate::precautionary_hearing_codec::values(&bytes, &view)?
                .review_targets()
                .to_vec()
        };
        resolved.push(selected);
    }
    Ok(Prefix {
        rows,
        targets: resolved,
    })
}

pub(crate) fn selection(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: PrecautionaryHearingId,
    selected: Option<PrecautionaryHearingRevision>,
) -> Result<HearingProofRef, ApplicationError> {
    audit::hearing_intact(tx, id)?;
    let revision = selected.map(|r| i64::from(r.get()));
    let row=tx.query_opt("SELECT revision,CASE WHEN octet_length(capture_digest)=32 THEN capture_digest ELSE NULL::bytea END AS capture_digest FROM case_precautionary_hearing_revisions WHERE case_id=$1 AND hearing_id=$2 AND ($3::bigint IS NULL OR revision=$3) ORDER BY revision DESC LIMIT 1",&[&case.as_uuid(),&id.as_uuid(),&revision]).map_err(port)?.ok_or(PrecautionaryHearingError::NotFound)?;
    Ok(HearingProofRef {
        hearing_id: id,
        revision: PrecautionaryHearingRevision::new(
            u32::try_from(row.get::<_, i64>("revision")).map_err(inconsistent)?,
        )
        .map_err(inconsistent)?,
        capture_digest: super::decode::digest(
            row.get::<_, Option<Vec<u8>>>("capture_digest")
                .ok_or_else(|| {
                    inconsistent("selected hearing capture digest has invalid length")
                })?,
        )?,
    })
}
pub(super) fn operation(
    tx: &mut Transaction<'_>,
    case: CaseId,
    op: PrecautionaryHearingOperationId,
    hasher: &dyn DocumentHasher,
) -> Result<Option<PrecautionaryHearingStoredOperation>, ApplicationError> {
    let Some(row)=tx.query_opt("SELECT case_id,hearing_id,revision FROM case_precautionary_hearing_revisions WHERE operation_id=$1",&[&op.as_uuid()]).map_err(port)? else {audit::operation_absent(tx,case,op)?;return Ok(None)};
    if row.get::<_, uuid::Uuid>("case_id") != case.as_uuid() {
        return Err(PrecautionaryHearingError::OperationConflict.into());
    }
    let id = PrecautionaryHearingId::from_uuid(row.get("hearing_id"));
    let revision = PrecautionaryHearingRevision::new(
        u32::try_from(row.get::<_, i64>("revision")).map_err(inconsistent)?,
    )
    .map_err(inconsistent)?;
    detail(tx, case, id, Some(revision), hasher).map(Some)
}
