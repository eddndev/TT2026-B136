//! One mutation in one transaction: the exact replay of an operation is
//! found first; then the author and the expected revision are rechecked
//! under the locks before any row is written.

use super::{
    errors::{inconsistent, port, refusal},
    read::staff_scope,
    receipt,
    write_rows::{self, RequestRow, SignatureRow},
    write_steps::{self, contract_of, Event},
    CONTRACT_LOCK,
};
use application::{
    contracts::{
        ContractCommand, ContractCommit, ContractRead, ContractReceipt, ContractWorkflowError,
        ContractWrite, EventKind,
    },
    document_signing::SigningWorkflowError,
    identity::Principal,
    ApplicationError,
};
use domain::clock::OffsetDateTime;
use postgres::{Client, Transaction};
use uuid::Uuid;

pub(super) fn commit(
    client: &mut Client,
    command: ContractCommand,
) -> Result<ContractCommit, ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    tx.execute("SELECT pg_advisory_xact_lock($1)", &[&CONTRACT_LOCK])
        .map_err(port)?;
    if let Some(existing) = receipt::load(&mut tx, command.operation)? {
        if !same(&existing, &command) {
            return Err(SigningWorkflowError::Conflict.into());
        }
        tx.commit().map_err(port)?;
        return Ok(ContractCommit::Existing(existing));
    }
    if let Some(actor) = &command.actor {
        current_actor(&mut tx, actor)?;
    }
    let contract = contract_of(&command);
    let previous: i64 = tx
        .query_one(
            "SELECT coalesce(max(revision),0) FROM contract_events WHERE contract_id=$1",
            &[&contract],
        )
        .map_err(port)?
        .get(0);
    let expiring = match &command.write {
        ContractWrite::Abandon { expire_first, .. }
        | ContractWrite::Request { expire_first, .. } => expire_first.as_ref(),
        _ => None,
    };
    let first = expiring.map_or(command.revision, |step| step.revision);
    if previous + 1 != i64::from(first)
        || expiring.is_some_and(|step| step.revision + 1 != command.revision)
    {
        return Err(ContractWorkflowError::RevisionConflict.into());
    }
    if let Some(step) = expiring {
        write_steps::expire(&mut tx, contract, step, command.at)?;
    }
    apply(&mut tx, &command)?;
    tx.commit().map_err(refusal)?;
    receipt::load(client, command.operation)?
        .map(ContractCommit::Applied)
        .ok_or_else(inconsistent)
}

/// The author must still be the same active account with the same role.
fn current_actor(tx: &mut Transaction<'_>, actor: &Principal) -> Result<(), ApplicationError> {
    let current = tx
        .query_opt(
            "SELECT 1 FROM users WHERE id=$1 AND active AND email=$2 AND role=$3",
            &[&actor.id.as_uuid(), &actor.email, &actor.role.as_str()],
        )
        .map_err(port)?;
    if current.is_none() {
        return Err(ApplicationError::InvalidSession);
    }
    Ok(())
}

