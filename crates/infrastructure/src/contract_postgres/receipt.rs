//! The exact receipt of one committed mutation, found by its operation.

use super::{
    decode::{self, get},
    errors::port,
};
use application::{
    contracts::{ContractReceipt, EventKind},
    ApplicationError,
};
use domain::{cases::CaseId, crypto::Sha256Digest};
use postgres::GenericClient;
use uuid::Uuid;

pub(super) fn load<C: GenericClient>(
    client: &mut C,
    operation: Uuid,
) -> Result<Option<ContractReceipt>, ApplicationError> {
    let Some(row) = client
        .query_opt(
            "SELECT e.*,c.case_id FROM contract_events e JOIN contracts c ON c.id=e.contract_id
            WHERE e.operation_id=$1",
            &[&operation],
        )
        .map_err(port)?
    else {
        return Ok(None);
    };
    let event = decode::event(&row)?;
    let request_statement = match (event.kind, event.request) {
        (EventKind::Requested, Some(request)) => client
            .query_opt(
                "SELECT statement FROM contract_signature_requests WHERE request_id=$1",
                &[&request],
            )
            .map_err(port)?
            .map(|row| row.get(0)),
        _ => None,
    };
    let signature = client
        .query_opt(
            "SELECT s.*,a.root_der,t.crl_der FROM contract_party_signatures s
            JOIN participant_credential_trust_revisions t
                ON t.deployment_id=s.deployment_id AND t.revision=s.trust_revision
            JOIN participant_credential_authority a ON a.deployment_id=s.deployment_id
            WHERE s.operation_id=$1",
            &[&operation],
        )
        .map_err(port)?
        .as_ref()
        .map(decode::signature)
        .transpose()?;
    let manifest_sha256 = match (event.kind, event.request) {
        (EventKind::Completed, Some(request)) => client
            .query_opt(
                "SELECT manifest_sha256 FROM contract_completions WHERE request_id=$1",
                &[&request],
            )
            .map_err(port)?
            .map(|row| {
                let bytes: Vec<u8> = row.get(0);
                Sha256Digest::from_bytes(&bytes).map_err(|_| super::errors::inconsistent())
            })
            .transpose()?,
        _ => None,
    };
    Ok(Some(ContractReceipt {
        contract: get(&row, "contract_id")?,
        case: CaseId::from_uuid(get(&row, "case_id")?),
        event,
        request_statement,
        signature,
        manifest_sha256,
    }))
}
