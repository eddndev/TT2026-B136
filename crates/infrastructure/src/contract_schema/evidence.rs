//! Decodes every stored statement with the domain codecs and rechecks each
//! party signature with the trust revision it names, never a current CRL.
//! Token instants against `created_at` and `expires_at` are judged by the
//! signing service and the validator, not here.
use super::{completion, inconsistent, port};
use crate::RingSha256Hasher;
use application::ApplicationError;
use domain::{
    cases::CaseId,
    crypto::{
        DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef,
        InternalDeclarationVerifier, Sha256Digest, Signature,
    },
    document_signing::contract::{ContractPartyStatement, ContractPosition, ContractRequest},
    identity::UserId,
};
use evidence_verify::{policy::DEFAULT_ADMITTED_POLICY, timestamp};
use postgres::{GenericClient, Row};
use uuid::Uuid;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for row in client
        .query(
            "SELECT * FROM contract_signature_requests ORDER BY created_at_seconds,request_id",
            &[],
        )
        .map_err(port)?
    {
        request(&row)?;
    }
    let signatures = client
        .query(
            "SELECT s.*,r.statement AS request_statement,r.content_digest,b.certificate_der,
                t.crl_der,a.root_der,f.signature AS first_signature,f.timestamp_token AS first_token
            FROM contract_party_signatures s
            JOIN contract_signature_requests r ON r.request_id=s.request_id
            JOIN document_signing_bindings b ON b.binding_id=s.binding_id
            JOIN participant_credential_trust_revisions t
                ON t.deployment_id=s.deployment_id AND t.revision=s.trust_revision
            JOIN participant_credential_authority a ON a.deployment_id=s.deployment_id
            LEFT JOIN contract_party_signatures f ON f.request_id=s.request_id AND f.\"position\"=1
            ORDER BY s.request_id,s.\"position\"",
            &[],
        )
        .map_err(port)?;
    let count: i64 = client
        .query_one("SELECT count(*) FROM contract_party_signatures", &[])
        .map_err(port)?
        .get(0);
    if usize::try_from(count).ok() != Some(signatures.len()) {
        return Err(inconsistent());
    }
    for row in signatures {
        signature(&row)?;
    }
    completion::validate(client)
}

pub(super) fn digest(row: &Row, column: &str) -> Result<Sha256Digest, ApplicationError> {
    let bytes: Vec<u8> = row.try_get(column).map_err(port)?;
    Sha256Digest::from_bytes(&bytes).map_err(|_| inconsistent())
}

pub(super) fn sha256(bytes: &[u8]) -> Sha256Digest {
    RingSha256Hasher.hash_bytes(bytes)
}

fn uuid(row: &Row, column: &str) -> Result<Uuid, ApplicationError> {
    row.try_get(column).map_err(port)
}

fn request(row: &Row) -> Result<(), ApplicationError> {
    let statement: Vec<u8> = row.try_get("statement").map_err(port)?;
    let decoded = ContractRequest::from_canonical_bytes(&statement).map_err(|_| inconsistent())?;
    let version = u32::try_from(row.try_get::<_, i64>("document_version").map_err(port)?)
        .map_err(|_| inconsistent())?;
    let fields = decoded.fields();
    let document = DocumentVersionRef {
        id: DocumentId::from_uuid(uuid(row, "document_id")?),
        version: DocumentVersion::new(version).map_err(|_| inconsistent())?,
    };
    if fields.deployment != uuid(row, "deployment_id")?
        || fields.request != uuid(row, "request_id")?
        || fields.case != CaseId::from_uuid(uuid(row, "case_id")?)
        || fields.document != document
        || fields.content_digest != digest(row, "content_digest")?
        || fields.client != UserId::from_uuid(uuid(row, "client_id")?)
        || fields.firm_signer != UserId::from_uuid(uuid(row, "firm_signer_id")?)
        || fields.requester != UserId::from_uuid(uuid(row, "requester_id")?)
        || fields.client_leaf != digest(row, "client_leaf")?
        || fields.firm_leaf != digest(row, "firm_leaf")?
        || fields.created_at_seconds != row.try_get::<_, i64>("created_at_seconds").map_err(port)?
        || fields.expires_at_seconds != row.try_get::<_, i64>("expires_at_seconds").map_err(port)?
    {
        return Err(inconsistent());
    }
    Ok(())
}

fn signature(row: &Row) -> Result<(), ApplicationError> {
    let bytes: Vec<u8> = row.try_get("statement").map_err(port)?;
    let statement =
        ContractPartyStatement::from_canonical_bytes(&bytes).map_err(|_| inconsistent())?;
    let account = statement.account();
    let material = statement.material();
    let request: Vec<u8> = row.try_get("request_statement").map_err(port)?;
    let signature: Vec<u8> = row.try_get("signature").map_err(port)?;
    let position: i16 = row.try_get("position").map_err(port)?;
    let chain_matches = match statement.position() {
        ContractPosition::Client => position == 1,
        ContractPosition::Firm {
            signature_digest,
            token_digest,
        } => {
            let first: Option<Vec<u8>> = row.try_get("first_signature").map_err(port)?;
            let token: Option<Vec<u8>> = row.try_get("first_token").map_err(port)?;
            position == 2
                && first.is_some_and(|first| sha256(&first) == signature_digest)
                && token.is_some_and(|token| sha256(&token) == token_digest)
        }
    };
    let counter = |column: &str| -> Result<u64, ApplicationError> {
        u64::try_from(row.try_get::<_, i64>(column).map_err(port)?).map_err(|_| inconsistent())
    };
    if !chain_matches
        || account.id() != UserId::from_uuid(uuid(row, "signer_id")?)
        || account.revision() != counter("account_revision")?
        || account.generation() != counter("auth_generation")?
        || material.deployment() != uuid(row, "deployment_id")?
        || material.binding() != uuid(row, "binding_id")?
        || material.root() != digest(row, "root_fingerprint")?
        || material.leaf() != digest(row, "leaf_fingerprint")?
        || u64::from(material.trust_revision()) != counter("trust_revision")?
        || statement.operation() != uuid(row, "operation_id")?
        || statement.request() != uuid(row, "request_id")?
        || statement.request_digest() != sha256(&request)
        || statement.content_digest() != digest(row, "content_digest")?
    {
        return Err(inconsistent());
    }
    let check = InternalDeclarationVerifier::verify(
        &crate::certificates::InternalRsaDeclarationVerifier,
        &sha256(&bytes),
        &row.try_get::<_, Vec<u8>>("certificate_der").map_err(port)?,
        &Signature::from_bytes(signature.clone()).map_err(|_| inconsistent())?,
        &row.try_get::<_, Vec<u8>>("root_der").map_err(port)?,
        &row.try_get::<_, Vec<u8>>("crl_der").map_err(port)?,
        row.try_get("checked_at").map_err(port)?,
    )
    .map_err(|_| inconsistent())?;
    if check.valid_from != row.try_get::<_, i64>("valid_from").map_err(port)?
        || check.valid_until != row.try_get::<_, i64>("valid_until").map_err(port)?
    {
        return Err(inconsistent());
    }
    if position == 1 {
        let token: Vec<u8> = row.try_get("timestamp_token").map_err(port)?;
        timestamp::verify(
            &token,
            sha256(&signature).as_bytes(),
            &[DEFAULT_ADMITTED_POLICY],
        )
        .map_err(|_| inconsistent())?;
    }
    Ok(())
}
