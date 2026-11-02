//! JSON projections of contracts, requests, statements and receipts. Account
//! counters are decimal strings; statement instants are Unix seconds and
//! recorded instants RFC 3339. No private key or password is ever present.

use crate::error::ApiError;
use application::contracts::{
    CaseContracts, ContractEvent, ContractReceipt, ContractView, PartySignature,
    PreparedContractSigning, RequestRecord, RequestStatus, RequestView,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::{
    clock::OffsetDateTime, crypto::CredentialCertificate,
    document_signing::contract::ContractPartyStatement,
};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;

pub(super) fn case_contracts(row: CaseContracts) -> Result<Value, ApiError> {
    let contracts = row
        .contracts
        .iter()
        .map(contract)
        .collect::<Result<Vec<_>, _>>()?;
    let signers: Vec<Value> = row
        .firm_signers
        .iter()
        .map(|signer| {
            json!({"account_id":signer.id.to_string(),"email":signer.email,
                "role":signer.role.as_str()})
        })
        .collect();
    Ok(json!({"contracts":contracts,"firm_signers":signers}))
}

pub(super) fn contract(view: &ContractView) -> Result<Value, ApiError> {
    let snapshot = &view.snapshot;
    let root = &snapshot.root;
    let events = snapshot
        .events
        .iter()
        .map(event)
        .collect::<Result<Vec<_>, _>>()?;
    let mut requests = Vec::with_capacity(snapshot.requests.len());
    for record in &snapshot.requests {
        let status = view.status(record.id()).ok_or_else(ApiError::internal)?;
        requests.push(request(record, status, view)?);
    }
    Ok(
        json!({"contract_id":root.id.to_string(),"case_id":root.case.to_string(),
        "document_id":root.document.to_string(),"title":root.title,
        "created_by":root.created_by.to_string(),"created_by_email":root.created_by_email,
        "created_at":timestamp(root.created_at)?,"state":view.state.as_str(),
        "revision":snapshot.revision(),"derived_at_unix":view.derived_at,
        "events":events,"requests":requests}),
    )
}

fn event(value: &ContractEvent) -> Result<Value, ApiError> {
    Ok(json!({"revision":value.revision,"kind":value.kind.as_str(),
        "request_id":value.request.map(|id| id.to_string()),
        "actor_id":value.actor.map(|id| id.to_string()),"actor_label":value.actor_label,
        "reason":value.reason,"at":timestamp(value.at)?,
        "operation_id":value.operation.to_string()}))
}

fn request(
    record: &RequestRecord,
    status: RequestStatus,
    view: &ContractView,
) -> Result<Value, ApiError> {
    let fields = record.request.fields();
    let signed = |position: u8| -> Result<Option<String>, ApiError> {
        record
            .signature(position)
            .map(|value| timestamp(value.accepted_at))
            .transpose()
    };
    let completed = record
        .completion
        .as_ref()
        .map(|value| timestamp(value.completed_at))
        .transpose()?;
    let subject = record
        .subject
        .map(|(id, revision)| json!({"subject_id":id.to_string(),"revision":revision.to_string()}));
    Ok(
        json!({"request_id":record.id().to_string(),"status":status.as_str(),
        "contract_id":view.snapshot.root.id.to_string(),
        "document_id":fields.document.id.to_string(),"version":fields.document.version.get(),
        "document_digest":fields.content_digest.to_hex(),
        "client_id":fields.client.to_string(),"firm_signer_id":fields.firm_signer.to_string(),
        "requester_id":fields.requester.to_string(),"requester_email":record.requester_email,
        "firm_signer_role":record.firm_role.as_str(),
        "client_certificate_fingerprint":fields.client_leaf.to_hex(),
        "firm_certificate_fingerprint":fields.firm_leaf.to_hex(),
        "created_at_unix":fields.created_at_seconds,"expires_at_unix":fields.expires_at_seconds,
        "created_at":timestamp(record.created_at)?,"ceremony_reference":record.ceremony_reference,
        "subject":subject,"client_signed_at":signed(1)?,"firm_signed_at":signed(2)?,
        "completed_at":completed}),
    )
}

pub(super) fn my_requests(rows: &[RequestView]) -> Result<Value, ApiError> {
    let mut requests = Vec::with_capacity(rows.len());
    for row in rows {
        let record = row
            .contract
            .snapshot
            .request(row.request)
            .ok_or_else(ApiError::internal)?;
        let mut value = request(record, row.status, &row.contract)?;
        value["case_id"] = json!(row.contract.snapshot.root.case.to_string());
        value["title"] = json!(row.contract.snapshot.root.title);
        value["contract_state"] = json!(row.contract.state.as_str());
        value["revision"] = json!(row.contract.snapshot.revision());
        requests.push(value);
    }
    Ok(json!({ "requests": requests }))
}

/// One request with its exact bytes, both certificates and the signatures.
pub(super) fn request_detail(row: &RequestView) -> Result<Value, ApiError> {
    let record = row
        .contract
        .snapshot
        .request(row.request)
        .ok_or_else(ApiError::internal)?;
    let mut value = request(record, row.status, &row.contract)?;
    value["statement_base64"] = json!(STANDARD.encode(record.bytes()));
    value["revision"] = json!(row.contract.snapshot.revision());
    value["title"] = json!(row.contract.snapshot.root.title);
    value["case_id"] = json!(row.contract.snapshot.root.case.to_string());
    value["contract_state"] = json!(row.contract.state.as_str());
    value["client_certificate"] = certificate(&record.client_binding.check.certificate);
    value["firm_certificate"] = certificate(&record.firm_binding.check.certificate);
    value["signatures"] = json!(record
        .signatures
        .iter()
        .map(signature)
        .collect::<Result<Vec<_>, _>>()?);
    Ok(value)
}

fn certificate(value: &CredentialCertificate) -> Value {
    let summary = &value.summary;
    json!({"der_base64":STANDARD.encode(&value.der),"fingerprint":value.fingerprint.to_hex(),
        "summary":{"subject":summary.subject,"issuer":summary.issuer,"serial_hex":summary.serial_hex,
        "not_before_unix":summary.not_before_unix,"not_after_unix":summary.not_after_unix}})
}

fn statement(value: &ContractPartyStatement) -> Value {
    let (account, material, position) = (value.account(), value.material(), value.position());
    json!({"position":position.number(),"account_id":account.id().to_string(),
        "account_revision":account.revision().to_string(),
        "auth_generation":account.generation().to_string(),
        "binding_id":material.binding().to_string(),"deployment_id":material.deployment().to_string(),
        "root_fingerprint":material.root().to_hex(),"trust_revision":material.trust_revision(),
        "certificate_fingerprint":material.leaf().to_hex(),"operation_id":value.operation().to_string(),
        "request_id":value.request().to_string(),"request_sha256":value.request_digest().to_hex(),
        "document_digest":value.content_digest().to_hex(),
        "client_signature_sha256":position.signature_digest().map(|d| d.to_hex()),
        "client_timestamp_sha256":position.token_digest().map(|d| d.to_hex()),
        "statement_base64":STANDARD.encode(value.canonical_bytes())})
}

fn signature(value: &PartySignature) -> Result<Value, ApiError> {
    let mut row = statement(&value.statement);
    row["signature_base64"] = json!(STANDARD.encode(&value.signature));
    row["accepted_at"] = json!(timestamp(value.accepted_at)?);
    row["timestamp_base64"] = json!(value
        .timestamp
        .as_ref()
        .map(|token| STANDARD.encode(&token.token)));
    Ok(row)
}

pub(super) fn prepared(row: &PreparedContractSigning) -> Value {
    let mut value = statement(&row.statement);
    value["revision"] = json!(row.revision);
    value
}

pub(super) fn receipt(row: &ContractReceipt) -> Result<Value, ApiError> {
    Ok(
        json!({"contract_id":row.contract.to_string(),"case_id":row.case.to_string(),
        "event":event(&row.event)?,
        "request_statement_base64":row.request_statement.as_ref().map(|bytes| STANDARD.encode(bytes)),
        "signature":row.signature.as_ref().map(signature).transpose()?,
        "manifest_sha256":row.manifest_sha256.map(|digest| digest.to_hex())}),
    )
}

fn timestamp(value: OffsetDateTime) -> Result<String, ApiError> {
    value.format(&Rfc3339).map_err(|_| ApiError::internal())
}
