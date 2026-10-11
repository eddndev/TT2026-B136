use super::*;
use postgres::Row;
use serde_json::{json, Value};

pub(super) struct Delivery {
    pub report_id: CaseReportId,
    pub delivery_id: Uuid,
    pub state: CaseReportEmailState,
    pub generation: u64,
    pub lease_id: Option<Uuid>,
    pub lease_until: Option<OffsetDateTime>,
    pub provider_deadline: Option<OffsetDateTime>,
    pub retry_at: Option<OffsetDateTime>,
    pub message: Option<CaseReportEmailMessage>,
    pub updated_at: OffsetDateTime,
}

impl Delivery {
    pub fn live(&self, delivery: Uuid, generation: u64, lease: Uuid, now: OffsetDateTime) -> bool {
        self.state == CaseReportEmailState::Pending
            && self.delivery_id == delivery
            && self.generation == generation
            && self.lease_id == Some(lease)
            && self.lease_until.is_some_and(|until| until > now)
            && self.provider_deadline.is_some_and(|until| until > now)
    }

    pub fn due(&self, now: OffsetDateTime) -> bool {
        self.provider_deadline.is_none_or(|until| until > now)
            && self.lease_until.is_none_or(|until| until <= now)
            && match self.state {
                CaseReportEmailState::Pending => true,
                CaseReportEmailState::Uncertain => self.retry_at.is_some_and(|at| at <= now),
                _ => false,
            }
    }
}

pub(super) fn load(
    tx: &mut Transaction<'_>,
    id: CaseReportId,
) -> Result<Option<Delivery>, ApplicationError> {
    tx.query_opt(
        "SELECT * FROM case_report_email_deliveries WHERE report_id=$1 FOR UPDATE",
        &[&id.as_uuid()],
    )
    .map_err(port)?
    .map(|row| decode(&row))
    .transpose()
}

fn decode(row: &Row) -> Result<Delivery, ApplicationError> {
    let delivery = Delivery {
        report_id: CaseReportId::from_uuid(row.get("report_id")),
        delivery_id: row.get("delivery_id"),
        state: state(row.get("state"))?,
        generation: u64::try_from(row.get::<_, i64>("generation")).map_err(inconsistent)?,
        lease_id: row.get("lease_id"),
        lease_until: optional_time(row, "lease_until")?,
        provider_deadline: optional_time(row, "provider_deadline")?,
        retry_at: optional_time(row, "retry_at")?,
        message: row
            .get::<_, Option<Value>>("message")
            .map(read_message)
            .transpose()?,
        updated_at: parse_time(row.get("updated_at"))?,
    };
    let created_at = parse_time(row.get("created_at"))?;
    if delivery.report_id.as_uuid().is_nil()
        || delivery.delivery_id.is_nil()
        || delivery.lease_id.is_some_and(|id| id.is_nil())
        || created_at > delivery.updated_at
        || delivery.lease_id.is_some() != delivery.lease_until.is_some()
        || delivery.message.is_some() != delivery.provider_deadline.is_some()
        || (delivery.generation == 0) != delivery.message.is_none()
        || delivery.lease_until.is_some_and(|until| {
            delivery.state != CaseReportEmailState::Pending
                || until <= delivery.updated_at
                || delivery
                    .provider_deadline
                    .is_none_or(|deadline| until > deadline)
        })
        || delivery.retry_at.is_some_and(|retry| {
            delivery.state != CaseReportEmailState::Uncertain
                || retry <= delivery.updated_at
                || delivery
                    .provider_deadline
                    .is_none_or(|deadline| retry >= deadline)
        })
    {
        return Err(inconsistent("stored report email fields disagree"));
    }
    Ok(delivery)
}

fn optional_time(row: &Row, column: &str) -> Result<Option<OffsetDateTime>, ApplicationError> {
    row.get::<_, Option<&str>>(column)
        .map(parse_time)
        .transpose()
}

