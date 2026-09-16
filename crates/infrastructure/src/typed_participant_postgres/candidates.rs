use super::*;
fn known_name(values: &SubjectValues) -> Option<&str> {
    match values {
        SubjectValues::NaturalPerson {
            name: RepresentedName::Known(name),
            ..
        } => Some(name.as_str()),
        SubjectValues::InstitutionalBody { name, .. } => Some(name.as_str()),
        _ => None,
    }
}
pub(super) fn find(
    tx: &mut impl GenericClient,
    case: CaseId,
    values: &SubjectValues,
    exclude_subject: Option<CaseSubjectId>,
    exclude_participant: Option<ParticipantId>,
    certificate: Option<Sha256Digest>,
) -> Result<Vec<IdentityCandidate>, ApplicationError> {
    let mut result = Vec::new();
    let mut cursor: Option<Uuid> = None;
    let exclude = exclude_subject.map(CaseSubjectId::as_uuid);
    let fingerprint = certificate.map(|d| d.as_bytes().to_vec());
    loop {
        let rows=tx.query("SELECT s.id,r.revision,r.subject_kind,r.display_name,r.name_known,r.declared_identifier,r.identity_document_digest,r.identity_document_locator,EXISTS(SELECT 1 FROM case_participant_typed_revisions p JOIN participant_credential_evidence e ON e.participant_id=p.participant_id AND e.revision=p.credential_origin_revision WHERE p.subject_id=s.id AND e.certificate_fingerprint=$4) AS certificate_match FROM case_subjects s JOIN LATERAL(SELECT revision,subject_kind,display_name,name_known,declared_identifier,identity_document_digest,identity_document_locator FROM case_subject_revisions WHERE subject_id=s.id ORDER BY revision DESC LIMIT 1) r ON TRUE WHERE s.case_id=$1 AND ($2::uuid IS NULL OR s.id>$2) AND s.id IS DISTINCT FROM $3 ORDER BY s.id LIMIT 64",&[&case.as_uuid(),&cursor,&exclude,&fingerprint]).map_err(port)?;
        for row in &rows {
            let id: Uuid = row.get("id");
            cursor = Some(id);
            let mut signals = Vec::new();
            let name: String = row.get("display_name");
            if row.get::<_, bool>("name_known")
                && known_name(values).is_some_and(|n| n.eq_ignore_ascii_case(&name))
            {
                signals.push(IdentityCandidateSignal::Name);
            }
            if values.declared_identifier().is_some()
                && values.declared_identifier()
                    == row
                        .get::<_, Option<String>>("declared_identifier")
                        .as_deref()
            {
                signals.push(IdentityCandidateSignal::DeclaredIdentifier);
            }
            if row.get::<_, bool>("certificate_match") {
                signals.push(IdentityCandidateSignal::Certificate);
            }
            if digest(row.get("identity_document_digest"))? == values.identity_support().digest()
                && row.get::<_, String>("identity_document_locator")
                    == values.identity_support().locator()
            {
                signals.push(IdentityCandidateSignal::DocumentaryEvidence);
            }
            if !signals.is_empty() {
                result.push(IdentityCandidate {
                    reference: IdentityCandidateRef::Subject {
                        id: CaseSubjectId::from_uuid(id),
                        revision: subject_revision(row.get("revision"))?,
                    },
                    display_name: name,
                    kind: Some(kind(&row.get::<_, String>("subject_kind"))?),
                    signals,
                });
            }
            if result.len() > 16 {
                return Err(ApplicationError::ParticipantCandidateLimit);
            }
        }
        if rows.len() < 64 {
            break;
        }
    }
    if let Some(name) = known_name(values) {
        let mut cursor: Option<Uuid> = None;
        let exclude = exclude_participant.map(ParticipantId::as_uuid);
        loop {
            let rows=tx.query("SELECT p.id,r.revision,r.display_name FROM case_participants p JOIN LATERAL(SELECT revision,display_name FROM case_participant_revisions WHERE participant_id=p.id ORDER BY revision DESC LIMIT 1) r ON TRUE WHERE p.case_id=$1 AND ($2::uuid IS NULL OR p.id>$2) AND p.id IS DISTINCT FROM $3 AND NOT EXISTS(SELECT 1 FROM case_participant_typed_revisions WHERE participant_id=p.id) ORDER BY p.id LIMIT 64",&[&case.as_uuid(),&cursor,&exclude]).map_err(port)?;
            for row in &rows {
                let id: Uuid = row.get(0);
                cursor = Some(id);
                let candidate: String = row.get(2);
                if name.eq_ignore_ascii_case(&candidate) {
                    result.push(IdentityCandidate {
                        reference: IdentityCandidateRef::ManualParticipant {
                            id: ParticipantId::from_uuid(id),
                            revision: revision(row.get(1))?,
                        },
                        display_name: candidate,
                        kind: None,
                        signals: vec![IdentityCandidateSignal::Name],
                    });
                }
                if result.len() > 16 {
                    return Err(ApplicationError::ParticipantCandidateLimit);
                }
            }
            if rows.len() < 64 {
                break;
            }
        }
    }
    result.sort_by_key(|c| c.reference.sort_key());
    Ok(result)
}
pub(super) fn require_review(
    review: &IdentityReviewSubmission,
    stamp: &CaseDirectoryStamp,
    candidates: &[IdentityCandidate],
) -> Result<(), ApplicationError> {
    review.canonical_bytes()?;
    if &review.directory_stamp != stamp {
        return Err(ApplicationError::ParticipantIdentityReviewConflict);
    }
    let mut supplied = review
        .different
        .iter()
        .map(|c| c.candidate.sort_key())
        .collect::<Vec<_>>();
    supplied.sort();
    let mut expected = candidates
        .iter()
        .map(|c| c.reference.sort_key())
        .collect::<Vec<_>>();
    expected.sort();
    if supplied != expected {
        return Err(ApplicationError::ParticipantIdentityReviewRequired);
    }
    Ok(())
}
pub(crate) fn kind(value: &str) -> Result<SubjectKind, ApplicationError> {
    match value {
        "natural_person" => Ok(SubjectKind::NaturalPerson),
        "institutional_body" => Ok(SubjectKind::InstitutionalBody),
        _ => Err(inconsistent()),
    }
}
