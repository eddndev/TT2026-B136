use super::*;
use postgres::Row;

pub(super) struct Job {
    pub detail: CaseReportDetail,
    pub lease: Option<CaseReportLease>,
    pub generation: u64,
    pub attempts: i32,
    pub case_ids: Vec<Uuid>,
}
pub(super) fn get(
    tx: &mut Transaction<'_>,
    id: CaseReportId,
    hasher: &dyn DocumentHasher,
) -> Result<Job, ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT * FROM case_report_jobs WHERE id=$1 FOR UPDATE",
            &[&id.as_uuid()],
        )
        .map_err(port)?
        .ok_or(CaseReportError::NotFound)?;
    decode(tx, &row, hasher)
}
pub(super) fn decode(
    tx: &mut Transaction<'_>,
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<Job, ApplicationError> {
    let id = CaseReportId::from_uuid(row.get("id"));
    let requester = CaseReportRequester {
        principal: serde_json::from_value(row.get("principal")).map_err(inconsistent)?,
        account_revision: u64::try_from(row.get::<_, i64>("account_revision"))
            .map_err(inconsistent)?,
        auth_generation: u64::try_from(row.get::<_, i64>("auth_generation"))
            .map_err(inconsistent)?,
    };
    let scope = codec::read_scope(row.get("scope"))?;
    let command = codec::read_command(row.get("command"))?;
    let request_digest = digest(row.get::<_, &[u8]>("request_digest"))?;
    if row.get::<_, Uuid>("requester_id") != requester.principal.id.as_uuid()
        || row.get::<_, Uuid>("operation_id") != command.operation_id.as_uuid()
        || case_report_request_digest(hasher, &requester.principal, scope, &command)?
            != request_digest
    {
        return Err(inconsistent(
            "report request capture differs from its identity or digest",
        ));
    }
    let state = match row.get::<_, &str>("state") {
        "queued" => CaseReportState::Queued,
        "capturing" => CaseReportState::Processing(CaseReportPhase::Capturing),
        "rendering" => CaseReportState::Processing(CaseReportPhase::Rendering),
        "retry_capturing" | "retry_rendering" => CaseReportState::RetryWaiting {
            phase: if row.get::<_, &str>("state") == "retry_capturing" {
                CaseReportPhase::Capturing
            } else {
                CaseReportPhase::Rendering
            },
            retry_at: parse_time(
                row.get::<_, Option<&str>>("retry_at")
                    .ok_or_else(|| inconsistent("retry time missing"))?,
            )?,
        },
        "failed" => CaseReportState::Failed(codec::read_failure(
            row.get::<_, Option<&str>>("failure")
                .ok_or_else(|| inconsistent("report failure missing"))?,
        )?),
        "revoked" => CaseReportState::AccessRevoked,
        "ready" => ready(tx, id)?,
        _ => return Err(inconsistent("unknown report state")),
    };
    let notice = tx
        .query_opt(
            "SELECT kind,created_at,read_at FROM case_report_notices WHERE report_id=$1",
            &[&id.as_uuid()],
        )
        .map_err(port)?
        .map(|row| {
            Ok(CaseReportNotice {
                kind: match row.get::<_, &str>("kind") {
                    "ready" => CaseReportNoticeKind::Ready,
                    "failed" => CaseReportNoticeKind::Failed,
                    _ => return Err(inconsistent("unknown report notice")),
                },
                created_at: parse_time(row.get("created_at"))?,
                read_at: row
                    .get::<_, Option<&str>>("read_at")
                    .map(parse_time)
                    .transpose()?,
            })
        })
        .transpose()?;
    let generation = u64::try_from(row.get::<_, i64>("lease_generation")).map_err(inconsistent)?;
    let lease = match (
        row.get::<_, Option<Uuid>>("lease_attempt"),
        row.get::<_, Option<Uuid>>("lease_token"),
        row.get::<_, Option<&str>>("lease_expires_at"),
    ) {
        (Some(attempt), Some(token), Some(expires)) => Some(CaseReportLease {
            report_id: id,
            attempt_id: CaseReportAttemptId::from_uuid(attempt),
            token: CaseReportLeaseToken::from_uuid(token),
            generation,
            expires_at: parse_time(expires)?,
        }),
        (None, None, None) => None,
        _ => return Err(inconsistent("report lease fields disagree")),
    };
    let requested_at = parse_time(row.get("requested_at"))?;
    let updated_at = parse_time(row.get("updated_at"))?;
    let case_ids: Vec<Uuid> = row.get("case_ids");
    if requested_at > updated_at
        || case_ids.len() > MAX_REPORT_CASES
        || case_ids.windows(2).any(|w| w[0] >= w[1])
    {
        return Err(inconsistent("report clock or captured case index differs"));
    }
    Ok(Job {
        detail: CaseReportDetail {
            id,
            requester,
            scope,
            command,
            request_digest,
            requested_at,
            updated_at,
            state,
            notice,
        },
        lease,
        generation,
        attempts: row.get("attempts"),
        case_ids,
    })
}
fn ready(tx: &mut Transaction<'_>, id: CaseReportId) -> Result<CaseReportState, ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT digest,checked_at FROM case_report_snapshots WHERE report_id=$1",
            &[&id.as_uuid()],
        )
        .map_err(port)?
        .ok_or_else(|| inconsistent("ready report has no capture"))?;
    let rows = tx.query("SELECT format,bytes,digest FROM case_report_artifacts WHERE report_id=$1 ORDER BY format LIMIT 3", &[&id.as_uuid()]).map_err(port)?;
    if rows.len() != 2 {
        return Err(inconsistent("ready report requires both artifacts"));
    }
    let artifacts = rows
        .iter()
        .map(|row| {
            Ok(CaseReportArtifactMetadata {
                format: format_parse(row.get("format"))?,
                bytes: u64::try_from(row.get::<_, i64>("bytes")).map_err(inconsistent)?,
                digest: digest(row.get::<_, &[u8]>("digest"))?,
            })
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    Ok(CaseReportState::Ready {
        snapshot_digest: digest(row.get::<_, &[u8]>("digest"))?,
        checked_at: parse_time(row.get("checked_at"))?,
        artifacts,
    })
}
pub(super) fn fence(
    job: &Job,
    lease: &CaseReportLease,
    now: OffsetDateTime,
) -> Result<(), ApplicationError> {
    if job.lease.as_ref() != Some(lease)
        || lease.expires_at <= now
        || !matches!(job.detail.state, CaseReportState::Processing(_))
    {
        return Err(CaseReportError::LeaseLost.into());
    }
    if now < job.detail.updated_at {
        return Err(inconsistent("report operation predates current state"));
    }
    Ok(())
}