pub(super) fn state(value: &str) -> Result<CaseReportEmailState, ApplicationError> {
    match value {
        "pending" => Ok(CaseReportEmailState::Pending),
        "accepted" => Ok(CaseReportEmailState::Accepted),
        "rejected" => Ok(CaseReportEmailState::Rejected),
        "uncertain" => Ok(CaseReportEmailState::Uncertain),
        "access_revoked" => Ok(CaseReportEmailState::AccessRevoked),
        _ => Err(inconsistent("unknown report email state")),
    }
}

pub(super) fn state_name(value: CaseReportEmailState) -> &'static str {
    match value {
        CaseReportEmailState::Pending => "pending",
        CaseReportEmailState::Accepted => "accepted",
        CaseReportEmailState::Rejected => "rejected",
        CaseReportEmailState::Uncertain => "uncertain",
        CaseReportEmailState::AccessRevoked => "access_revoked",
    }
}

pub(super) fn message_json(message: &CaseReportEmailMessage) -> Value {
    json!({"from":message.from, "to":message.to, "subject":message.subject, "text":message.text})
}

fn read_message(value: Value) -> Result<CaseReportEmailMessage, ApplicationError> {
    let fields = value
        .as_object()
        .filter(|fields| fields.len() == 4)
        .ok_or_else(|| inconsistent("stored report email envelope is invalid"))?;
    let field = |key: &str| -> Result<String, ApplicationError> {
        fields
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| inconsistent("stored report email field is invalid"))
    };
    let message = CaseReportEmailMessage {
        from: field("from")?,
        to: field("to")?,
        subject: field("subject")?,
        text: field("text")?,
    };
    if !valid_message(&message) {
        return Err(inconsistent(
            "stored report email envelope exceeds its limits",
        ));
    }
    Ok(message)
}

pub(super) fn valid_message(message: &CaseReportEmailMessage) -> bool {
    let address = |value: &str| {
        !value.is_empty()
            && value.len() <= 254
            && value.contains('@')
            && !value.chars().any(|c| c.is_control() || c.is_whitespace())
    };
    address(&message.from)
        && address(&message.to)
        && !message.subject.is_empty()
        && message.subject.len() <= 200
        && !message.subject.chars().any(char::is_control)
        && !message.text.is_empty()
        && message.text.len() <= 4096
        && !message
            .text
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        && message_json(message).to_string().len() <= 16_384
}

pub(super) fn ready_job(
    tx: &mut Transaction<'_>,
    id: CaseReportId,
    hasher: &dyn DocumentHasher,
) -> Result<storage::Job, ApplicationError> {
    let job = storage::get(tx, id, hasher)?;
    if !matches!(job.detail.state, CaseReportState::Ready { .. })
        || !job
            .detail
            .estimate
            .is_some_and(CaseReportEstimate::requires_ready_email)
        || !job
            .detail
            .notice
            .as_ref()
            .is_some_and(|notice| notice.kind == CaseReportNoticeKind::Ready)
    {
        return Err(inconsistent(
            "report email requires an eligible available report",
        ));
    }
    Ok(job)
}

pub(super) fn authorize(
    tx: &mut Transaction<'_>,
    job: &storage::Job,
    now: OffsetDateTime,
) -> Result<bool, ApplicationError> {
    match access::original(tx, job) {
        Ok(()) => Ok(true),
        Err(ApplicationError::CaseReport(CaseReportError::AccessRevoked)) => {
            tx.execute(
                "UPDATE case_report_email_deliveries SET state='access_revoked',
                lease_id=NULL,lease_until=NULL,retry_at=NULL,updated_at=$2 WHERE report_id=$1",
                &[&job.detail.id.as_uuid(), &timestamp(now)?],
            )
            .map_err(port)?;
            audit(tx, &job.detail, "case_report.email_revoke", now)?;
            Ok(false)
        }
        Err(error) => Err(error),
    }
}