fn apply(tx: &mut Transaction<'_>, command: &ContractCommand) -> Result<(), ApplicationError> {
    let actor = command.actor.as_ref();
    let base = |kind, request, reason| Event {
        contract: contract_of(command),
        revision: command.revision,
        kind,
        request,
        actor,
        reason,
        at: command.at,
        operation: command.operation,
    };
    match &command.write {
        ContractWrite::Create { .. } => {
            write_steps::insert_root(tx, command, actor.ok_or_else(inconsistent)?)
        }
        ContractWrite::Abandon { reason, .. } => write_steps::event(
            tx,
            &base(EventKind::Abandoned, None, Some(reason.as_str())),
            None,
        ),
        ContractWrite::Request {
            contract,
            request,
            client_binding,
            firm_binding,
            ceremony_reference,
            subject,
            ..
        } => {
            let event = base(EventKind::Requested, Some(request.fields().request), None);
            let requester = actor.ok_or_else(inconsistent)?;
            let audit_first = crate::audit_postgres::append_transaction(
                tx,
                &requester.email,
                &EventKind::Requested.action(),
                &format!(
                    "contract-request:{}:revision:{}",
                    request.fields().request,
                    command.revision
                ),
                command.at,
            )?;
            write_rows::request(
                tx,
                &RequestRow {
                    contract: *contract,
                    request,
                    client_binding: *client_binding,
                    firm_binding: *firm_binding,
                    ceremony_reference,
                    subject: *subject,
                    requester,
                    at: command.at,
                },
            )?;
            write_steps::insert_event(tx, &event, audit_first.event.sequence)?;
            write_steps::notices(tx, &event, &command.notices)
        }
        ContractWrite::Sign {
            request,
            statement,
            signature,
            check,
            timestamp,
            completion,
            ..
        } => {
            let kind = if completion.is_some() {
                EventKind::Completed
            } else {
                EventKind::ClientSigned
            };
            let event = base(kind, Some(*request), None);
            write_steps::event(tx, &event, None)?;
            write_rows::signature(
                tx,
                &SignatureRow {
                    request: *request,
                    operation: command.operation,
                    signer: actor.ok_or_else(inconsistent)?,
                    statement,
                    signature,
                    check,
                    timestamp: timestamp.as_ref(),
                    at: command.at,
                },
            )?;
            if let Some(completion) = completion {
                write_rows::completion(tx, *request, completion)?;
            }
            write_steps::notices(tx, &event, &command.notices)
        }
        ContractWrite::Close {
            request,
            kind,
            reason,
            ..
        } => {
            let event = base(*kind, Some(*request), reason.as_deref());
            write_steps::event(tx, &event, None)?;
            write_steps::notices(tx, &event, &command.notices)
        }
    }
}

/// Whether a stored receipt is the exact result of `command`.
fn same(existing: &ContractReceipt, command: &ContractCommand) -> bool {
    let event = &existing.event;
    let author = event.actor == command.actor.as_ref().map(|actor| actor.id);
    author
        && match &command.write {
            ContractWrite::Create { contract, case, .. } => {
                event.kind == EventKind::Created
                    && existing.contract == *contract
                    && existing.case == *case
            }
            ContractWrite::Abandon {
                contract, reason, ..
            } => {
                event.kind == EventKind::Abandoned
                    && existing.contract == *contract
                    && event.reason.as_deref() == Some(reason.as_str())
            }
            ContractWrite::Request { request, .. } => {
                event.kind == EventKind::Requested
                    && event.request == Some(request.fields().request)
            }
            ContractWrite::Sign {
                request,
                statement,
                signature,
                ..
            } => existing.signature.as_ref().is_some_and(|stored| {
                event.request == Some(*request)
                    && stored.statement == *statement
                    && stored.signature == *signature
            }),
            ContractWrite::Close {
                request,
                kind,
                reason,
                ..
            } => event.kind == *kind && event.request == Some(*request) && event.reason == *reason,
        }
}

/// Appends the audit of a content read or an evidence export by a party or
/// by staff with scope on the case.
pub(super) fn record_read(
    client: &mut Client,
    principal: &Principal,
    request: Uuid,
    read: ContractRead,
    at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    current_actor(&mut tx, principal)?;
    let row = tx
        .query_opt(
            "SELECT case_id,client_id,firm_signer_id FROM contract_signature_requests WHERE request_id=$1",
            &[&request],
        )
        .map_err(port)?
        .ok_or(ContractWorkflowError::NotFound)?;
    let party = [row.get::<_, Uuid>(1), row.get(2)].contains(&principal.id.as_uuid());
    if !party && !staff_scope(&mut tx, principal, row.get(0))? {
        return Err(ContractWorkflowError::NotFound.into());
    }
    let (action, resource) = match read {
        ContractRead::Content => (
            "contract.content_read",
            format!("contract-request:{request}:content"),
        ),
        ContractRead::PartialEvidence => (
            "contract.evidence_exported",
            format!("contract-request:{request}:evidence:partial"),
        ),
        ContractRead::FinalEvidence => (
            "contract.evidence_exported",
            format!("contract-request:{request}:evidence:final"),
        ),
    };
    crate::audit_postgres::append_transaction(&mut tx, &principal.email, action, &resource, at)?;
    tx.commit().map_err(port)
}
