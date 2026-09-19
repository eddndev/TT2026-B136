use super::{event, inconsistent, port, timestamp, VerifiedDeadlineJob};
use application::{
    deadline_reevaluation::TechnicalCause,
    deadlines::{DeadlineId, DeadlineOperationId},
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher, typed_participants::Uuid};
use postgres::Transaction;

/// Authenticate a stable job and its exact source event without requiring that
/// today's deadline head still selects the dependency or that a cursor points
/// at the event. Existing jobs remain meaningful after human edits or retirement.
pub(crate) fn load_job(
    tx: &mut Transaction<'_>,
    id: Uuid,
    hasher: &dyn DocumentHasher,
) -> Result<VerifiedDeadlineJob, ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT j.id,j.operation_id,j.deadline_id,j.case_id,j.event_sequence,
        j.bootstrap_policy_version,j.created_at_seconds,j.created_at_nanoseconds,
        d.case_id AS deadline_case,c.id AS existing_case
        FROM deadline_reevaluation_jobs j
        LEFT JOIN case_deadlines d ON d.id=j.deadline_id
        LEFT JOIN cases c ON c.id=j.case_id WHERE j.id=$1",
            &[&id],
        )
        .map_err(port)?
        .ok_or_else(|| inconsistent("durable deadline job is absent"))?;
    let case = row.try_get::<_, Uuid>("case_id").map_err(inconsistent)?;
    if row.try_get::<_, Uuid>("id").map_err(inconsistent)? != id
        || row
            .try_get::<_, Option<Uuid>>("deadline_case")
            .map_err(inconsistent)?
            != Some(case)
        || row
            .try_get::<_, Option<Uuid>>("existing_case")
            .map_err(inconsistent)?
            != Some(case)
    {
        return Err(inconsistent("deadline job root or case differs"));
    }
    let sequence: Option<i64> = row.try_get("event_sequence").map_err(inconsistent)?;
    let policy: Option<i16> = row
        .try_get("bootstrap_policy_version")
        .map_err(inconsistent)?;
    let cause = match (sequence, policy) {
        (Some(sequence), None) if sequence > 0 => {
            let event = event::load(tx, sequence, hasher)?;
            if event
                .case_id
                .is_some_and(|event_case| event_case.as_uuid() != case)
            {
                return Err(inconsistent("job event belongs to another case"));
            }
            TechnicalCause::SourceEvent { job_id: id, event }
        }
        (None, Some(1)) => TechnicalCause::LegacyBootstrap {
            job_id: id,
            policy_version: 1,
        },
        _ => return Err(inconsistent("deadline job cause shape is invalid")),
    };
    Ok(VerifiedDeadlineJob {
        id,
        operation_id: DeadlineOperationId::from_uuid(
            row.try_get("operation_id").map_err(inconsistent)?,
        ),
        deadline_id: DeadlineId::from_uuid(row.try_get("deadline_id").map_err(inconsistent)?),
        case_id: CaseId::from_uuid(case),
        cause,
        created_at: timestamp(
            row.try_get("created_at_seconds").map_err(inconsistent)?,
            row.try_get("created_at_nanoseconds")
                .map_err(inconsistent)?,
        )?,
    })
}
