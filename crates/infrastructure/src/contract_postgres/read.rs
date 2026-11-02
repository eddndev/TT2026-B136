//! One whole stored contract with the facts its readers need. A caller with
//! neither staff scope on the case nor a designation in one of its requests
//! receives nothing; a party without staff scope receives only its own
//! requests and no actor or reason of any other request's events.

use super::{
    decode::{self, get},
    errors::{inconsistent, port},
};
use application::{
    contracts::{ContractSnapshot, RequestRecord},
    documents::DocumentRecord,
    identity::Principal,
    ApplicationError,
};
use domain::{document_signing::contract::ContractRequest, identity::Role};
use postgres::{GenericClient, Row};
use std::str::FromStr;
use uuid::Uuid;

const REQUESTS: &str = "SELECT r.*,cu.role AS client_role,fu.role AS firm_role
    FROM contract_signature_requests r
    JOIN users cu ON cu.id=r.client_id JOIN users fu ON fu.id=r.firm_signer_id
    WHERE r.contract_id=$1 ORDER BY r.created_at_seconds,r.created_at_nanoseconds,r.request_id";

const SIGNATURES: &str = "SELECT s.*,a.root_der,t.crl_der FROM contract_party_signatures s
    JOIN participant_credential_trust_revisions t
        ON t.deployment_id=s.deployment_id AND t.revision=s.trust_revision
    JOIN participant_credential_authority a ON a.deployment_id=s.deployment_id
    WHERE s.request_id=$1 ORDER BY s.\"position\"";

/// Whether `principal` is an active staff account with scope on `case`:
/// an Owner, or a Litigator or Paralegal member.
pub(super) fn staff_scope<C: GenericClient>(
    client: &mut C,
    principal: &Principal,
    case: Uuid,
) -> Result<bool, ApplicationError> {
    client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM users u WHERE u.id=$1 AND u.active AND u.email=$3
                AND u.role=$4 AND (u.role='owner' OR (u.role IN ('litigator','paralegal')
                AND EXISTS(SELECT 1 FROM case_memberships m WHERE m.case_id=$2 AND m.user_id=u.id))))",
            &[
                &principal.id.as_uuid(),
                &case,
                &principal.email,
                &principal.role.as_str(),
            ],
        )
        .map(|row| row.get(0))
        .map_err(port)
}

pub(super) fn contract_of<C: GenericClient>(
    client: &mut C,
    request: Uuid,
) -> Result<Option<Uuid>, ApplicationError> {
    client
        .query_opt(
            "SELECT contract_id FROM contract_signature_requests WHERE request_id=$1",
            &[&request],
        )
        .map(|row| row.map(|row| row.get(0)))
        .map_err(port)
}

fn request<C: GenericClient>(client: &mut C, row: &Row) -> Result<RequestRecord, ApplicationError> {
    let id: Uuid = get(row, "request_id")?;
    let statement: Vec<u8> = get(row, "statement")?;
    let request = ContractRequest::from_canonical_bytes(&statement).map_err(|_| inconsistent())?;
    let binding = |client: &mut C, column: &str| -> Result<_, ApplicationError> {
        crate::signing_binding_postgres::load(client, get(row, column)?)?.ok_or_else(inconsistent)
    };
    let client_binding = binding(client, "client_binding_id")?;
    let firm_binding = binding(client, "firm_binding_id")?;
    let role = |column: &str| -> Result<Role, ApplicationError> {
        Role::from_str(get::<&str>(row, column)?).map_err(|_| inconsistent())
    };
    let signatures = client
        .query(SIGNATURES, &[&id])
        .map_err(port)?
        .iter()
        .map(decode::signature)
        .collect::<Result<Vec<_>, _>>()?;
    let completion = client
        .query_opt(
            "SELECT * FROM contract_completions WHERE request_id=$1",
            &[&id],
        )
        .map_err(port)?
        .as_ref()
        .map(decode::completion)
        .transpose()?;
    let subject: Option<Uuid> = get(row, "client_subject_id")?;
    let subject_revision: Option<i64> = get(row, "client_subject_revision")?;
    let subject = match (subject, subject_revision) {
        (Some(id), Some(revision)) => {
            Some((id, u64::try_from(revision).map_err(|_| inconsistent())?))
        }
        _ => None,
    };
    if request.fields().request != id {
        return Err(inconsistent());
    }
    Ok(RequestRecord {
        request,
        requester_email: get(row, "requester_email")?,
        ceremony_reference: get(row, "ceremony_reference")?,
        subject,
        created_at: decode::instant(row, "created_at_seconds", "created_at_nanoseconds")?,
        client_binding,
        firm_binding,
        client_role: role("client_role")?,
        firm_role: role("firm_role")?,
        signatures,
        completion,
    })
}

