//! Bounded lists: the contracts of a case with its eligible firm signers,
//! the caller's own requests, and the exhausted requests to expire.

use super::{
    decode::get,
    errors::{inconsistent, port},
    read::{snapshot, staff_scope},
};
use application::{
    contracts::{CaseContractsListing, ContractSnapshot, ContractWorkflowError, FirmSigner},
    identity::Principal,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    identity::{Permission, Role, UserId},
};
use postgres::GenericClient;
use std::str::FromStr;
use uuid::Uuid;

const LIMIT: i64 = 50;

fn snapshots<C: GenericClient>(
    client: &mut C,
    viewer: Option<&Principal>,
    ids: Vec<Uuid>,
) -> Result<Vec<ContractSnapshot>, ApplicationError> {
    let mut values = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(value) = snapshot(client, viewer, id)? {
            values.push(value);
        }
    }
    Ok(values)
}

pub(super) fn case_contracts<C: GenericClient>(
    client: &mut C,
    principal: &Principal,
    case: CaseId,
) -> Result<CaseContractsListing, ApplicationError> {
    if !principal.role.allows(Permission::ReadContract)
        || !staff_scope(client, principal, case.as_uuid())?
    {
        return Err(ContractWorkflowError::NotFound.into());
    }
    let ids = client
        .query(
            "SELECT id FROM contracts WHERE case_id=$1
            ORDER BY created_at_seconds DESC,created_at_nanoseconds DESC,id LIMIT $2",
            &[&case.as_uuid(), &LIMIT],
        )
        .map_err(port)?
        .iter()
        .map(|row| row.get(0))
        .collect();
    let contracts = snapshots(client, Some(principal), ids)?;
    let firm_signers = client
        .query(
            "SELECT u.id,u.email,u.role FROM users u WHERE u.active
                AND (u.role='owner' OR (u.role='litigator' AND EXISTS(SELECT 1 FROM case_memberships m
                    WHERE m.case_id=$1 AND m.user_id=u.id)))
                AND EXISTS(SELECT 1 FROM document_signing_bindings b
                    JOIN participant_credential_authority a ON a.deployment_id=b.deployment_id
                    WHERE b.signer_id=u.id AND b.root_fingerprint=a.root_fingerprint
                    AND NOT EXISTS(SELECT 1 FROM document_signing_withdrawals w
                        WHERE w.binding_id=b.binding_id))
            ORDER BY u.email,u.id LIMIT $2",
            &[&case.as_uuid(), &LIMIT],
        )
        .map_err(port)?
        .iter()
        .map(|row| {
            Ok(FirmSigner {
                id: UserId::from_uuid(get(row, "id")?),
                email: get(row, "email")?,
                role: Role::from_str(get::<&str>(row, "role")?).map_err(|_| inconsistent())?,
            })
        })
        .collect::<Result<Vec<_>, ApplicationError>>()?;
    Ok(CaseContractsListing {
        contracts,
        firm_signers,
    })
}

pub(super) fn party_contracts<C: GenericClient>(
    client: &mut C,
    principal: &Principal,
) -> Result<Vec<ContractSnapshot>, ApplicationError> {
    let ids = client
        .query(
            "SELECT contract_id FROM contract_signature_requests
            WHERE client_id=$1 OR firm_signer_id=$1 GROUP BY contract_id
            ORDER BY max(created_at_seconds) DESC,contract_id LIMIT $2",
            &[&principal.id.as_uuid(), &LIMIT],
        )
        .map_err(port)?
        .iter()
        .map(|row| row.get(0))
        .collect();
    snapshots(client, Some(principal), ids)
}

/// Contracts whose last event is `requested` or `client_signed` of a
/// request exhausted at `at`, oldest expiry first.
pub(super) fn exhausted<C: GenericClient>(
    client: &mut C,
    at: i64,
    limit: usize,
) -> Result<Vec<ContractSnapshot>, ApplicationError> {
    let limit = i64::try_from(limit).map_err(|_| inconsistent())?;
    let ids = client
        .query(
            "SELECT r.contract_id FROM contract_signature_requests r
            JOIN LATERAL (SELECT e.kind,e.request_id FROM contract_events e
                WHERE e.contract_id=r.contract_id ORDER BY e.revision DESC LIMIT 1) last ON TRUE
            WHERE r.expires_at_seconds<=$1 AND last.request_id=r.request_id
                AND last.kind IN ('requested','client_signed')
            ORDER BY r.expires_at_seconds,r.request_id LIMIT $2",
            &[&at, &limit],
        )
        .map_err(port)?
        .iter()
        .map(|row| row.get(0))
        .collect();
    snapshots(client, None, ids)
}
