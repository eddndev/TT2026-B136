use super::*;
impl PostgresTypedParticipantStore {
    pub(super) fn commit_role(
        &self,
        actor_id: UserId,
        case: CaseId,
        prepared: PreparedTypedParticipantChange,
    ) -> Result<ParticipantDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor_id, case, true)?;
        let observed = preparation::participant(
            &mut tx,
            case,
            prepared.request(),
            &StageSupportReadLimits::standard(),
            self.hasher.as_ref(),
            prepared.check().map(|c| c.certificate.fingerprint),
        )?;
        if observed.records != prepared.records() {
            return Err(ApplicationError::ParticipantSupportChanged);
        }
        if observed.trust.as_ref() != prepared.trust() {
            return Err(ApplicationError::CredentialTrustChanged);
        }
        let at = self.clock.now().to_offset(UtcOffset::UTC);
        let p = &prepared.request().proposal;
        let rev = p
            .expected_participant()
            .next()
            .ok_or(ApplicationError::ParticipantRevisionExhausted)?;
        let credential = match (prepared.check(), prepared.trust(), prepared.declaration()) {
            (Some(check), Some(trust), Some(bytes)) => {
                if at.unix_timestamp() < check.valid_from
                    || at.unix_timestamp() > check.valid_until
                    || at.unix_timestamp() < check.checked_at
                {
                    return Err(ApplicationError::CredentialTrustChanged);
                }
                Some(ParticipantCredentialEvidence {
                    case_id: case,
                    reference: ParticipantCredentialRef {
                        participant_id: p.participant_id(),
                        participant_revision: rev,
                        statement_digest: check.statement_digest,
                    },
                    subject: p.values().subject(),
                    declaration: bytes.to_vec(),
                    check: check.clone(),
                    trust: trust.clone(),
                    accepted_at: at,
                    accepted_by: actor(&principal),
                })
            }
            (None, None, None) => None,
            _ => return Err(inconsistent()),
        };
        let bound = match p.subject() {
            SubjectChange::Keep(reference) => storage::subject(
                &mut tx,
                case,
                reference.id,
                Some(reference.revision),
                self.hasher.as_ref(),
            )?,
            SubjectChange::Append {
                id,
                expected,
                values,
            } => {
                let snapshot = SubjectSnapshot {
                    case_id: case,
                    id: *id,
                    revision: expected
                        .next()
                        .ok_or(ApplicationError::SubjectRevisionExhausted)?,
                    values: values.clone(),
                    values_digest: subject_digest(self.hasher.as_ref(), values),
                    changed_at: at,
                    changed_by: actor(&principal),
                };
                write::subject(&mut tx, &snapshot, &prepared.request().review)?;
                audit(
                    &mut tx,
                    &principal,
                    if snapshot.revision.get() == 1 {
                        "participant.subject_created"
                    } else {
                        "participant.subject_replaced"
                    },
                    &format!(
                        "case:{case}:subject:{id}:revision:{}:sha256:{}",
                        snapshot.revision.get(),
                        snapshot.values_digest.to_hex()
                    ),
                    at,
                )?;
                snapshot
            }
        };
        if p.expected_participant() == ParticipantExpectation::Absent {
            tx.execute(
                "INSERT INTO case_participants(id,case_id) VALUES($1,$2)",
                &[&p.participant_id().as_uuid(), &case.as_uuid()],
            )
            .map_err(port)?;
        }
        let snapshot = TypedParticipantSnapshot {
            case_id: case,
            id: p.participant_id(),
            revision: rev,
            values: p.values().clone(),
            values_digest: typed_participant_digest(self.hasher.as_ref(), p.values()),
            changed_at: at,
            changed_by: actor(&principal),
            credential_origin: credential.as_ref().map(|e| e.reference.clone()),
            submission_digest: prepared.submission_digest(),
            submission_revision: rev,
        };
        write::typed(&mut tx, &snapshot)?;
        write::review(&mut tx, case, &prepared, self.hasher.as_ref())?;
        audit(
            &mut tx,
            &principal,
            "participant.identity_reviewed",
            &format!(
                "case:{case}:participant:{}:revision:{}:sha256:{}",
                snapshot.id,
                rev.get(),
                participant_review_digest(self.hasher.as_ref(), &prepared.request().review)?
                    .to_hex()
            ),
            at,
        )?;
        if let Some(evidence) = credential {
            write::credential(&mut tx, &evidence)?;
            audit(
                &mut tx,
                &principal,
                "participant.credential_accepted",
                &format!(
                    "case:{case}:participant:{}:revision:{}:sha256:{}",
                    snapshot.id,
                    rev.get(),
                    evidence.reference.statement_digest.to_hex()
                ),
                at,
            )?;
        }
        audit(
            &mut tx,
            &principal,
            if snapshot.revision.get() == 1 {
                "participant.typed_created"
            } else {
                "participant.typed_replaced"
            },
            &format!(
                "case:{case}:participant:{}:revision:{}:sha256:{}",
                snapshot.id,
                rev.get(),
                snapshot.values_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(ParticipantDetail {
            revision: ParticipantRevisionSnapshot::Typed(Box::new(snapshot)),
            bound_subject: Some(bound),
        })
    }
    pub(super) fn commit_identity(
        &self,
        actor_id: UserId,
        case: CaseId,
        prepared: PreparedSubjectChange,
    ) -> Result<SubjectSnapshot, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor_id, case, true)?;
        let observed = preparation::subject(
            &mut tx,
            case,
            prepared.request(),
            &StageSupportReadLimits::standard(),
            self.hasher.as_ref(),
        )?;
        if observed.records != prepared.records() {
            return Err(ApplicationError::ParticipantSupportChanged);
        }
        let at = self.clock.now().to_offset(UtcOffset::UTC);
        let request = prepared.request();
        let snapshot = SubjectSnapshot {
            case_id: case,
            id: request.id,
            revision: request
                .expected_revision
                .next()
                .ok_or(ApplicationError::SubjectRevisionExhausted)?,
            values: request.values.clone(),
            values_digest: subject_digest(self.hasher.as_ref(), &request.values),
            changed_at: at,
            changed_by: actor(&principal),
        };
        write::subject(&mut tx, &snapshot, &request.review)?;
        audit(
            &mut tx,
            &principal,
            if snapshot.revision.get() == 1 {
                "participant.subject_created"
            } else {
                "participant.subject_replaced"
            },
            &format!(
                "case:{case}:subject:{}:revision:{}:sha256:{}",
                snapshot.id,
                snapshot.revision.get(),
                snapshot.values_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(snapshot)
    }
}
