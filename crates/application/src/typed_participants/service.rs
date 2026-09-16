use super::*;
use crate::{
    documents::{DocumentProcessor, StageDocumentFormat},
    identity::IdentityWorkflow,
};
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::{
        CredentialCertificate, CredentialFailure, DocumentHasher, InternalDeclarationVerifier,
    },
    identity::{Permission, UserId},
};
use std::sync::Arc;

pub struct TypedParticipantService {
    pub(super) identity: Arc<dyn IdentityWorkflow>,
    pub(super) store: Arc<dyn TypedParticipantStore>,
    pub(super) processor: Arc<DocumentProcessor>,
    pub(super) hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub(super) formats: Arc<dyn DocumentFormatBatchValidator>,
    pub(super) credentials: Arc<dyn InternalDeclarationVerifier>,
    pub(super) clock: Arc<dyn Clock + Send + Sync>,
    pub(super) limits: StageSupportReadLimits,
}
impl TypedParticipantService {
    pub fn new(
        identity: Arc<dyn IdentityWorkflow>,
        store: Arc<dyn TypedParticipantStore>,
        processor: Arc<DocumentProcessor>,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        formats: Arc<dyn DocumentFormatBatchValidator>,
        credentials: Arc<dyn InternalDeclarationVerifier>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            identity,
            store,
            processor,
            hasher,
            formats,
            credentials,
            clock,
            limits: StageSupportReadLimits::standard(),
        }
    }
    pub(super) fn actor(
        &self,
        token: &str,
        permission: Permission,
    ) -> Result<UserId, ApplicationError> {
        let principal = self.identity.authenticate(token)?;
        if !principal.role.allows(permission) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(principal.id)
    }
    pub(super) fn same_actor(&self, token: &str, actor: UserId) -> Result<(), ApplicationError> {
        if self.actor(token, Permission::ManageParticipant)? != actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
    pub(super) fn inspect(
        &self,
        certificate: Option<&[u8]>,
    ) -> Result<Option<CredentialCertificate>, ApplicationError> {
        certificate
            .map(|bytes| {
                if bytes.len() > 16 * 1024 {
                    return Err(ApplicationError::ParticipantCredentialRejected(
                        CredentialFailure::LimitExceeded,
                    ));
                }
                self.credentials
                    .inspect_certificate(bytes)
                    .map_err(Into::into)
            })
            .transpose()
    }
    pub(super) fn prepare(
        &self,
        actor: UserId,
        case_id: CaseId,
        request: &ParticipantPreparationRequest,
    ) -> Result<
        (
            TypedParticipantPreparation,
            Vec<StageDocumentFormat>,
            Option<CredentialCertificate>,
        ),
        ApplicationError,
    > {
        request.review.canonical_bytes()?;
        let certificate = self.inspect(request.certificate.as_deref())?;
        let required = request
            .proposal
            .values()
            .role()
            .profile()
            .requires_credential();
        if required && certificate.is_none() {
            return Err(ApplicationError::ParticipantCredentialRequired);
        }
        let mut preparation =
            self.store
                .prepare_participant(actor, case_id, request, &self.limits)?;
        if preparation.directory_stamp != request.review.directory_stamp {
            return Err(ApplicationError::ParticipantIdentityReviewConflict);
        }
        let subject = &preparation.subject_values;
        if subject_digest(self.hasher.as_ref(), subject)
            != request.proposal.values().subject().values_digest
        {
            return Err(ApplicationError::SubjectRevisionConflict);
        }
        if let SubjectChange::Append { values, .. } = request.proposal.subject() {
            if values != subject {
                return Err(ApplicationError::SubjectRevisionConflict);
            }
        }
        if !request
            .proposal
            .values()
            .kind()
            .accepts_subject(subject.kind())
        {
            return Err(domain::DomainError::ParticipantSubjectKindMismatch.into());
        }
        if certificate.is_some() && subject.kind() == SubjectKind::InstitutionalBody {
            return Err(ApplicationError::ParticipantCredentialUnexpected);
        }
        let refs = super::validation::participant_supports(request, subject)?;
        preparation.records = super::validation::ordered_records(&refs, preparation.records)?;
        let formats = self.processor.validate_support_batch(
            &preparation.records,
            &self.limits,
            self.formats.as_ref(),
        )?;
        Ok((preparation, formats, certificate))
    }
    pub(super) fn declaration(
        &self,
        case_id: CaseId,
        proposal: &ParticipantProposal,
        trust: Option<&CredentialTrustSnapshot>,
        certificate: Option<CredentialCertificate>,
    ) -> Result<Option<ParticipantDeclaration>, ApplicationError> {
        let Some(certificate) = certificate else {
            return Ok(None);
        };
        let trust = trust.ok_or(ApplicationError::CredentialTrustUnavailable)?;
        let bytes = participant_declaration_bytes(
            case_id,
            proposal,
            trust,
            certificate.fingerprint,
            self.hasher.as_ref(),
        );
        Ok(Some(ParticipantDeclaration {
            digest: self.hasher.hash_bytes(&bytes),
            bytes,
            certificate,
            deployment_id: trust.deployment_id,
            trust_revision: trust.revision,
            root_fingerprint: trust.inspection.root_fingerprint,
            participant_values_digest: typed_participant_digest(
                self.hasher.as_ref(),
                proposal.values(),
            ),
        }))
    }
}
