use super::{incomplete, port};
use application::ApplicationError;
use postgres::Client;

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    // Reuse exact source-revision, operation and scope verification under the lock.
    crate::deadline_source_event_schema::validate_inventory(&mut tx)?;
    let broken_cursor: bool = tx.query_one(
        "SELECT (SELECT count(*) FROM deadline_dispatch_cursor)<>1
        OR EXISTS(SELECT 1 FROM deadline_dispatch_cursor c
            LEFT JOIN deadline_source_events completed ON completed.sequence=c.completed_event_sequence
            LEFT JOIN deadline_source_events active ON active.sequence=c.active_event_sequence
            WHERE c.singleton IS DISTINCT FROM TRUE
                OR (c.completed_event_sequence IS NOT NULL AND (completed.sequence IS NULL OR c.completed_event_sequence<=0))
                OR num_nonnulls(c.active_event_sequence,c.after_deadline_id) NOT IN (0,2)
                OR (c.active_event_sequence IS NOT NULL AND (
                    active.sequence IS NULL OR c.active_event_sequence<=coalesce(c.completed_event_sequence,0)
                    OR c.active_event_sequence IS DISTINCT FROM (SELECT min(e.sequence)
                        FROM deadline_source_events e WHERE e.sequence>coalesce(c.completed_event_sequence,0))
                    OR NOT EXISTS(SELECT 1 FROM deadline_reevaluation_jobs j
                        WHERE j.event_sequence=c.active_event_sequence AND j.deadline_id=c.after_deadline_id)))
                OR (c.bootstrap_after_deadline_id IS NOT NULL AND NOT EXISTS(
                    SELECT 1 FROM deadline_reevaluation_jobs j WHERE j.event_sequence IS NULL
                        AND j.bootstrap_policy_version=1 AND j.deadline_id=c.bootstrap_after_deadline_id)))",
        &[],
    ).map_err(port)?.get(0);
    if broken_cursor {
        return Err(incomplete());
    }
    let broken_jobs: bool = tx.query_one(
        "SELECT EXISTS(SELECT 1 FROM deadline_reevaluation_jobs j
            LEFT JOIN case_deadlines d ON d.id=j.deadline_id AND d.case_id=j.case_id
            LEFT JOIN cases c ON c.id=j.case_id
            LEFT JOIN deadline_source_events e ON e.sequence=j.event_sequence
            CROSS JOIN deadline_dispatch_cursor position
            WHERE d.id IS NULL OR c.id IS NULL
                OR ((j.event_sequence IS NOT NULL AND j.event_sequence>0 AND j.bootstrap_policy_version IS NULL)
                    OR (j.event_sequence IS NULL AND j.bootstrap_policy_version=1)) IS NOT TRUE
                OR j.created_at_seconds NOT BETWEEN -62135596800 AND 253402300799
                OR j.created_at_nanoseconds NOT BETWEEN 0 AND 999999999
                OR (j.event_sequence IS NOT NULL AND (e.sequence IS NULL
                    OR (e.case_id IS NOT NULL AND e.case_id IS DISTINCT FROM j.case_id)
                    OR (j.event_sequence>coalesce(position.completed_event_sequence,0)
                        AND j.event_sequence IS DISTINCT FROM (SELECT min(next_event.sequence)
                            FROM deadline_source_events next_event
                            WHERE next_event.sequence>coalesce(position.completed_event_sequence,0)))))
                )",
        &[],
    ).map_err(port)?.get(0);
    if broken_jobs {
        return Err(incomplete());
    }
    // Current heads may no longer select a dispatched dependency, or may be retired.
    // Past scan coverage is enforced when advancing, not inferred from later heads.
    crate::deadline_worker_schema::validate_reserved_operations(&mut tx)?;
    tx.rollback().map_err(port)
}
