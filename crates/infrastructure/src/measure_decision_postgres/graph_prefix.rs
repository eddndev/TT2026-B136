use super::{graph_budget::HearingBudget, inconsistent, port};
use application::ApplicationError;
use domain::cases::CaseId;
use postgres::Transaction;
use std::collections::BTreeMap;
use uuid::Uuid;

// Charge distinct scalar identities before transferring any hearing payload.
pub(super) fn admit(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: Uuid,
    selected: u32,
    budget: &mut HearingBudget,
) -> Result<BTreeMap<u32, usize>, ApplicationError> {
    if !(1..=256).contains(&selected) {
        return Err(inconsistent("hearing prefix exceeds bounds"));
    }
    let rows=tx.query("SELECT revision,action,CASE WHEN action='cancel' THEN 0
        WHEN jsonb_typeof(values_view->'review_targets')='array'
        THEN jsonb_array_length(values_view->'review_targets') ELSE -1 END AS targets
        FROM case_precautionary_hearing_revisions WHERE case_id=$1 AND hearing_id=$2 AND revision<=$3
        ORDER BY revision LIMIT 257",&[&case.as_uuid(),&id,&i64::from(selected)]).map_err(port)?;
    if rows.len() != selected as usize {
        return Err(inconsistent("hearing scalar prefix has a gap"));
    }
    let mut counts = BTreeMap::new();
    let mut previous = None;
    for (index, row) in rows.iter().enumerate() {
        let revision = index as u32 + 1;
        if row.get::<_, i64>("revision") != i64::from(revision) {
            return Err(inconsistent("hearing scalar prefix is unordered"));
        }
        let count = if row.get::<_, String>("action") == "cancel" {
            previous.ok_or_else(|| inconsistent("cancellation has no target predecessor"))?
        } else {
            usize::try_from(row.get::<_, i32>("targets")).map_err(inconsistent)?
        };
        budget.admit(id, revision, count)?;
        counts.insert(revision, count);
        previous = Some(count);
    }
    Ok(counts)
}
