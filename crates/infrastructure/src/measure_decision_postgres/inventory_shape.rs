use super::{advertised, inconsistent, port, storage};
use application::ApplicationError;
use domain::{
    cases::CaseId, precautionary_hearings::PrecautionaryMeasureRef, precautionary_measures::*,
};
use postgres::Transaction;
use uuid::Uuid;

/// Validate one bounded outcome at a time, including disconnected zero-row owners.
pub(super) fn validate(tx: &mut Transaction<'_>) -> Result<(), ApplicationError> {
    let mut after: Option<Uuid> = None;
    loop {
        let rows=tx.query("SELECT operation_id,case_id FROM case_measure_decisions WHERE ($1::uuid IS NULL OR operation_id>$1) ORDER BY operation_id LIMIT 8",&[&after]).map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let op: Uuid = row.get("operation_id");
            let case: Uuid = row.get("case_id");
            let payload = storage::raw(
                tx,
                CaseId::from_uuid(case),
                MeasureDecisionOperationId::from_uuid(op),
            )?;
            let outcome = advertised::outcome(&payload)?;
            let family: String = payload.get("family");
            let mut expected: Vec<(Uuid, i64, String)> = Vec::new();
            let mut roots = Vec::new();
            for effect in outcome.changes().unwrap_or(&[]) {
                match effect {
                    MeasureEffect::Impose(proposal) => {
                        expected.push((proposal.id.as_uuid(), 1, "impose".into()));
                        roots.push(proposal.id.as_uuid());
                    }
                    MeasureEffect::Confirm { previous }
                    | MeasureEffect::Modify { previous, .. }
                    | MeasureEffect::Revoke { previous }
                    | MeasureEffect::Cease { previous } => {
                        let action = match effect {
                            MeasureEffect::Confirm { .. } => "confirm",
                            MeasureEffect::Modify { .. } => "modify",
                            MeasureEffect::Revoke { .. } => "revoke",
                            _ => "cease",
                        };
                        expected.push(prior(tx, case, *previous, action, &family)?);
                    }
                    MeasureEffect::Substitute {
                        predecessors,
                        successors,
                    } => {
                        for reference in predecessors {
                            expected.push(prior(tx, case, *reference, "substitute_out", &family)?);
                        }
                        for proposal in successors {
                            expected.push((proposal.id.as_uuid(), 1, "substitute_in".into()));
                            roots.push(proposal.id.as_uuid());
                        }
                    }
                }
            }
            expected.sort();
            roots.sort();
            let actual=tx.query("SELECT measure_id,revision,action FROM case_measure_revisions WHERE owner_operation=$1 AND case_id=$2 AND octet_length(action)<=14 ORDER BY measure_id,revision LIMIT 33",&[&op,&case]).map_err(port)?.iter().map(|r|(r.get::<_,Uuid>(0),r.get::<_,i64>(1),r.get::<_,String>(2))).collect::<Vec<_>>();
            let actual_roots=tx.query("SELECT id FROM case_measures WHERE root_operation=$1 AND case_id=$2 ORDER BY id LIMIT 33",&[&op,&case]).map_err(port)?.iter().map(|r|r.get::<_,Uuid>(0)).collect::<Vec<_>>();
            if actual != expected || actual_roots != roots {
                return Err(inconsistent(
                    "advertised measure members or original roots are missing or contradictory",
                ));
            }
            after = Some(op);
        }
    }
    Ok(())
}
fn prior(
    tx: &mut Transaction<'_>,
    case: Uuid,
    reference: PrecautionaryMeasureRef,
    action: &str,
    family: &str,
) -> Result<(Uuid, i64, String), ApplicationError> {
    let revision = reference
        .revision()
        .get()
        .checked_add(1)
        .ok_or_else(|| inconsistent("predecessor revision overflows"))?;
    let exists:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM case_measure_revisions WHERE case_id=$1 AND measure_id=$2 AND revision=$3 AND capture_digest=$4
        AND (($5='g1' AND family='m1') OR ($5='g2' AND family IN ('m1','m2','c1')))
        AND validity='valid' AND action NOT IN ('revoke','cease','substitute_out'))",&[&case,&reference.id().as_uuid(),&i64::from(reference.revision().get()),&reference.digest().as_bytes().as_slice(),&family]).map_err(port)?.get(0);
    if !exists {
        return Err(inconsistent(
            "advertised predecessor is absent, terminal or contradictory",
        ));
    }
    Ok((reference.id().as_uuid(), i64::from(revision), action.into()))
}
