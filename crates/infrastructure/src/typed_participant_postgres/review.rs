use super::*;
impl PostgresTypedParticipantStore {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn review_role(
        &self,
        actor_id: UserId,
        case: CaseId,
        request: ParticipantProposalRequest,
        subject_id: CaseSubjectId,
        participant_id: ParticipantId,
        certificate: Option<domain::crypto::CredentialCertificate>,
        at: OffsetDateTime,
    ) -> Result<ParticipantProposalReview, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor_id, case, true)?;
        let (subject, values) = match request.subject {
            SubjectDraftSelection::Create(values) => (
                SubjectChange::Append {
                    id: subject_id,
                    expected: SubjectExpectation::Absent,
                    values: values.clone(),
                },
                values,
            ),
            SubjectDraftSelection::Keep(reference) => {
                let current =
                    storage::subject(&mut tx, case, reference.id, None, self.hasher.as_ref())?;
                if current.revision != reference.revision
                    || current.values_digest != reference.values_digest
                {
                    return Err(ApplicationError::SubjectRevisionConflict);
                }
                (SubjectChange::Keep(reference), current.values)
            }
            SubjectDraftSelection::Replace { current, values } => {
                let observed =
                    storage::subject(&mut tx, case, current.id, None, self.hasher.as_ref())?;
                if observed.revision != current.revision
                    || observed.values_digest != current.values_digest
                {
                    return Err(ApplicationError::SubjectRevisionConflict);
                }
                if observed.values.kind() != values.kind() {
                    return Err(ApplicationError::ParticipantSubjectChangeForbidden);
                }
                (
                    SubjectChange::Append {
                        id: current.id,
                        expected: SubjectExpectation::Revision(current.revision),
                        values: values.clone(),
                    },
                    values,
                )
            }
        };
        let (id, expected, status) = match request.participant {
            ParticipantDraftTarget::Create => (
                participant_id,
                ParticipantExpectation::Absent,
                DirectoryStatus::Active,
            ),
            ParticipantDraftTarget::Existing {
                id,
                expected_revision,
            } => {
                let current = storage::head(&mut tx, case, id, self.hasher.as_ref())?
                    .ok_or(ApplicationError::ParticipantNotFound)?;
                if current.revision != expected_revision {
                    return Err(ApplicationError::ParticipantRevisionConflict);
                }
                if current.subject.is_some_and(|s| s != subject.id()) {
                    return Err(ApplicationError::ParticipantSubjectChangeForbidden);
                }
                (
                    id,
                    ParticipantExpectation::Revision(expected_revision),
                    current.status,
                )
            }
        };
        if !request.role.kind().accepts_subject(values.kind()) {
            return Err(ApplicationError::ParticipantSubjectChangeForbidden);
        }
        let reference = match &subject {
            SubjectChange::Keep(r) => *r,
            SubjectChange::Append { id, expected, .. } => SubjectRevisionRef {
                id: *id,
                revision: expected
                    .next()
                    .ok_or(ApplicationError::SubjectRevisionExhausted)?,
                values_digest: subject_digest(self.hasher.as_ref(), &values),
            },
        };
        let proposal = ParticipantProposal::new(
            subject,
            id,
            expected,
            TypedParticipantValues::new(reference, status, request.role),
        )?;
        let stamp = directory::stamp(&mut tx, case, self.hasher.as_ref())?;
        let candidates = candidates::find(
            &mut tx,
            case,
            &values,
            Some(reference.id),
            Some(id),
            certificate.map(|c| c.fingerprint),
        )?;
        audit(
            &mut tx,
            &principal,
            "participant.identity_reviewed",
            &format!("case:{case}:participant:{id}"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(ParticipantProposalReview {
            proposal,
            directory_stamp: stamp,
            candidates,
        })
    }
    pub(super) fn review_identity(
        &self,
        actor_id: UserId,
        case: CaseId,
        id: CaseSubjectId,
        expected: SubjectRevision,
        values: SubjectValues,
        at: OffsetDateTime,
    ) -> Result<SubjectReplacementReview, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor_id, case, true)?;
        let current = storage::subject(&mut tx, case, id, None, self.hasher.as_ref())?;
        if current.revision != expected {
            return Err(ApplicationError::SubjectRevisionConflict);
        }
        if current.values.kind() != values.kind() {
            return Err(ApplicationError::ParticipantSubjectChangeForbidden);
        }
        let stamp = directory::stamp(&mut tx, case, self.hasher.as_ref())?;
        let candidates = candidates::find(&mut tx, case, &values, Some(id), None, None)?;
        audit(
            &mut tx,
            &principal,
            "participant.subject_identity_reviewed",
            &format!("case:{case}:subject:{id}"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(SubjectReplacementReview {
            id,
            expected_revision: expected,
            values,
            directory_stamp: stamp,
            candidates,
        })
    }
}
