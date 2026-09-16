use super::{
    projection,
    proposal::Proposal,
    review::{Candidate, Review},
    values::SubjectRef,
};
use crate::error::ApiError;
use application::typed_participants as m;
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::{cases::CaseId, crypto::CertificateSummary};
use serde_json::{json, Value};

pub(super) fn review(case: CaseId, row: m::ParticipantProposalReview) -> Value {
    json!({"case_id":case.to_string(),"proposal":Proposal::from(&row.proposal),
        "directory_stamp":row.directory_stamp.0.to_hex(),"candidates":row.candidates.into_iter().map(Candidate::from).collect::<Vec<_>>()})
}
pub(super) fn draft(case: CaseId, row: m::ParticipantSigningDraft) -> Value {
    json!({"case_id":case.to_string(),"proposal":Proposal::from(&row.proposal),"review":Review::from(&row.review),
        "declaration":row.declaration.map(declaration),"submission_digest":row.submission_digest.map(|v|v.to_hex()),
        "submission_revision":row.submission_revision.get()})
}
fn declaration(row: m::ParticipantDeclaration) -> Value {
    json!({"bytes_base64":STANDARD.encode(row.bytes),"digest":row.digest.to_hex(),
        "participant_values_digest":row.participant_values_digest.to_hex(),"certificate":{
            "der_base64":STANDARD.encode(row.certificate.der),"fingerprint":row.certificate.fingerprint.to_hex(),
            "summary":summary(&row.certificate.summary)},"deployment_id":row.deployment_id.to_string(),
        "trust_revision":row.trust_revision.get(),"root_fingerprint":row.root_fingerprint.to_hex(),"policy":"internal_demo_v1"})
}
fn summary(value: &CertificateSummary) -> Value {
    json!({"subject":value.subject,"issuer":value.issuer,"serial_hex":value.serial_hex,
        "not_before_unix":value.not_before_unix,"not_after_unix":value.not_after_unix})
}
pub(super) fn credential(row: m::ParticipantCredentialEvidence) -> Result<Value, ApiError> {
    let c = &row.check;
    let t = &row.trust;
    let i = &t.inspection;
    Ok(
        json!({"case_id":row.case_id.to_string(),"reference":projection::credential_ref(&row.reference),
        "subject":SubjectRef::from(row.subject),"declaration_base64":STANDARD.encode(row.declaration),
        "statement_digest":c.statement_digest.to_hex(),"certificate_der_base64":STANDARD.encode(&c.certificate.der),
        "certificate_fingerprint":c.certificate.fingerprint.to_hex(),"certificate_summary":summary(&c.certificate.summary),
        "signature_base64":STANDARD.encode(c.signature.as_bytes()),"policy":"internal_demo_v1",
        "checked_at_unix":c.checked_at,"valid_from_unix":c.valid_from,"valid_until_unix":c.valid_until,
        "trust":{"deployment_id":t.deployment_id.to_string(),"revision":t.revision.get(),
            "root_der_base64":STANDARD.encode(&i.root_der),"crl_der_base64":STANDARD.encode(&i.crl_der),
            "root_fingerprint":i.root_fingerprint.to_hex(),"crl_digest":i.crl_digest.to_hex(),"crl_number":i.crl_number.to_string(),
            "crl_this_update_unix":i.crl_this_update,"crl_next_update_unix":i.crl_next_update,
            "valid_from_unix":i.valid_from,"valid_until_unix":i.valid_until,
            "published_at":projection::time(t.published_at)?,"published_by":t.published_by},
        "accepted_at":projection::time(row.accepted_at)?,"accepted_by":projection::actor(&row.accepted_by)}),
    )
}
