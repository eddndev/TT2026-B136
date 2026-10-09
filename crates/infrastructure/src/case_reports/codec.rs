use super::codec_activity::{Activity, StoredKind};
use super::*;
use application::cases::CaseStatusFilter;
use domain::{
    case_administration::{CaseAdministrativeStatus, CaseRevision},
    cases::CaseId,
    identity::UserId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Filters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kind: Option<StoredKind>,
    from: String,
    before: String,
    status: String,
    litigator: Option<Uuid>,
}
impl Filters {
    fn write(value: &CaseReportFilters) -> Result<Self, ApplicationError> {
        Ok(Self {
            kind: (value.kind == CaseReportKind::LitigatorActivity).then_some(StoredKind::Activity),
            from: timestamp(value.period_from)?,
            before: timestamp(value.period_before)?,
            status: match value.status {
                CaseStatusFilter::All => "all",
                CaseStatusFilter::Active => "active",
                CaseStatusFilter::Closed => "closed",
            }
            .into(),
            litigator: value.litigator.map(UserId::as_uuid),
        })
    }
    fn read(self) -> Result<CaseReportFilters, ApplicationError> {
        Ok(CaseReportFilters {
            kind: if self.kind.is_some() {
                CaseReportKind::LitigatorActivity
            } else {
                CaseReportKind::CaseState
            },
            period_from: parse_time(&self.from)?,
            period_before: parse_time(&self.before)?,
            status: match self.status.as_str() {
                "all" => CaseStatusFilter::All,
                "active" => CaseStatusFilter::Active,
                "closed" => CaseStatusFilter::Closed,
                _ => return Err(inconsistent("unknown report status filter")),
            },
            litigator: self.litigator.map(UserId::from_uuid),
        })
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    operation_id: Uuid,
    filters: Filters,
}
pub(super) fn command(value: &CaseReportCommand) -> Result<Value, ApplicationError> {
    serde_json::to_value(Command {
        operation_id: value.operation_id.as_uuid(),
        filters: Filters::write(&value.filters)?,
    })
    .map_err(inconsistent)
}
pub(super) fn read_command(value: Value) -> Result<CaseReportCommand, ApplicationError> {
    let wire: Command = serde_json::from_value(value).map_err(inconsistent)?;
    Ok(CaseReportCommand {
        operation_id: CaseReportOperationId::from_uuid(wire.operation_id),
        filters: wire.filters.read()?,
    })
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Person {
    id: Uuid,
    email: String,
}
impl Person {
    fn write(value: &CaseReportLitigator) -> Self {
        Self {
            id: value.user_id.as_uuid(),
            email: value.email.clone(),
        }
    }
    fn read(self) -> CaseReportLitigator {
        CaseReportLitigator {
            user_id: UserId::from_uuid(self.id),
            email: self.email,
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    id: Uuid,
    title: String,
    reference: String,
    created_at: String,
    status: String,
    revision: Option<u32>,
    digest: Option<Sha256Digest>,
    assigned: Vec<Person>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Workload {
    person: Person,
    active: u64,
    closed: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    activity: Option<Activity>,
    version: u32,
    report_id: Uuid,
    principal: Principal,
    account_revision: u64,
    auth_generation: u64,
    scope: String,
    filters: Filters,
    checked_at: String,
    cases: Vec<Row>,
    workload: Vec<Workload>,
    digest: Sha256Digest,
}
pub(crate) fn snapshot(value: &CaseReportSnapshot) -> Result<Vec<u8>, ApplicationError> {
    let cases = value
        .cases
        .iter()
        .map(|row| {
            Ok(Row {
                id: row.case_id.as_uuid(),
                title: row.title.clone(),
                reference: row.reference.clone(),
                created_at: timestamp(row.created_at)?,
                status: row.status.as_str().into(),
                revision: row.administration_revision.map(CaseRevision::get),
                digest: row.administration_digest,
                assigned: row.assigned_litigators.iter().map(Person::write).collect(),
            })
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    let wire = Snapshot {
        activity: value.activity.as_ref().map(Activity::write),
        version: if value.filters.kind == CaseReportKind::CaseState {
            1
        } else {
            2
        },
        report_id: value.report_id.as_uuid(),
        principal: value.requester.principal.clone(),
        account_revision: value.requester.account_revision,
        auth_generation: value.requester.auth_generation,
        scope: scope_name(value.scope).into(),
        filters: Filters::write(&value.filters)?,
        checked_at: timestamp(value.checked_at)?,
        cases,
        workload: value
            .workload
            .iter()
            .map(|row| Workload {
                person: Person::write(&row.litigator),
                active: row.active_cases,
                closed: row.closed_cases,
            })
            .collect(),
        digest: value.digest,
    };
    let bytes = serde_json::to_vec(&wire).map_err(inconsistent)?;
    if bytes.len() > MAX_REPORT_SNAPSHOT_BYTES {
        return Err(CaseReportError::CapacityExceeded.into());
    }
    Ok(bytes)
}
pub(crate) fn read_snapshot(bytes: &[u8]) -> Result<CaseReportSnapshot, ApplicationError> {
    if bytes.len() > MAX_REPORT_SNAPSHOT_BYTES {
        return Err(CaseReportError::CapacityExceeded.into());
    }
    let wire: Snapshot = serde_json::from_slice(bytes).map_err(inconsistent)?;
    let expected = match (&wire.filters.kind, &wire.activity) {
        (None, None) => 1,
        (Some(StoredKind::Activity), Some(_)) => 2,
        _ => return Err(inconsistent("report capture kind and payload differ")),
    };
    if wire.version != expected
        || wire.cases.len() > MAX_REPORT_CASES
        || wire.workload.len() > MAX_REPORT_WORKLOAD
        || wire
            .cases
            .iter()
            .try_fold(0usize, |n, row| n.checked_add(row.assigned.len()))
            .is_none_or(|n| n > MAX_REPORT_ASSIGNMENTS)
    {
        return Err(inconsistent("report capture schema or capacity differs"));
    }
    let cases = wire
        .cases
        .into_iter()
        .map(|row| {
            Ok(CaseReportRow {
                case_id: CaseId::from_uuid(row.id),
                title: row.title,
                reference: row.reference,
                created_at: parse_time(&row.created_at)?,
                status: row
                    .status
                    .parse::<CaseAdministrativeStatus>()
                    .map_err(inconsistent)?,
                administration_revision: row
                    .revision
                    .map(CaseRevision::new)
                    .transpose()
                    .map_err(inconsistent)?,
                administration_digest: row.digest,
                assigned_litigators: row.assigned.into_iter().map(Person::read).collect(),
            })
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    Ok(CaseReportSnapshot {
        activity: wire.activity.map(Activity::read),
        report_id: CaseReportId::from_uuid(wire.report_id),
        requester: CaseReportRequester {
            principal: wire.principal,
            account_revision: wire.account_revision,
            auth_generation: wire.auth_generation,
        },
        scope: read_scope(&wire.scope)?,
        filters: wire.filters.read()?,
        checked_at: parse_time(&wire.checked_at)?,
        cases,
        workload: wire
            .workload
            .into_iter()
            .map(|row| CaseReportWorkload {
                litigator: row.person.read(),
                active_cases: row.active,
                closed_cases: row.closed,
            })
            .collect(),
        digest: wire.digest,
    })
}
pub(super) fn read_scope(value: &str) -> Result<CaseReportScope, ApplicationError> {
    match value {
        "office" => Ok(CaseReportScope::Office),
        "assigned_cases" => Ok(CaseReportScope::AssignedCases),
        _ => Err(inconsistent("unknown report scope")),
    }
}
pub(super) fn failure_name(value: CaseReportFailure) -> &'static str {
    match value {
        CaseReportFailure::TemporaryUnavailable => "temporary_unavailable",
        CaseReportFailure::RenderUnavailable => "render_unavailable",
        CaseReportFailure::RenderFailed => "render_failed",
        CaseReportFailure::CapacityExceeded => "capacity_exceeded",
        CaseReportFailure::InvalidStoredCapture => "invalid_stored_capture",
        CaseReportFailure::AccessRevoked => "access_revoked",
    }
}
pub(super) fn read_failure(value: &str) -> Result<CaseReportFailure, ApplicationError> {
    match value {
        "temporary_unavailable" => Ok(CaseReportFailure::TemporaryUnavailable),
        "render_unavailable" => Ok(CaseReportFailure::RenderUnavailable),
        "render_failed" => Ok(CaseReportFailure::RenderFailed),
        "capacity_exceeded" => Ok(CaseReportFailure::CapacityExceeded),
        "invalid_stored_capture" => Ok(CaseReportFailure::InvalidStoredCapture),
        "access_revoked" => Ok(CaseReportFailure::AccessRevoked),
        _ => Err(inconsistent("unknown report failure")),
    }
}
