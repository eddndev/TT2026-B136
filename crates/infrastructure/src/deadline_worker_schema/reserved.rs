//! Reserved operations are usable only by their authentic completed worker job.
use super::{incomplete, port};
use application::{
    deadline_reevaluation::{TechnicalCause, TechnicalService},
    deadlines::{
        DeadlineAction, DeadlineActorSnapshot, DeadlineId, DeadlineReceiptVersion, DeadlineRevision,
    },
    ApplicationError,
};
use domain::{cases::CaseId, typed_participants::Uuid};
use postgres::Transaction;

/// The caller holds the audited mutation lock. Inspect technical or reserved
/// rows, including technical orphans that cannot appear in a jobs-first join.
pub(crate) fn validate(tx: &mut Transaction<'_>) -> Result<(), ApplicationError> {
    let mut after: Option<Uuid> = None;
    let mut revision = 0_i64;
    loop {
        let rows = tx
            .query(
                "SELECT r.deadline_id,r.case_id,r.revision,j.id AS job_id
            FROM case_deadline_revisions r
            LEFT JOIN deadline_reevaluation_jobs j ON j.operation_id=r.operation_id
            WHERE (r.action='reevaluate' OR r.recorded_by IS NULL OR j.id IS NOT NULL)
                AND ($1::uuid IS NULL OR (r.deadline_id,r.revision)>($1,$2))
            ORDER BY r.deadline_id,r.revision LIMIT 64",
                &[&after, &revision],
            )
            .map_err(port)?;
        for row in &rows {
            let id: Uuid = row.try_get("deadline_id").map_err(|_| incomplete())?;
            let case: Uuid = row.try_get("case_id").map_err(|_| incomplete())?;
            let number: i64 = row.try_get("revision").map_err(|_| incomplete())?;
            let job: Option<Uuid> = row.try_get("job_id").map_err(|_| incomplete())?;
            let job = job.ok_or_else(incomplete)?;
            let selected = DeadlineRevision::new(u32::try_from(number).map_err(|_| incomplete())?)
                .map_err(|_| incomplete())?;
            // The real reader verifies complete evidence, the predecessor and
            // shared provenance, including the matching immutable completion.
            let detail = crate::deadline_postgres::storage::detail(
                tx,
                CaseId::from_uuid(case),
                DeadlineId::from_uuid(id),
                Some(selected),
                &crate::RingSha256Hasher,
            )?;
            if detail.receipt.action != DeadlineAction::Reevaluate
                || detail.recorded_by
                    != (DeadlineActorSnapshot::Technical {
                        service: TechnicalService::DeadlineReevaluator,
                        policy_version: 1,
                    })
            {
                return Err(incomplete());
            }
            let DeadlineReceiptVersion::Tracked(receipt) = &detail.receipt.version else {
                return Err(incomplete());
            };
            let recorded_job = match receipt.cause {
                Some(TechnicalCause::SourceEvent { job_id, .. })
                | Some(TechnicalCause::LegacyBootstrap { job_id, .. }) => job_id,
                None => return Err(incomplete()),
            };
            if recorded_job != job {
                return Err(incomplete());
            }
            after = Some(id);
            revision = number;
        }
        if rows.len() < 64 {
            return Ok(());
        }
    }
}
