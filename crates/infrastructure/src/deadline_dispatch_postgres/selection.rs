use super::{inconsistent, port};
use application::{
    deadline_dispatch::DeadlineDispatchLimit, deadlines::DeadlineId, ApplicationError,
};
use domain::cases::CaseId;
use postgres::Transaction;

#[derive(Clone, Copy)]
pub(super) struct Candidate {
    pub id: DeadlineId,
    pub case_id: CaseId,
}

pub(super) fn page(
    tx: &mut Transaction<'_>,
    event: Option<u64>,
    after: Option<DeadlineId>,
    limit: DeadlineDispatchLimit,
) -> Result<Vec<Candidate>, ApplicationError> {
    let sequence = event.map(i64::try_from).transpose().map_err(inconsistent)?;
    let count = i32::try_from(limit.get()).map_err(inconsistent)? + 1;
    let rows = tx
        .query(
            "SELECT c.deadline_id,c.case_id FROM deadline_dispatch_candidates(
            $1,$2,FALSE,NULL,$1::bigint IS NULL,$3) c ORDER BY c.deadline_id",
            &[&sequence, &after.map(|id| id.as_uuid()), &count],
        )
        .map_err(port)?;
    if rows.len() > usize::try_from(count).map_err(inconsistent)? {
        return Err(inconsistent("dispatch page exceeds limit"));
    }
    let mut previous = after.map(|id| id.as_uuid());
    rows.into_iter()
        .map(|row| {
            let id: uuid::Uuid = row.try_get(0).map_err(inconsistent)?;
            if previous.is_some_and(|old| id <= old) {
                return Err(inconsistent("dispatch candidates are not strictly ordered"));
            }
            previous = Some(id);
            Ok(Candidate {
                id: DeadlineId::from_uuid(id),
                case_id: CaseId::from_uuid(row.try_get(1).map_err(inconsistent)?),
            })
        })
        .collect()
}
