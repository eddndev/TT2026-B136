use super::{audit, decode, inconsistent, port, targets};
use crate::measure_decision_postgres::{load_measure_targets, LoadedMeasureHistory};
use application::{precautionary_hearings::*, ApplicationError};
use domain::{cases::CaseId, crypto::DocumentHasher, precautionary_hearings::*};
use postgres::{Row, Transaction};

const BOUNDS:&str="octet_length(action)<=8 AND COALESCE(octet_length(reason),0)<=4000
 AND COALESCE(octet_length(values_canonical),0)<=16395 AND COALESCE(octet_length(values_view::text),0)<=65536
 AND COALESCE(octet_length(previous_capture_digest),32)=32 AND COALESCE(octet_length(values_digest),32)=32
 AND octet_length(observed_context_digest)=32 AND octet_length(submission_digest)=32 AND octet_length(review_digest)=32 AND octet_length(capture_digest)=32
 AND octet_length(recorded_by_email) BETWEEN 1 AND 1280 AND octet_length(recorded_by_role)<=9
 AND COALESCE(octet_length(support_format),0)<=4 AND COALESCE(octet_length(support_policy),0)<=11";

pub(super) struct Prefix {
    rows: Vec<Row>,
    targets: Vec<Vec<PrecautionaryMeasureRef>>,
    pub(super) refs: Vec<PrecautionaryMeasureRef>,
}
impl Prefix {
    pub(super) fn len(&self) -> usize {
        self.rows.len()
    }
    pub(super) fn last_targets(&self) -> &[PrecautionaryMeasureRef] {
        self.targets.last().map(Vec::as_slice).unwrap_or(&[])
    }
}

pub(super) fn detail(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: PrecautionaryHearingId,
    selected: Option<PrecautionaryHearingRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<PrecautionaryHearingStoredOperation, ApplicationError> {
    let prefix = prefix(tx, case, id, selected)?;
    let measures = load_measure_targets(tx, case, &prefix.refs, hasher)?;
    reconstruct(tx, &prefix, &measures, hasher)
}

pub(super) fn prefix(
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
    let refs = targets::union(resolved.iter().map(Vec::as_slice))?;
    Ok(Prefix {
        rows,
        targets: resolved,
        refs,
    })
}

pub(super) fn reconstruct(
    tx: &mut Transaction<'_>,
    prefix: &Prefix,
    measures: &LoadedMeasureHistory,
    hasher: &dyn DocumentHasher,
) -> Result<PrecautionaryHearingStoredOperation, ApplicationError> {
    let mut captures = Vec::with_capacity(prefix.rows.len());
    for (index, row) in prefix.rows.iter().enumerate() {
        let previous_targets = index
            .checked_sub(1)
            .map(|i| prefix.targets[i].as_slice())
            .unwrap_or(&[]);
        let refs = targets::union([prefix.targets[index].as_slice(), previous_targets])?;
        let proof = measures.subclosure(&refs)?;
        let capture = decode::capture(tx, row, captures.last(), &proof, hasher)?;
        audit::verify(tx, &capture, hasher)?;
        captures.push(capture);
    }
    let first = captures
        .first()
        .ok_or_else(|| inconsistent("empty hearing prefix"))?;
    let first_proof = measures.subclosure(&prefix.targets[0])?;
    let origin = precautionary_hearing_origin_with_measure_history(hasher, first, &first_proof)
        .map_err(inconsistent)?;
    let measure_history = measures.subclosure(&prefix.refs)?;
    precautionary_hearing_history_with_measure_history_matches(
        hasher,
        &captures,
        &origin,
        &measure_history,
    )
    .map_err(inconsistent)?;
    Ok(PrecautionaryHearingStoredOperation {
        capture: captures
            .last()
            .ok_or_else(|| inconsistent("empty hearing prefix"))?
            .clone(),
        history: PrecautionaryHearingHistoryEvidence {
            origin,
            captures,
            measure_history,
        },
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
