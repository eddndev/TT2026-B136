//! The rows of each mutation, in the order the storage triggers require:
//! the audit row first, the root before `created`, a request before its
//! `requested` event, each signature after its event and the completion
//! after the second signature, then the notices of the event.

use super::errors::{port, refusal};
use application::{
    contracts::{
        ContractCommand, ContractNotice, ContractWrite, EventKind, ExpiryStep, EXPIRY_ACTOR,
    },
    identity::Principal,
    ApplicationError,
};
use domain::clock::OffsetDateTime;
use postgres::Transaction;
use uuid::Uuid;

pub(super) struct Event<'a> {
    pub(super) contract: Uuid,
    pub(super) revision: u32,
    pub(super) kind: EventKind,
    pub(super) request: Option<Uuid>,
    pub(super) actor: Option<&'a Principal>,
    pub(super) reason: Option<&'a str>,
    pub(super) at: OffsetDateTime,
    pub(super) operation: Uuid,
}

impl Event<'_> {
    fn label(&self) -> &str {
        self.actor
            .map_or(EXPIRY_ACTOR, |actor| actor.email.as_str())
    }

    fn resource(&self, document: Option<Uuid>) -> String {
        match (self.kind, self.request, document) {
            (EventKind::Created, _, Some(document)) => {
                format!("contract:{}:document:{document}", self.contract)
            }
            (EventKind::Abandoned, _, _) => {
                format!("contract:{}:revision:{}", self.contract, self.revision)
            }
            (_, Some(request), _) => {
                format!("contract-request:{request}:revision:{}", self.revision)
            }
            _ => String::new(),
        }
    }
}

/// The audit row and the event, in that order.
pub(super) fn event(
    tx: &mut Transaction<'_>,
    value: &Event<'_>,
    document: Option<Uuid>,
) -> Result<(), ApplicationError> {
    let audit = crate::audit_postgres::append_transaction(
        tx,
        value.label(),
        &value.kind.action(),
        &value.resource(document),
        value.at,
    )?;
    insert_event(tx, value, audit.event.sequence)
}

/// The event row naming the audit row `sequence` written before it.
pub(super) fn insert_event(
    tx: &mut Transaction<'_>,
    value: &Event<'_>,
    sequence: u64,
) -> Result<(), ApplicationError> {
    let sequence = i64::try_from(sequence).map_err(|_| super::errors::inconsistent())?;
    tx.execute(
        "INSERT INTO contract_events(contract_id,revision,kind,request_id,actor_id,actor_label,
        reason,at_seconds,at_nanoseconds,operation_id,audit_sequence)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        &[
            &value.contract,
            &i64::from(value.revision),
            &value.kind.as_str(),
            &value.request,
            &value.actor.map(|actor| actor.id.as_uuid()),
            &value.label(),
            &value.reason,
            &value.at.unix_timestamp(),
            &nanoseconds(value.at),
            &value.operation,
            &sequence,
        ],
    )
    .map_err(refusal)?;
    Ok(())
}

pub(super) fn nanoseconds(at: OffsetDateTime) -> i32 {
    i32::try_from(at.nanosecond()).unwrap_or(0)
}

/// One pending notice per recipient of an event.
pub(super) fn notices(
    tx: &mut Transaction<'_>,
    value: &Event<'_>,
    notices: &[ContractNotice],
) -> Result<(), ApplicationError> {
    for notice in notices {
        tx.execute(
            "INSERT INTO contract_notifications(id,request_id,contract_id,event_revision,recipient_id,
            kind,message_from,message_to,subject,message_text,created_at_seconds,delivery_state,
            delivery_generation)
            SELECT $1,$2,$3,$4,$5,$6,$7,u.email,$8,$9,$10,'pending',0 FROM users u WHERE u.id=$5",
            &[
                &Uuid::new_v4(),
                &value.request,
                &value.contract,
                &i64::from(value.revision),
                &notice.recipient.as_uuid(),
                &value.kind.as_str(),
                &notice.from,
                &notice.subject,
                &notice.text,
                &value.at.unix_timestamp(),
            ],
        )
        .map_err(refusal)?;
    }
    Ok(())
}

/// The `expired` event of an exhausted request, written before the event
/// that follows it.
pub(super) fn expire(
    tx: &mut Transaction<'_>,
    contract: Uuid,
    step: &ExpiryStep,
    at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    let value = Event {
        contract,
        revision: step.revision,
        kind: EventKind::Expired,
        request: Some(step.request),
        actor: None,
        reason: None,
        at,
        operation: step.operation,
    };
    event(tx, &value, None)?;
    notices(tx, &value, &step.notices)
}

/// The contract a command writes to.
pub(super) fn contract_of(command: &ContractCommand) -> Uuid {
    match &command.write {
        ContractWrite::Create { contract, .. }
        | ContractWrite::Abandon { contract, .. }
        | ContractWrite::Request { contract, .. }
        | ContractWrite::Sign { contract, .. }
        | ContractWrite::Close { contract, .. } => *contract,
    }
}

pub(super) fn insert_root(
    tx: &mut Transaction<'_>,
    command: &ContractCommand,
    actor: &Principal,
) -> Result<(), ApplicationError> {
    let ContractWrite::Create {
        contract,
        case,
        document,
        title,
    } = &command.write
    else {
        return Err(super::errors::inconsistent());
    };
    let series = tx
        .query_opt(
            "SELECT 1 FROM document_series WHERE id=$1 AND case_id=$2",
            &[&document.as_uuid(), &case.as_uuid()],
        )
        .map_err(port)?;
    if series.is_none() {
        return Err(ApplicationError::DocumentNotFound(document.to_string()));
    }
    let value = Event {
        contract: *contract,
        revision: 1,
        kind: EventKind::Created,
        request: None,
        actor: Some(actor),
        reason: None,
        at: command.at,
        operation: command.operation,
    };
    let audit = crate::audit_postgres::append_transaction(
        tx,
        value.label(),
        &value.kind.action(),
        &value.resource(Some(document.as_uuid())),
        value.at,
    )?;
    tx.execute(
        "INSERT INTO contracts(id,case_id,document_id,title,created_by,created_by_email,
        created_at_seconds,created_at_nanoseconds) VALUES($1,$2,$3,$4,$5,$6,$7,$8)",
        &[
            contract,
            &case.as_uuid(),
            &document.as_uuid(),
            title,
            &actor.id.as_uuid(),
            &actor.email,
            &command.at.unix_timestamp(),
            &nanoseconds(command.at),
        ],
    )
    .map_err(refusal)?;
    insert_event(tx, &value, audit.event.sequence)
}
