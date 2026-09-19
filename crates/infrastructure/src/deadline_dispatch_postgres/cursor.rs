use super::{inconsistent, port};
use application::{deadline_dispatch::*, deadlines::DeadlineId, ApplicationError};
use postgres::Transaction;
use uuid::Uuid;

pub(super) fn load(tx: &mut Transaction<'_>) -> Result<DeadlineDispatchProgress, ApplicationError> {
    let rows = tx
        .query(
            "SELECT singleton,completed_event_sequence,active_event_sequence,after_deadline_id,
            bootstrap_after_deadline_id FROM deadline_dispatch_cursor FOR UPDATE",
            &[],
        )
        .map_err(port)?;
    if rows.len() != 1 || !rows[0].try_get::<_, bool>(0).map_err(inconsistent)? {
        return Err(inconsistent(
            "dispatch singleton cursor is absent or duplicated",
        ));
    }
    let row = &rows[0];
    let progress = DeadlineDispatchProgress {
        event: DeadlineEventDispatchPosition {
            completed_sequence: sequence(row.try_get(1).map_err(inconsistent)?)?,
            active_sequence: sequence(row.try_get(2).map_err(inconsistent)?)?,
            after_deadline_id: row
                .try_get::<_, Option<Uuid>>(3)
                .map_err(inconsistent)?
                .map(DeadlineId::from_uuid),
        },
        bootstrap_after_deadline_id: row
            .try_get::<_, Option<Uuid>>(4)
            .map_err(inconsistent)?
            .map(DeadlineId::from_uuid),
    };
    if progress.event.active_sequence.is_some() != progress.event.after_deadline_id.is_some()
        || progress
            .event
            .active_sequence
            .is_some_and(|active| active <= progress.event.completed_sequence.unwrap_or(0))
    {
        return Err(inconsistent("dispatch event position is invalid"));
    }
    Ok(progress)
}

fn sequence(value: Option<i64>) -> Result<Option<u64>, ApplicationError> {
    value
        .map(|value| {
            if value <= 0 {
                return Err(inconsistent("dispatch sequence is not positive"));
            }
            u64::try_from(value).map_err(inconsistent)
        })
        .transpose()
}

pub(super) fn save(
    tx: &mut Transaction<'_>,
    value: DeadlineDispatchProgress,
) -> Result<(), ApplicationError> {
    let completed = value
        .event
        .completed_sequence
        .map(i64::try_from)
        .transpose()
        .map_err(inconsistent)?;
    let active = value
        .event
        .active_sequence
        .map(i64::try_from)
        .transpose()
        .map_err(inconsistent)?;
    let count = tx.execute("UPDATE deadline_dispatch_cursor SET completed_event_sequence=$1,
        active_event_sequence=$2,after_deadline_id=$3,bootstrap_after_deadline_id=$4 WHERE singleton",
        &[&completed,&active,&value.event.after_deadline_id.map(|id| id.as_uuid()),
            &value.bootstrap_after_deadline_id.map(|id| id.as_uuid())]).map_err(port)?;
    if count != 1 {
        return Err(inconsistent(
            "dispatch cursor update did not affect one row",
        ));
    }
    Ok(())
}
