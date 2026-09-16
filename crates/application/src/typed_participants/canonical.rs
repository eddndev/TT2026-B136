use super::*;
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest, Signature},
};

pub fn subject_digest(hasher: &dyn DocumentHasher, values: &SubjectValues) -> Sha256Digest {
    hasher.hash_bytes(&values.canonical_bytes())
}
pub fn typed_participant_digest(
    hasher: &dyn DocumentHasher,
    values: &TypedParticipantValues,
) -> Sha256Digest {
    hasher.hash_bytes(&values.canonical_bytes())
}
impl IdentityCandidateRef {
    pub fn sort_key(&self) -> (u8, Uuid, u32) {
        match self {
            Self::Subject { id, revision } => (0, id.as_uuid(), revision.get()),
            Self::ManualParticipant { id, revision } => (1, id.as_uuid(), revision.get()),
        }
    }
}
impl IdentityReviewSubmission {
    /// PREV1 records staff decisions, separately from the represented person's signature.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ApplicationError> {
        if self.different.len() > 16 {
            return Err(ApplicationError::ParticipantCandidateLimit);
        }
        let mut decisions = self.different.iter().collect::<Vec<_>>();
        decisions.sort_by_key(|d| d.candidate.sort_key());
        if decisions
            .windows(2)
            .any(|p| p[0].candidate.sort_key() == p[1].candidate.sort_key())
        {
            return Err(ApplicationError::ParticipantIdentityReviewRequired);
        }
        let mut out = b"PREV1".to_vec();
        out.extend_from_slice(self.directory_stamp.0.as_bytes());
        text(&mut out, self.selection_reason.as_str());
        out.extend_from_slice(&(decisions.len() as u32).to_be_bytes());
        for decision in decisions {
            let (kind, id, revision) = decision.candidate.sort_key();
            out.push(kind);
            out.extend_from_slice(id.as_bytes());
            out.extend_from_slice(&revision.to_be_bytes());
            text(&mut out, decision.reason.as_str());
            let support = &decision.support;
            out.extend_from_slice(support.reference().id.as_uuid().as_bytes());
            out.extend_from_slice(&support.reference().version.get().to_be_bytes());
            out.extend_from_slice(support.digest().as_bytes());
            text(&mut out, support.locator());
        }
        Ok(out)
    }
}
pub fn participant_review_digest(
    hasher: &dyn DocumentHasher,
    review: &IdentityReviewSubmission,
) -> Result<Sha256Digest, ApplicationError> {
    Ok(hasher.hash_bytes(&review.canonical_bytes()?))
}
/// The server constructs PCRED1; delivery adapters never hash their own JSON.
pub fn participant_declaration_bytes(
    case_id: CaseId,
    proposal: &ParticipantProposal,
    trust: &CredentialTrustSnapshot,
    certificate_fingerprint: Sha256Digest,
    hasher: &dyn DocumentHasher,
) -> Vec<u8> {
    let mut out = b"PCRED1\0\0".to_vec();
    out.extend_from_slice(trust.deployment_id.as_bytes());
    out.extend_from_slice(trust.inspection.root_fingerprint.as_bytes());
    out.extend_from_slice(case_id.as_uuid().as_bytes());
    let subject = proposal.values().subject();
    out.extend_from_slice(subject.id.as_uuid().as_bytes());
    out.push(match proposal.subject() {
        SubjectChange::Keep(_) => 0,
        SubjectChange::Append { .. } => 1,
    });
    out.extend_from_slice(&proposal.subject().expectation().get().to_be_bytes());
    out.extend_from_slice(&subject.revision.get().to_be_bytes());
    out.extend_from_slice(subject.values_digest.as_bytes());
    out.extend_from_slice(proposal.participant_id().as_uuid().as_bytes());
    out.extend_from_slice(&proposal.expected_participant().get().to_be_bytes());
    out.extend_from_slice(
        &proposal
            .expected_participant()
            .next()
            .expect("validated proposal")
            .get()
            .to_be_bytes(),
    );
    out.push(proposal.values().kind().tag());
    out.extend_from_slice(typed_participant_digest(hasher, proposal.values()).as_bytes());
    out.extend_from_slice(certificate_fingerprint.as_bytes());
    out
}
/// PTXN1 includes the exact review and optional proof, enabling explicit reconciliation.
pub fn participant_submission_digest(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    request: &ParticipantPreparationRequest,
    proof: Option<(Sha256Digest, Sha256Digest, &Signature)>,
) -> Result<Sha256Digest, ApplicationError> {
    Ok(hasher.hash_bytes(&participant_submission_bytes(
        hasher, case_id, request, proof,
    )?))
}
pub fn participant_submission_bytes(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    request: &ParticipantPreparationRequest,
    proof: Option<(Sha256Digest, Sha256Digest, &Signature)>,
) -> Result<Vec<u8>, ApplicationError> {
    let p = &request.proposal;
    let mut out = b"PTXN1".to_vec();
    out.extend_from_slice(case_id.as_uuid().as_bytes());
    out.push(match p.subject() {
        SubjectChange::Keep(_) => 0,
        SubjectChange::Append { .. } => 1,
    });
    let subject = p.values().subject();
    out.extend_from_slice(subject.id.as_uuid().as_bytes());
    out.extend_from_slice(&p.subject().expectation().get().to_be_bytes());
    out.extend_from_slice(&subject.revision.get().to_be_bytes());
    out.extend_from_slice(subject.values_digest.as_bytes());
    out.extend_from_slice(p.participant_id().as_uuid().as_bytes());
    out.extend_from_slice(&p.expected_participant().get().to_be_bytes());
    out.extend_from_slice(
        &p.expected_participant()
            .next()
            .ok_or(ApplicationError::ParticipantRevisionExhausted)?
            .get()
            .to_be_bytes(),
    );
    out.extend_from_slice(typed_participant_digest(hasher, p.values()).as_bytes());
    out.extend_from_slice(participant_review_digest(hasher, &request.review)?.as_bytes());
    match proof {
        None => out.push(0),
        Some((statement, certificate, signature)) => {
            if signature.as_bytes().len() != 384 {
                return Err(ApplicationError::ParticipantCredentialRejected(
                    domain::crypto::CredentialFailure::InvalidSignature,
                ));
            }
            out.push(1);
            out.extend_from_slice(statement.as_bytes());
            out.extend_from_slice(certificate.as_bytes());
            out.extend_from_slice(signature.as_bytes());
        }
    }
    Ok(out)
}
/// Starts a constant-memory hash fold over stable ordered current directory heads.
pub fn directory_stamp_seed(hasher: &dyn DocumentHasher, case_id: CaseId) -> CaseDirectoryStamp {
    let mut bytes = b"DIRST1".to_vec();
    bytes.extend_from_slice(case_id.as_uuid().as_bytes());
    CaseDirectoryStamp(hasher.hash_bytes(&bytes))
}
pub fn advance_directory_stamp(
    hasher: &dyn DocumentHasher,
    current: CaseDirectoryStamp,
    kind: u8,
    id: Uuid,
    revision: u32,
    digest: Sha256Digest,
) -> CaseDirectoryStamp {
    let mut bytes = current.0.as_bytes().to_vec();
    bytes.push(kind);
    bytes.extend_from_slice(id.as_bytes());
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(digest.as_bytes());
    CaseDirectoryStamp(hasher.hash_bytes(&bytes))
}
fn text(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u32).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}
