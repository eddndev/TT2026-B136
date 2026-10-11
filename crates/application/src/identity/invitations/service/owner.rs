use crate::{
    identity::{normalize_email, Principal},
    ApplicationError,
};
use domain::identity::Role;

use super::super::*;
use super::{validation, InvitationService};

impl InvitationService {
    fn owner(&self, token: &str) -> Result<InvitationActor, ApplicationError> {
        let principal = self.ports.identity.authenticate(token)?;
        if principal.role != Role::Owner {
            return Err(ApplicationError::PermissionDenied);
        }
        let user = self
            .ports
            .users
            .find_by_id(principal.id)?
            .ok_or(ApplicationError::InvalidSession)?;
        if !user.active
            || Principal::from(&user) != principal
            || user.revision > i64::MAX as u64
            || user.auth_generation > i64::MAX as u64
        {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(InvitationActor {
            principal,
            revision: user.revision,
            auth_generation: user.auth_generation,
        })
    }

    fn reauthenticate(&self, token: &str, actor: &InvitationActor) -> Result<(), ApplicationError> {
        if self.owner(token)? != *actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }

    fn delivery_token(
        &self,
        id: InvitationId,
        revision: u64,
    ) -> Result<(domain::crypto::Sha256Digest, Vec<u8>), ApplicationError> {
        let token = self.ports.tokens.generate()?;
        let digest = self.token_digest(token.as_ref());
        let protected = self.ports.protector.protect(
            InvitationProtectionContext {
                purpose: InvitationPurpose::Delivery,
                id,
                revision,
            },
            token.as_ref(),
        )?;
        validation::ciphertext(&protected)?;
        Ok((digest, protected))
    }

    pub(super) fn issue_invitation(
        &self,
        token: &str,
        mut request: InvitationIssueRequest,
    ) -> Result<InvitationSummary, ApplicationError> {
        let actor = self.owner(token)?;
        validation::operation(request.operation_id)?;
        validation::role(request.role)?;
        request.email = normalize_email(&request.email)?;
        let expected = request.clone();
        let id = InvitationId::from_uuid(request.operation_id);
        let (token_digest, protected_token) = self.delivery_token(id, 1)?;
        self.reauthenticate(token, &actor)?;
        let result = self.ports.store.issue(
            actor,
            IssueInvitation {
                request,
                token_digest,
                protected_token,
                at: self.ports.clock.now(),
            },
        )?;
        validation::summary(&result)?;
        if result.id != id
            || result.revision != 1
            || result.operation_id != expected.operation_id
            || result.email != expected.email
            || result.role != expected.role
        {
            return Err(validation::stored());
        }
        Ok(result)
    }

    pub(super) fn resend_invitation(
        &self,
        token: &str,
        id: InvitationId,
        request: InvitationResendRequest,
    ) -> Result<InvitationSummary, ApplicationError> {
        let actor = self.owner(token)?;
        validation::identifier(id)?;
        validation::operation(request.operation_id)?;
        if !validation::revision(request.expected_revision)
            || request.expected_revision == i64::MAX as u64
        {
            return Err(ApplicationError::InvalidInput(
                "invitation revision cannot advance".into(),
            ));
        }
        let revision = request.expected_revision + 1;
        let (token_digest, protected_token) = self.delivery_token(id, revision)?;
        self.reauthenticate(token, &actor)?;
        let result = self.ports.store.resend(
            actor,
            id,
            ResendInvitation {
                request,
                token_digest,
                protected_token,
                at: self.ports.clock.now(),
            },
        )?;
        validation::summary(&result)?;
        if result.id != id
            || result.revision != revision
            || result.operation_id != request.operation_id
        {
            return Err(validation::stored());
        }
        Ok(result)
    }

    pub(super) fn list_invitations(
        &self,
        token: &str,
        query: InvitationQuery,
    ) -> Result<InvitationPage, ApplicationError> {
        let actor = self.owner(token)?;
        if !(1..=100).contains(&query.limit) {
            return Err(ApplicationError::InvalidInput(
                "invitation page limit must be 1..100".into(),
            ));
        }
        if let Some(after) = query.after {
            validation::identifier(after)?;
        }
        let result = self.ports.store.list(actor.clone(), query)?;
        if result.items.len() > usize::from(query.limit) {
            return Err(validation::stored());
        }
        let mut previous = query.after;
        for value in &result.items {
            validation::summary(value)?;
            if previous.is_some_and(|id| id.as_uuid() >= value.id.as_uuid()) {
                return Err(validation::stored());
            }
            previous = Some(value.id);
        }
        if result.next.is_some()
            && (result.items.len() != usize::from(query.limit)
                || result.next != result.items.last().map(|value| value.id))
        {
            return Err(validation::stored());
        }
        self.reauthenticate(token, &actor)?;
        Ok(result)
    }

    pub(super) fn get_invitation(
        &self,
        token: &str,
        id: InvitationId,
    ) -> Result<InvitationSummary, ApplicationError> {
        let actor = self.owner(token)?;
        validation::identifier(id)?;
        let result = self.ports.store.get(actor.clone(), id)?;
        validation::summary(&result)?;
        if result.id != id {
            return Err(validation::stored());
        }
        self.reauthenticate(token, &actor)?;
        Ok(result)
    }
}
