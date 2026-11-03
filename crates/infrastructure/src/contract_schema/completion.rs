//! Rechecks each completion: the document and manifest tokens, and the
//! manifest identity, signers and every artifact whose bytes are stored.
//! Certificates and CRLs are encoded by the exporter and checked there.
use super::{
    evidence::{digest, sha256},
    inconsistent, port,
};
use application::ApplicationError;
use evidence_verify::{
    manifest::contract::{parse, ContractRole},
    policy::DEFAULT_ADMITTED_POLICY,
    timestamp,
};
use postgres::{GenericClient, Row};
use std::collections::BTreeMap;
use uuid::Uuid;

const QUERY: &str = "SELECT c.*,r.deployment_id,r.case_id,r.document_id,r.document_version,
        r.content_digest,r.statement AS request_statement,r.expires_at_seconds,
        r.client_id,r.client_binding_id,r.firm_signer_id,r.firm_binding_id
    FROM contract_completions c JOIN contract_signature_requests r ON r.request_id=c.request_id
    ORDER BY c.request_id";

const PARTIES: &str = "SELECT s.\"position\",s.operation_id,s.statement,s.signature,
        s.timestamp_token,s.tsa_chain_pem,b.statement AS binding_statement,
        b.signature AS binding_signature,b.registered_at_seconds,a.root_der,t.crl_der
    FROM contract_party_signatures s
    JOIN document_signing_bindings b ON b.binding_id=s.binding_id
    JOIN participant_credential_authority a ON a.deployment_id=b.deployment_id
    JOIN participant_credential_trust_revisions t
        ON t.deployment_id=b.deployment_id AND t.revision=b.trust_revision
    WHERE s.request_id=$1 ORDER BY s.\"position\"";

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for row in client.query(QUERY, &[]).map_err(port)? {
        let request: Uuid = row.try_get("request_id").map_err(port)?;
        let parties = client.query(PARTIES, &[&request]).map_err(port)?;
        if parties.len() != 2 {
            return Err(inconsistent());
        }
        check(&row, &parties)?;
    }
    Ok(())
}

fn bytes(row: &Row, column: &str) -> Result<Vec<u8>, ApplicationError> {
    row.try_get(column).map_err(port)
}

fn check(row: &Row, parties: &[Row]) -> Result<(), ApplicationError> {
    let content = digest(row, "content_digest")?;
    let manifest = bytes(row, "manifest")?;
    for (token, covered) in [
        (bytes(row, "document_token")?, content),
        (bytes(row, "manifest_token")?, sha256(&manifest)),
    ] {
        timestamp::verify(&token, covered.as_bytes(), &[DEFAULT_ADMITTED_POLICY])
            .map_err(|_| inconsistent())?;
    }
    let parsed = parse(&manifest).map_err(|_| inconsistent())?;
    let request = bytes(row, "request_statement")?;
    let text = |column: &str| -> Result<String, ApplicationError> {
        Ok(row.try_get::<_, Uuid>(column).map_err(port)?.to_string())
    };
    let version = u64::try_from(row.try_get::<_, i64>("document_version").map_err(port)?)
        .map_err(|_| inconsistent())?;
    if parsed.deployment_id != text("deployment_id")?
        || parsed.case_id != text("case_id")?
        || parsed.request_id != text("request_id")?
        || parsed.document_id != text("document_id")?
        || parsed.document_version != version
        || parsed.document_sha256 != *content.as_bytes()
        || parsed.request_sha256 != *sha256(&request).as_bytes()
        || parsed.expires_at != row.try_get::<_, i64>("expires_at_seconds").map_err(port)?
    {
        return Err(inconsistent());
    }
    let mut stored: BTreeMap<(u8, ContractRole), Vec<u8>> = BTreeMap::from([
        ((0, ContractRole::Request), request),
        (
            (0, ContractRole::DocumentTimestamp),
            bytes(row, "document_token")?,
        ),
        ((0, ContractRole::TsaChain), bytes(row, "tsa_chain_pem")?),
    ]);
    let signers = [
        ("client_id", "client_binding_id"),
        ("firm_signer_id", "firm_binding_id"),
    ];
    for (index, (party, (account, binding))) in parties.iter().zip(signers).enumerate() {
        let number = u8::try_from(index + 1).map_err(|_| inconsistent())?;
        let signer = &parsed.signers[index];
        if signer.account_id != text(account)?
            || signer.binding_id != text(binding)?
            || signer.operation_id
                != party
                    .try_get::<_, Uuid>("operation_id")
                    .map_err(port)?
                    .to_string()
            || signer.binding_registered_at
                != party
                    .try_get::<_, i64>("registered_at_seconds")
                    .map_err(port)?
        {
            return Err(inconsistent());
        }
        for (role, column) in [
            (ContractRole::Statement, "statement"),
            (ContractRole::StatementSignature, "signature"),
            (ContractRole::BindingStatement, "binding_statement"),
            (ContractRole::BindingSignature, "binding_signature"),
            (ContractRole::BindingRoot, "root_der"),
            (ContractRole::BindingCrl, "crl_der"),
        ] {
            stored.insert((number, role), bytes(party, column)?);
        }
        if number == 1 {
            stored.insert(
                (1, ContractRole::StatementTimestamp),
                bytes(party, "timestamp_token")?,
            );
            stored.insert(
                (1, ContractRole::StatementTsaChain),
                bytes(party, "tsa_chain_pem")?,
            );
        }
    }
    for artifact in &parsed.artifacts {
        if artifact.role == ContractRole::Document {
            if artifact.sha256 != *content.as_bytes() {
                return Err(inconsistent());
            }
        } else if let Some(value) = stored.remove(&(artifact.party, artifact.role)) {
            if artifact.sha256 != *sha256(&value).as_bytes()
                || u64::try_from(value.len()).ok() != Some(artifact.size)
            {
                return Err(inconsistent());
            }
        }
    }
    if !stored.is_empty() {
        return Err(inconsistent());
    }
    Ok(())
}
