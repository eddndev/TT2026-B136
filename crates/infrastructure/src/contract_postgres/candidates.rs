//! The parties and document a new request would name. A party that is
//! missing, ineligible or without a live binding under the current root is
//! absent, without saying which; there is no lookup by e-mail elsewhere.

use super::{
    decode::get,
    errors::{inconsistent, port},
    read::staff_scope,
};
use application::{
    contracts::{ContractWorkflowError, RequestCandidates},
    document_signing::SigningBindingReceipt,
    identity::Principal,
    ApplicationError,
};
use domain::identity::{Permission, Role, UserId};
use postgres::GenericClient;
use std::str::FromStr;
use uuid::Uuid;

/// The current unwithdrawn binding of `account` under the current root.
fn live_binding<C: GenericClient>(
    client: &mut C,
    account: Uuid,
) -> Result<Option<SigningBindingReceipt>, ApplicationError> {
    let binding: Option<Uuid> = client
        .query_opt(
            "SELECT b.binding_id FROM document_signing_bindings b
            JOIN participant_credential_authority a ON a.deployment_id=b.deployment_id
            WHERE b.signer_id=$1 AND b.root_fingerprint=a.root_fingerprint
                AND NOT EXISTS(SELECT 1 FROM document_signing_withdrawals w WHERE w.binding_id=b.binding_id)
            ORDER BY b.registered_at_seconds DESC,b.registered_at_nanoseconds DESC LIMIT 1",
            &[&account],
        )
        .map_err(port)?
        .map(|row| row.get(0));
    match binding {
        Some(id) => crate::signing_binding_postgres::load(client, id),
        None => Ok(None),
    }
}

pub(super) fn load<C: GenericClient>(
    client: &mut C,
    principal: &Principal,
    contract: Uuid,
    client_email: &str,
    firm_signer: UserId,
) -> Result<RequestCandidates, ApplicationError> {
    let not_found = || ApplicationError::from(ContractWorkflowError::NotFound);
    let row = client
        .query_opt(
            "SELECT case_id,document_id FROM contracts WHERE id=$1",
            &[&contract],
        )
        .map_err(port)?
        .ok_or_else(not_found)?;
    let case: Uuid = get(&row, "case_id")?;
    if !principal.role.allows(Permission::ManageContract) || !staff_scope(client, principal, case)?
    {
        return Err(not_found());
    }
    let account: Option<Uuid> = client
        .query_opt(
            "SELECT u.id FROM users u WHERE u.email=$1 AND u.role='client' AND u.active
                AND EXISTS(SELECT 1 FROM member_invitation_completions c
                    JOIN member_invitations i ON i.id=c.invitation_id
                    WHERE c.user_id=u.id AND i.role='client')",
            &[&client_email],
        )
        .map_err(port)?
        .map(|row| row.get(0));
    let client_party = match account {
        Some(id) => live_binding(client, id)?.map(|binding| (UserId::from_uuid(id), binding)),
        None => None,
    };
    let firm_role: Option<String> = client
        .query_opt(
            "SELECT u.role FROM users u WHERE u.id=$1 AND u.active AND (u.role='owner'
                OR (u.role='litigator' AND EXISTS(SELECT 1 FROM case_memberships m
                    WHERE m.case_id=$2 AND m.user_id=u.id)))",
            &[&firm_signer.as_uuid(), &case],
        )
        .map_err(port)?
        .map(|row| row.get(0));
    let firm_party = match firm_role {
        Some(role) => {
            let role = Role::from_str(&role).map_err(|_| inconsistent())?;
            live_binding(client, firm_signer.as_uuid())?.map(|binding| (firm_signer, role, binding))
        }
        None => None,
    };
    let document = client
        .query_opt(
            "SELECT id,version,name,digest,vault,evidence FROM documents
            WHERE id=$1 AND case_id=$2 ORDER BY version DESC LIMIT 1",
            &[&get::<Uuid>(&row, "document_id")?, &case],
        )
        .map_err(port)?
        .map(crate::document_postgres::decode_record)
        .transpose()?;
    Ok(RequestCandidates {
        client: client_party,
        firm_signer: firm_party,
        document,
        trust: crate::credential_trust_postgres::current(client)?,
    })
}
