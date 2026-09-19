use super::{inconsistent, port, selection::Candidate};
use application::{deadlines::DeadlineOperationId, ApplicationError};
use postgres::Transaction;
use time::OffsetDateTime;
use uuid::Uuid;

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    candidate: Candidate,
    event: Option<u64>,
    at: OffsetDateTime,
) -> Result<bool, ApplicationError> {
    let sequence = event.map(i64::try_from).transpose().map_err(inconsistent)?;
    let policy = event.is_none().then_some(1_i16);
    let id = Uuid::new_v4();
    let operation = DeadlineOperationId::new().as_uuid();
    let conflict = if event.is_some() {
        "(event_sequence,deadline_id) WHERE event_sequence IS NOT NULL"
    } else {
        "(deadline_id,bootstrap_policy_version) WHERE event_sequence IS NULL"
    };
    let count = tx
        .execute(
            &format!(
                "INSERT INTO deadline_reevaluation_jobs(
        id,operation_id,deadline_id,case_id,event_sequence,bootstrap_policy_version,
        created_at_seconds,created_at_nanoseconds) VALUES($1,$2,$3,$4,$5,$6,$7,$8)
        ON CONFLICT {conflict} DO NOTHING"
            ),
            &[
                &id,
                &operation,
                &candidate.id.as_uuid(),
                &candidate.case_id.as_uuid(),
                &sequence,
                &policy,
                &at.unix_timestamp(),
                &i32::try_from(at.nanosecond()).map_err(inconsistent)?,
            ],
        )
        .map_err(port)?;
    if count > 1 {
        return Err(inconsistent("dispatch inserted more than one job"));
    }
    let row = tx
        .query_one(
            "SELECT id,operation_id,case_id,event_sequence,bootstrap_policy_version,
        created_at_seconds,created_at_nanoseconds FROM deadline_reevaluation_jobs
        WHERE deadline_id=$1 AND event_sequence IS NOT DISTINCT FROM $2::bigint
            AND bootstrap_policy_version IS NOT DISTINCT FROM $3::smallint",
            &[&candidate.id.as_uuid(), &sequence, &policy],
        )
        .map_err(port)?;
    let saved_id: Uuid = row.try_get(0).map_err(inconsistent)?;
    let saved_operation: Uuid = row.try_get(1).map_err(inconsistent)?;
    let saved_at = OffsetDateTime::from_unix_timestamp(row.try_get(5).map_err(inconsistent)?)
        .map_err(inconsistent)?
        .replace_nanosecond(
            u32::try_from(row.try_get::<_, i32>(6).map_err(inconsistent)?).map_err(inconsistent)?,
        )
        .map_err(inconsistent)?;
    if row.try_get::<_, Uuid>(2).map_err(inconsistent)? != candidate.case_id.as_uuid()
        || row.try_get::<_, Option<i64>>(3).map_err(inconsistent)? != sequence
        || row.try_get::<_, Option<i16>>(4).map_err(inconsistent)? != policy
        || !(1..=9999).contains(&saved_at.year())
        || (count == 1 && (saved_id != id || saved_operation != operation || saved_at != at))
    {
        return Err(inconsistent(
            "dispatch job differs from its immutable operation or cause",
        ));
    }
    Ok(count == 1)
}