/// The stored contract as `viewer` may see it; `None` as viewer reads it
/// for the technical expiry closer.
pub(super) fn snapshot<C: GenericClient>(
    client: &mut C,
    viewer: Option<&Principal>,
    contract: Uuid,
) -> Result<Option<ContractSnapshot>, ApplicationError> {
    let Some(row) = client
        .query_opt("SELECT * FROM contracts WHERE id=$1", &[&contract])
        .map_err(port)?
    else {
        return Ok(None);
    };
    let root = decode::root(&row)?;
    let case_scope = match viewer {
        Some(principal) => staff_scope(client, principal, root.case.as_uuid())?,
        None => false,
    };
    let mut events = client
        .query(
            "SELECT * FROM contract_events WHERE contract_id=$1 ORDER BY revision",
            &[&contract],
        )
        .map_err(port)?
        .iter()
        .map(decode::event)
        .collect::<Result<Vec<_>, _>>()?;
    let mut requests = Vec::new();
    for row in client.query(REQUESTS, &[&contract]).map_err(port)? {
        requests.push(request(client, &row)?);
    }
    if let Some(principal) = viewer.filter(|_| !case_scope) {
        requests.retain(|request| {
            let fields = request.request.fields();
            fields.client == principal.id || fields.firm_signer == principal.id
        });
        if requests.is_empty() {
            return Ok(None);
        }
        let own: Vec<Uuid> = requests.iter().map(RequestRecord::id).collect();
        for event in &mut events {
            if event.request.is_some_and(|id| !own.contains(&id)) {
                event.actor = None;
                event.actor_label.clear();
                event.reason = None;
            }
        }
    }
    let latest: i64 = client
        .query_one(
            "SELECT coalesce(max(version),0) FROM documents WHERE id=$1",
            &[&root.document.as_uuid()],
        )
        .map_err(port)?
        .get(0);
    let case_closed: bool = client
        .query_one(
            "SELECT coalesce((SELECT administrative_status='closed' FROM case_administration_revisions
                WHERE case_id=$1 ORDER BY revision DESC LIMIT 1),FALSE)",
            &[&root.case.as_uuid()],
        )
        .map_err(port)?
        .get(0);
    Ok(Some(ContractSnapshot {
        root,
        events,
        requests,
        case_scope,
        latest_version: u32::try_from(latest).map_err(|_| inconsistent())?,
        case_closed,
    }))
}

/// The exact version a request names, for its parties and staff with scope.
pub(super) fn request_document<C: GenericClient>(
    client: &mut C,
    principal: &Principal,
    request: Uuid,
) -> Result<DocumentRecord, ApplicationError> {
    let not_found =
        || ApplicationError::from(application::contracts::ContractWorkflowError::NotFound);
    let row = client
        .query_opt(
            "SELECT d.id,d.version,d.name,d.digest,d.vault,d.evidence,r.case_id,r.client_id,r.firm_signer_id
            FROM contract_signature_requests r
            JOIN documents d ON d.id=r.document_id AND d.version=r.document_version
            WHERE r.request_id=$1",
            &[&request],
        )
        .map_err(port)?
        .ok_or_else(not_found)?;
    let party = [
        get::<Uuid>(&row, "client_id")?,
        get(&row, "firm_signer_id")?,
    ]
    .contains(&principal.id.as_uuid());
    if !party && !staff_scope(client, principal, get(&row, "case_id")?)? {
        return Err(not_found());
    }
    crate::document_postgres::decode_record(row)
}
