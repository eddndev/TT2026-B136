use super::values::{invalid_state, nonnegative, storage_error, timestamp, value};
use super::PostgresPasswordResetRepository;
use crate::audit_postgres::{append_transaction, begin_audited};
use application::identity::password_reset::{
    CompleteReset, IssueReset, PasswordResetRepository, ResetCandidate, ResetCompletion, ResetId,
    ResetIssue, ResetIssueOutcome,
};
use application::ApplicationError;
use domain::crypto::Sha256Digest;
use domain::identity::UserId;
use uuid::Uuid;

impl PasswordResetRepository for PostgresPasswordResetRepository {
    fn issue(&self, command: IssueReset) -> Result<ResetIssueOutcome, ApplicationError> {
        let id = Uuid::new_v4();
        let ttl = i64::try_from(command.policy.ttl_seconds()).map_err(|_| invalid_state())?;
        let capacity = i64::from(command.policy.max_pending_per_account());
        let mut client = self.lock()?;
        let mut transaction = begin_audited(&mut client).map_err(|_| storage_error())?;
        let query = format!(
            "SELECT id,email,(extract(epoch FROM expires_at)*1000000)::bigint AS expiry_micros
             FROM {}.password_reset_issue($1,$2,$3,$4,$5)",
            self.schema,
        );
        let row = transaction
            .query_opt(
                &query,
                &[
                    &id,
                    &command.email,
                    &&command.digest.as_bytes()[..],
                    &ttl,
                    &capacity,
                ],
            )
            .map_err(|_| storage_error())?;
        let Some(row) = row else {
            transaction.rollback().map_err(|_| storage_error())?;
            return Ok(ResetIssueOutcome::Ignored);
        };
        let stored_id: Uuid = value(&row, "id")?;
        let email: String = value(&row, "email")?;
        if stored_id != id || stored_id.is_nil() || email != command.email {
            return Err(invalid_state());
        }
        let issued = ResetIssue {
            id: ResetId::from_uuid(stored_id),
            email,
            expires_at: timestamp(&row, "expiry_micros")?,
        };
        transaction.commit().map_err(|_| storage_error())?;
        Ok(ResetIssueOutcome::Issued(issued))
    }

    fn inspect(&self, digest: Sha256Digest) -> Result<Option<ResetCandidate>, ApplicationError> {
        let query = format!(
            "SELECT id,user_id,auth_generation FROM {}.password_reset_inspect($1)",
            self.schema,
        );
        let row = self
            .lock()?
            .query_opt(&query, &[&&digest.as_bytes()[..]])
            .map_err(|_| storage_error())?;
        let Some(row) = row else { return Ok(None) };
        let id: Uuid = value(&row, "id")?;
        let user_id: Uuid = value(&row, "user_id")?;
        if id.is_nil() || user_id.is_nil() {
            return Err(invalid_state());
        }
        Ok(Some(ResetCandidate {
            id: ResetId::from_uuid(id),
            user_id: UserId::from_uuid(user_id),
            auth_generation: nonnegative(&row, "auth_generation")?,
        }))
    }

    fn complete(&self, command: CompleteReset) -> Result<ResetCompletion, ApplicationError> {
        let Ok(generation) = i64::try_from(command.candidate.auth_generation) else {
            return Ok(ResetCompletion::Rejected);
        };
        if command.candidate.id.as_uuid().is_nil() || command.candidate.user_id.as_uuid().is_nil() {
            return Ok(ResetCompletion::Rejected);
        }
        let mut client = self.lock()?;
        let mut transaction = begin_audited(&mut client).map_err(|_| storage_error())?;
        let query = format!(
            "SELECT (extract(epoch FROM reset_at)*1000000)::bigint AS reset_micros,
             next_audit,new_revision,new_generation
             FROM {}.password_reset_consume($1,$2,$3,$4,$5)",
            self.schema,
        );
        let row = transaction
            .query_opt(
                &query,
                &[
                    &command.candidate.id.as_uuid(),
                    &&command.digest.as_bytes()[..],
                    &command.candidate.user_id.as_uuid(),
                    &generation,
                    &command.password_hash,
                ],
            )
            .map_err(|_| storage_error())?;
        let Some(row) = row else {
            transaction.rollback().map_err(|_| storage_error())?;
            return Ok(ResetCompletion::Rejected);
        };
        let sequence = nonnegative(&row, "next_audit")?;
        let revision = nonnegative(&row, "new_revision")?;
        let next_generation = nonnegative(&row, "new_generation")?;
        if revision == 0
            || next_generation == 0
            || next_generation > revision
            || command.candidate.auth_generation.checked_add(1) != Some(next_generation)
        {
            return Err(invalid_state());
        }
        let resource = format!(
            "user:{}:revision:{revision}:generation:{next_generation}",
            command.candidate.user_id,
        );
        let event = append_transaction(
            &mut transaction,
            "password-reset",
            "identity.password_reset",
            &resource,
            timestamp(&row, "reset_micros")?,
        )
        .map_err(|_| storage_error())?;
        if event.event.sequence != sequence {
            return Err(invalid_state());
        }
        transaction.commit().map_err(|_| storage_error())?;
        Ok(ResetCompletion::Changed)
    }

    fn cancel_undelivered(
        &self,
        id: ResetId,
        digest: Sha256Digest,
    ) -> Result<(), ApplicationError> {
        let mut client = self.lock()?;
        let mut transaction = begin_audited(&mut client).map_err(|_| storage_error())?;
        let query = format!("SELECT {}.password_reset_cancel($1,$2)", self.schema);
        transaction
            .query_one(&query, &[&id.as_uuid(), &&digest.as_bytes()[..]])
            .map_err(|_| storage_error())?;
        transaction.commit().map_err(|_| storage_error())
    }
}
