//! The request, signature and completion rows of a mutation.

use super::{
    errors::{inconsistent, refusal},
    write_steps::nanoseconds,
};
use application::{
    contracts::{Completion, StatementToken},
    identity::Principal,
    ApplicationError,
};
use domain::{
    clock::OffsetDateTime,
    crypto::CredentialCheck,
    document_signing::contract::{ContractPartyStatement, ContractRequest},
};
use postgres::Transaction;
use uuid::Uuid;

pub(super) struct RequestRow<'a> {
    pub(super) contract: Uuid,
    pub(super) request: &'a ContractRequest,
    pub(super) client_binding: Uuid,
    pub(super) firm_binding: Uuid,
    pub(super) ceremony_reference: &'a str,
    pub(super) subject: Option<(Uuid, u64)>,
    pub(super) requester: &'a Principal,
    pub(super) at: OffsetDateTime,
}

pub(super) fn request(
    tx: &mut Transaction<'_>,
    row: &RequestRow<'_>,
) -> Result<(), ApplicationError> {
    let fields = row.request.fields();
    if fields.created_at_seconds != row.at.unix_timestamp() || fields.requester != row.requester.id
    {
        return Err(inconsistent());
    }
    let subject_revision = row
        .subject
        .map(|(_, revision)| i64::try_from(revision))
        .transpose()
        .map_err(|_| inconsistent())?;
    tx.execute(
        "INSERT INTO contract_signature_requests(request_id,contract_id,case_id,document_id,
        document_version,content_digest,client_id,firm_signer_id,requester_id,requester_email,
        client_binding_id,client_leaf,firm_binding_id,firm_leaf,client_subject_id,
        client_subject_revision,ceremony_reference,deployment_id,statement,statement_digest,
        created_at_seconds,created_at_nanoseconds,expires_at_seconds)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,sha256($19),$20,$21,$22)",
        &[
            &fields.request,
            &row.contract,
            &fields.case.as_uuid(),
            &fields.document.id.as_uuid(),
            &i64::from(fields.document.version.get()),
            &&fields.content_digest.as_bytes()[..],
            &fields.client.as_uuid(),
            &fields.firm_signer.as_uuid(),
            &fields.requester.as_uuid(),
            &row.requester.email,
            &row.client_binding,
            &&fields.client_leaf.as_bytes()[..],
            &row.firm_binding,
            &&fields.firm_leaf.as_bytes()[..],
            &row.subject.map(|(id, _)| id),
            &subject_revision,
            &row.ceremony_reference,
            &fields.deployment,
            &&row.request.canonical_bytes()[..],
            &fields.created_at_seconds,
            &nanoseconds(row.at),
            &fields.expires_at_seconds,
        ],
    )
    .map_err(refusal)?;
    Ok(())
}

pub(super) struct SignatureRow<'a> {
    pub(super) request: Uuid,
    pub(super) operation: Uuid,
    pub(super) signer: &'a Principal,
    pub(super) statement: &'a ContractPartyStatement,
    pub(super) signature: &'a [u8],
    pub(super) check: &'a CredentialCheck,
    pub(super) timestamp: Option<&'a StatementToken>,
    pub(super) at: OffsetDateTime,
}

pub(super) fn signature(
    tx: &mut Transaction<'_>,
    row: &SignatureRow<'_>,
) -> Result<(), ApplicationError> {
    let material = row.statement.material();
    let account = row.statement.account();
    let counter = |value: u64| i64::try_from(value).map_err(|_| inconsistent());
    tx.execute(
        "INSERT INTO contract_party_signatures(request_id,position,operation_id,signer_id,binding_id,
        leaf_fingerprint,deployment_id,trust_revision,root_fingerprint,account_revision,auth_generation,
        actor_email,statement,statement_digest,signature,checked_at,valid_from,valid_until,
        accepted_at_seconds,accepted_at_nanoseconds,timestamp_token,tsa_chain_pem)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,sha256($13),$14,$15,$16,$17,$18,$19,$20,$21)",
        &[
            &row.request,
            &i16::from(row.statement.position().number()),
            &row.operation,
            &row.signer.id.as_uuid(),
            &material.binding(),
            &&material.leaf().as_bytes()[..],
            &material.deployment(),
            &i64::from(material.trust_revision()),
            &&material.root().as_bytes()[..],
            &counter(account.revision())?,
            &counter(account.generation())?,
            &row.signer.email,
            &&row.statement.canonical_bytes()[..],
            &row.signature,
            &row.check.checked_at,
            &row.check.valid_from,
            &row.check.valid_until,
            &row.at.unix_timestamp(),
            &nanoseconds(row.at),
            &row.timestamp.map(|token| token.token.as_slice()),
            &row.timestamp.map(|token| token.tsa_chain_pem.as_slice()),
        ],
    )
    .map_err(refusal)?;
    Ok(())
}

pub(super) fn completion(
    tx: &mut Transaction<'_>,
    request: Uuid,
    value: &Completion,
) -> Result<(), ApplicationError> {
    tx.execute(
        "INSERT INTO contract_completions(request_id,document_token,manifest,manifest_sha256,
        manifest_token,tsa_chain_pem,completed_at_seconds,completed_at_nanoseconds)
        VALUES($1,$2,$3,sha256($3),$4,$5,$6,$7)",
        &[
            &request,
            &value.document_token,
            &value.manifest,
            &value.manifest_token,
            &value.tsa_chain_pem,
            &value.completed_at.unix_timestamp(),
            &nanoseconds(value.completed_at),
        ],
    )
    .map_err(refusal)?;
    Ok(())
}
