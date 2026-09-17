use super::{authorization, inconsistent, port, preparation, write, PostgresDeadlineProfileStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{deadline_profiles::*, ApplicationError};
use domain::identity::UserId;
use time::UtcOffset;
impl PostgresDeadlineProfileStore {
    pub(super) fn commit_change(
        &self,
        actor: UserId,
        prepared: PreparedDeadlineProfileChange,
    ) -> Result<DeadlineProfileDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::actor(
            &mut tx,
            actor,
            prepared.collection(),
            true,
            self.hasher.as_ref(),
        )?;
        if actor != prepared.actor() {
            return Err(ApplicationError::InvalidSession);
        }
        let command = prepared.command();
        let observed = preparation::load(
            &mut tx,
            prepared.collection(),
            command,
            self.hasher.as_ref(),
        )?;
        if observed != *prepared.preparation() {
            return Err(DeadlineProfileError::RevisionConflict.into());
        }
        let (definition, algorithm) = match &command.change {
            DeadlineProfileChange::Publish { definition }
            | DeadlineProfileChange::Replace { definition, .. } => {
                (definition, DeadlineProfileAlgorithm::V1)
            }
            DeadlineProfileChange::Retire { .. } => {
                let base = observed
                    .base
                    .as_ref()
                    .ok_or(DeadlineProfileError::NotFound)?;
                (&base.definition, base.algorithm)
            }
        };
        if definition != prepared.definition()
            || algorithm != prepared.algorithm()
            || deadline_profile_definition_digest(self.hasher.as_ref(), definition)
                != prepared.definition_digest()
            || deadline_profile_submission_digest(
                self.hasher.as_ref(),
                actor,
                command,
                algorithm,
                prepared.definition_digest(),
            ) != prepared.submission_digest()
        {
            return Err(inconsistent(
                "prepared profile differs from its locked sources",
            ));
        }
        let at = self.clock.now().to_offset(UtcOffset::UTC);
        if !(1..=9999).contains(&at.year()) {
            return Err(inconsistent(
                "profile capture clock is outside supported years",
            ));
        }
        let detail = DeadlineProfileDetail {
            id: command.profile_id,
            revision: command.result_revision()?,
            definition: definition.clone(),
            definition_digest: prepared.definition_digest(),
            algorithm,
            status: command.result_status(),
            reason: command.reason().cloned(),
            receipt: DeadlineProfileReceipt {
                operation_id: command.operation_id,
                action: command.action(),
                expected_revision: command.expected_revision(),
                submission_digest: prepared.submission_digest(),
            },
            recorded_at: at,
            recorded_by: DeadlineProfileActorSnapshot {
                id: principal.id,
                email: principal.email.clone(),
            },
        };
        deadline_profile_receipt_matches(self.hasher.as_ref(), &detail)?;
        write::insert(&mut tx, &detail, command)?;
        let action = match command.action() {
            DeadlineProfileAction::Publish => "deadline_profile.published",
            DeadlineProfileAction::Replace => "deadline_profile.replaced",
            DeadlineProfileAction::Retire => "deadline_profile.retired",
        };
        append_transaction(
            &mut tx,
            &principal.email,
            action,
            &format!(
                "profile:{}:revision:{}:operation:{}:sha256:{}",
                detail.id,
                detail.revision.get(),
                command.operation_id,
                detail.receipt.submission_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
}
