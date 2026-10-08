use super::{
    audit, authorize, load_precautionary_history, port, HistoryReserve, HistoryRoot,
    PostgresMeasureDecisionStore,
};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{identity::Principal, measure_corrections::*, ApplicationError};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{DocumentHasher, Sha256Digest},
    precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef},
};
use postgres::Transaction;

fn invalid(error: impl std::fmt::Display) -> ApplicationError {
    MeasureRecordReadError::StoredInconsistent(error.to_string()).into()
}
fn absent() -> ApplicationError {
    MeasureRecordReadError::NotFound.into()
}

fn selected_reference(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: MeasureId,
    revision: Option<MeasureRevision>,
) -> Result<PrecautionaryMeasureRef, ApplicationError> {
    // Select the actual row before checking its digest; a malformed head cannot
    // fall back to an older valid capture.
    let row = tx.query_opt("SELECT revision, CASE WHEN octet_length(capture_digest)=32 THEN capture_digest END AS capture_digest FROM case_measure_revisions WHERE case_id=$1 AND measure_id=$2 AND ($3::bigint IS NULL OR revision=$3) ORDER BY revision DESC LIMIT 1", &[&case.as_uuid(), &id.as_uuid(), &revision.map(|r| i64::from(r.get()))]).map_err(port)?.ok_or_else(absent)?;
    let revision =
        MeasureRevision::new(u32::try_from(row.get::<_, i64>("revision")).map_err(invalid)?)
            .map_err(invalid)?;
    let digest: [u8; 32] = row
        .get::<_, Option<Vec<u8>>>("capture_digest")
        .ok_or_else(|| invalid("selected record digest has invalid length"))?
        .try_into()
        .map_err(|_| invalid("selected record digest has invalid length"))?;
    Ok(PrecautionaryMeasureRef::new(
        id,
        revision,
        Sha256Digest::from_bytes(&digest).map_err(invalid)?,
    ))
}
fn detail(
    tx: &mut Transaction<'_>,
    case: CaseId,
    reference: PrecautionaryMeasureRef,
    hasher: &dyn DocumentHasher,
) -> Result<(MeasureRecordDetail, OffsetDateTime), ApplicationError> {
    let loaded = load_precautionary_history(
        tx,
        case,
        &[HistoryRoot::Measure(reference)],
        HistoryReserve::default(),
        hasher,
    )?;
    let record_history = loaded.record_subclosure(&[reference])?;
    let checked =
        resolve_measure_records_with_decision_history(hasher, case, &[reference], &record_history)
            .map_err(invalid)?;
    let target = checked
        .targets()
        .first()
        .ok_or_else(|| invalid("selected record is absent from its owning evidence"))?;
    let at = target.recorded_at();
    let record = target.record().clone();
    Ok((
        MeasureRecordDetail {
            case_id: case,
            reference,
            record,
            record_history,
        },
        at,
    ))
}
fn marker(case: CaseId, reference: PrecautionaryMeasureRef) -> String {
    format!(
        "case:{case}:measure:{}:revision:{}:capture:{}",
        reference.id(),
        reference.revision().get(),
        reference.digest()
    )
}

impl MeasureRecordReadStore for PostgresMeasureDecisionStore {
    fn list(
        &self,
        actor: &Principal,
        case: CaseId,
        query: MeasureRecordReadQuery,
    ) -> Result<MeasureRecordPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, false)?;
        audit::inventory_intact(&mut tx)?;
        let rows = tx.query("SELECT id FROM case_measures WHERE case_id=$1 AND ($2::uuid IS NULL OR id>$2) ORDER BY id LIMIT $3", &[&case.as_uuid(), &query.after_id().map(|id| id.as_uuid()), &(i64::from(query.limit())+1)]).map_err(port)?;
        let has_more = rows.len() > usize::from(query.limit());
        let mut items = Vec::with_capacity(rows.len().min(usize::from(query.limit())));
        let mut floor = None;
        for row in rows.iter().take(usize::from(query.limit())) {
            let reference =
                selected_reference(&mut tx, case, MeasureId::from_uuid(row.get(0)), None)?;
            let (item, at) = detail(&mut tx, case, reference, self.hasher.as_ref())?;
            floor = Some(floor.map_or(at, |f: OffsetDateTime| f.max(at)));
            items.push(item);
        }
        let next_after_id = if has_more {
            items.last().map(|v| v.reference.id())
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &actor.email,
            "measure_record.list",
            &format!("case:{case}"),
            self.now(floor)?,
        )?;
        tx.commit().map_err(port)?;
        Ok(MeasureRecordPage {
            case_id: case,
            items,
            has_more,
            next_after_id,
        })
    }
    fn get(
        &self,
        actor: &Principal,
        case: CaseId,
        id: MeasureId,
    ) -> Result<MeasureRecordDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, false)?;
        audit::inventory_intact(&mut tx)?;
        let reference = selected_reference(&mut tx, case, id, None)?;
        let (result, at) = detail(&mut tx, case, reference, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &actor.email,
            "measure_record.read",
            &marker(case, reference),
            self.now(Some(at))?,
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
    fn exact(
        &self,
        actor: &Principal,
        case: CaseId,
        reference: PrecautionaryMeasureRef,
    ) -> Result<MeasureRecordDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, false)?;
        audit::inventory_intact(&mut tx)?;
        if selected_reference(&mut tx, case, reference.id(), Some(reference.revision()))?
            != reference
        {
            return Err(absent());
        }
        let (result, at) = detail(&mut tx, case, reference, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &actor.email,
            "measure_record.exact",
            &marker(case, reference),
            self.now(Some(at))?,
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}
