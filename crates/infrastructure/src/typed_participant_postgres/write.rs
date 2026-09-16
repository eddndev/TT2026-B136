use super::*;
pub(super) fn subject(
    tx: &mut impl GenericClient,
    s: &SubjectSnapshot,
    review: &IdentityReviewSubmission,
) -> Result<(), ApplicationError> {
    if s.revision.get() == 1 {
        tx.execute(
            "INSERT INTO case_subjects(id,case_id) VALUES($1,$2)",
            &[&s.id.as_uuid(), &s.case_id.as_uuid()],
        )
        .map_err(port)?;
    }
    let v = &s.values;
    let support = v.identity_support();
    let known = matches!(
        v,
        SubjectValues::InstitutionalBody { .. }
            | SubjectValues::NaturalPerson {
                name: RepresentedName::Known(_),
                ..
            }
    );
    let kind = match v.kind() {
        SubjectKind::NaturalPerson => "natural_person",
        SubjectKind::InstitutionalBody => "institutional_body",
    };
    tx.execute("INSERT INTO case_subject_revisions(subject_id,revision,values_canonical,values_digest,subject_kind,display_name,name_known,declared_identifier,identity_document_id,identity_document_version,identity_document_digest,identity_document_locator,changed_at,changed_by,changed_by_email) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",&[&s.id.as_uuid(),&i64::from(s.revision.get()),&v.canonical_bytes(),&s.values_digest.as_bytes().as_slice(),&kind,&v.display_name(),&known,&v.declared_identifier(),&support.reference().id.as_uuid(),&i64::from(support.reference().version.get()),&support.digest().as_bytes().as_slice(),&support.locator(),&canonical_time(s.changed_at)?,&s.changed_by.id.as_uuid(),&s.changed_by.email]).map_err(port)?;
    let bytes = review.canonical_bytes()?;
    tx.execute("INSERT INTO subject_identity_reviews(subject_id,revision,review_canonical,review_digest) VALUES($1,$2,$3,sha256($3))",&[&s.id.as_uuid(),&i64::from(s.revision.get()),&bytes]).map_err(port)?;
    Ok(())
}
pub(super) fn typed(
    tx: &mut impl GenericClient,
    s: &TypedParticipantSnapshot,
) -> Result<(), ApplicationError> {
    let v = &s.values;
    let origin = s
        .credential_origin
        .as_ref()
        .map(|o| i64::from(o.participant_revision.get()));
    tx.execute("INSERT INTO case_participant_typed_revisions(participant_id,revision,values_canonical,values_digest,subject_id,subject_revision,role_kind,organization,directory_status,changed_at,changed_by,changed_by_email,submission_digest,submission_revision,credential_origin_revision) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",&[&s.id.as_uuid(),&i64::from(s.revision.get()),&v.canonical_bytes(),&s.values_digest.as_bytes().as_slice(),&v.subject().id.as_uuid(),&i64::from(v.subject().revision.get()),&v.kind().as_str(),&v.role().organization(),&v.directory_status().as_str(),&canonical_time(s.changed_at)?,&s.changed_by.id.as_uuid(),&s.changed_by.email,&s.submission_digest.as_bytes().as_slice(),&i64::from(s.submission_revision.get()),&origin]).map_err(port)?;
    Ok(())
}
pub(super) fn review(
    tx: &mut impl GenericClient,
    case: CaseId,
    prepared: &PreparedTypedParticipantChange,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let request = prepared.request();
    let p = &request.proposal;
    let bytes = request.review.canonical_bytes()?;
    let proof = prepared
        .check()
        .map(|c| (c.statement_digest, c.certificate.fingerprint, &c.signature));
    let submission = participant_submission_bytes(hasher, case, request, proof)?;
    if hasher.hash_bytes(&submission) != prepared.submission_digest() {
        return Err(inconsistent());
    }
    tx.execute("INSERT INTO participant_identity_reviews(participant_id,revision,review_canonical,review_digest,submission_canonical,submission_digest) VALUES($1,$2,$3,sha256($3),$4,sha256($4))",&[&p.participant_id().as_uuid(),&i64::from(p.expected_participant().next().ok_or(ApplicationError::ParticipantRevisionExhausted)?.get()),&bytes,&submission]).map_err(port)?;
    Ok(())
}
pub(super) fn credential(
    tx: &mut impl GenericClient,
    e: &ParticipantCredentialEvidence,
) -> Result<(), ApplicationError> {
    let c = &e.check;
    tx.execute("INSERT INTO participant_credential_evidence(participant_id,revision,deployment_id,trust_revision,declaration,statement_digest,certificate_der,certificate_fingerprint,signature,checked_at,valid_from,valid_until,accepted_at_seconds,accepted_at_nanoseconds) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",&[&e.reference.participant_id.as_uuid(),&i64::from(e.reference.participant_revision.get()),&e.trust.deployment_id,&i64::from(e.trust.revision.get()),&e.declaration,&c.statement_digest.as_bytes().as_slice(),&c.certificate.der,&c.certificate.fingerprint.as_bytes().as_slice(),&c.signature.as_bytes(),&c.checked_at,&c.valid_from,&c.valid_until,&e.accepted_at.unix_timestamp(),&(e.accepted_at.nanosecond() as i32)]).map_err(port)?;
    Ok(())
}
