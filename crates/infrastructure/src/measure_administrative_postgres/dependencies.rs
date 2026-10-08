use super::{inconsistent, inventory, port, storage};
use crate::measure_decision_postgres::{HearingProofRef, HistoryRoot};
use application::{precautionary_measures::MeasureDecisionAnchorRef, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    precautionary_hearings::PrecautionaryMeasureRef,
    precautionary_measures::{
        MeasureCorrectionOperationId, MeasureDecisionOperationId, MeasureEffect,
    },
};
use postgres::Transaction;
use std::collections::BTreeSet;
use uuid::Uuid;

mod hearings;

pub(super) fn roots(
    tx: &mut Transaction<'_>,
    case: CaseId,
    target: PrecautionaryMeasureRef,
    hasher: &dyn DocumentHasher,
) -> Result<Vec<HistoryRoot>, ApplicationError> {
    let (mut roots, hearing_refs) = hearings::roots(tx, case, target, hasher)?;
    let mut owners = BTreeSet::new();
    let mut after: Option<Uuid> = None;
    loop {
        let rows = tx
            .query(
                "SELECT operation_id FROM case_measure_decisions
            WHERE case_id=$1 AND ($2::uuid IS NULL OR operation_id>$2)
            ORDER BY operation_id LIMIT 8",
                &[&case.as_uuid(), &after],
            )
            .map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id: Uuid = row.get(0);
            let op = MeasureDecisionOperationId::from_uuid(id);
            let payload = crate::measure_decision_postgres::storage::raw(tx, case, op)?;
            let outcome = crate::measure_decision_postgres::advertised::outcome(&payload)?;
            judicial_audit(tx, &payload, hasher)?;
            let uses = outcome
                .changes()
                .unwrap_or(&[])
                .iter()
                .any(|effect| match effect {
                    MeasureEffect::Impose(_) => false,
                    MeasureEffect::Confirm { previous }
                    | MeasureEffect::Modify { previous, .. }
                    | MeasureEffect::Revoke { previous }
                    | MeasureEffect::Cease { previous } => *previous == target,
                    MeasureEffect::Substitute { predecessors, .. } => {
                        predecessors.contains(&target)
                    }
                });
            let anchor = match crate::measure_decision_postgres::anchors::reference(&payload)? {
                Some(MeasureDecisionAnchorRef::Precautionary {
                    hearing_id,
                    revision,
                    capture_digest,
                }) => hearing_refs.contains(&(
                    hearing_id.as_uuid(),
                    revision.get(),
                    *capture_digest.as_bytes(),
                )),
                _ => false,
            };
            if uses || anchor {
                owners.insert(id);
                bounded(owners.len())?;
                roots.push(HistoryRoot::Decision(op));
            }
            after = Some(id);
        }
    }
    after = None;
    loop {
        let rows = tx
            .query(
                "SELECT operation_id FROM case_measure_administrations
            WHERE case_id=$1 AND ($2::uuid IS NULL OR operation_id>$2)
            ORDER BY operation_id LIMIT 8",
                &[&case.as_uuid(), &after],
            )
            .map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id: Uuid = row.get(0);
            let op = MeasureCorrectionOperationId::from_uuid(id);
            let payload = storage::raw(tx, case, op)?;
            let command = inventory::advertised(tx, &payload, hasher)?;
            if command.target == target {
                if !owners.insert(id) {
                    return Err(inconsistent(
                        "dependency operation has multiple owner families",
                    ));
                }
                bounded(owners.len())?;
                roots.push(HistoryRoot::Administrative(op));
            }
            after = Some(id);
        }
    }
    Ok(roots)
}

fn bounded(count: usize) -> Result<(), ApplicationError> {
    if count > 255 {
        return Err(
            application::measure_corrections::MeasureAdministrativeError::IncompleteHistory.into(),
        );
    }
    Ok(())
}

fn judicial_audit(
    tx: &mut Transaction<'_>,
    row: &postgres::Row,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let group = super::decode::digest(row.get("group_digest"))?;
    let family: String = row.get("family");
    let prefix = match family.as_str() {
        "g1" => "mg1",
        "g2" => "mg2",
        _ => {
            return Err(inconsistent(
                "judicial declaration has another owner family",
            ))
        }
    };
    if group != super::decode::digest(row.get("owner_digest"))?
        || !matches!(
            row.get::<_, String>("recorded_by_role").as_str(),
            "owner" | "litigator"
        )
    {
        return Err(inconsistent(
            "judicial declaration owner or actor family differs",
        ));
    }
    let marker = format!(
        "{prefix}:case:{}:operation:{}:decision:{}:submission:{}:review:{}:decision_digest:{}:group:{}",
        row.get::<_, Uuid>("case_id"),
        row.get::<_, Uuid>("operation_id"),
        row.get::<_, Uuid>("decision_id"),
        super::decode::digest(row.get("submission_digest"))?.to_hex(),
        super::decode::digest(row.get("review_digest"))?.to_hex(),
        super::decode::digest(row.get("decision_digest"))?.to_hex(),
        group.to_hex(),
    );
    inventory::audit::verify(
        tx,
        row.get("audit_sequence"),
        &row.get::<_, String>("recorded_by_email"),
        "measure_decision.recorded",
        &marker,
        inventory::audit::time(row)?,
        hasher,
    )
}
