use super::*;
use crate::documents::DocumentRecord;

pub(super) fn participant_supports(
    request: &ParticipantPreparationRequest,
    subject: &SubjectValues,
) -> Result<Vec<ParticipantEvidenceLocator>, ApplicationError> {
    let mut refs = vec![subject.identity_support().clone()];
    refs.extend(
        request
            .proposal
            .values()
            .role()
            .supports()
            .into_iter()
            .cloned(),
    );
    refs.extend(request.review.different.iter().map(|d| d.support.clone()));
    unique_supports(refs)
}
pub(super) fn subject_supports(
    request: &SubjectReplacementRequest,
) -> Result<Vec<ParticipantEvidenceLocator>, ApplicationError> {
    let mut refs = vec![request.values.identity_support().clone()];
    refs.extend(request.review.different.iter().map(|d| d.support.clone()));
    unique_supports(refs)
}
fn unique_supports(
    mut refs: Vec<ParticipantEvidenceLocator>,
) -> Result<Vec<ParticipantEvidenceLocator>, ApplicationError> {
    refs.sort_by_key(|r| (r.reference().id.as_uuid(), r.reference().version.get()));
    if refs
        .windows(2)
        .any(|p| p[0].reference() == p[1].reference() && p[0].digest() != p[1].digest())
    {
        return Err(domain::DomainError::ParticipantSupportDigestMismatch.into());
    }
    refs.dedup_by_key(|r| r.reference());
    if refs.len() > 2 {
        return Err(ApplicationError::InvalidInput(
            "participant support batch exceeds two document versions".into(),
        ));
    }
    Ok(refs)
}
pub(super) fn ordered_records(
    refs: &[ParticipantEvidenceLocator],
    mut records: Vec<DocumentRecord>,
) -> Result<Vec<DocumentRecord>, ApplicationError> {
    if refs.len() != records.len() {
        return Err(ApplicationError::ParticipantSupportChanged);
    }
    records.sort_by_key(|r| (r.id.as_uuid(), r.version.get()));
    if !refs.iter().zip(&records).all(|(s, r)| {
        s.reference().id == r.id && s.reference().version == r.version && s.digest() == r.digest
    }) {
        return Err(ApplicationError::ParticipantSupportChanged);
    }
    Ok(records)
}
