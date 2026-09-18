use super::{authorization, inconsistent, port, preparation, write, PostgresDeadlineStore};
use application::{cases::CurrentCaseAdministration, deadlines::*, ApplicationError};
use domain::identity::UserId;
use time::UtcOffset;
impl PostgresDeadlineStore {
    pub(super) fn commit_change(
        &self,
        actor: UserId,
        prepared: PreparedDeadlineChange,
    ) -> Result<DeadlineDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let case = prepared.case_id();
        let principal = authorization::actor(&mut tx, actor, case, true, self.hasher.as_ref())?;
        if actor != prepared.actor() {
            return Err(ApplicationError::InvalidSession);
        }
        let command = prepared.command();
        let observed = preparation::load(&mut tx, case, command, self.hasher.as_ref())?;
        if !administration_follows(
            &prepared.preparation().administration,
            &observed.administration,
        ) {
            return Err(DeadlineError::RevisionConflict.into());
        }
        let fresh =
            prepare_deadline_change(self.hasher.as_ref(), actor, case, command.clone(), observed)?;
        if fresh.submission_digest() != prepared.submission_digest()
            || fresh.review_digest() != prepared.review_digest()
        {
            return Err(DeadlineError::SubmissionMismatch.into());
        }
        if matches!(
            command.action(),
            DeadlineAction::SetAttention | DeadlineAction::Retire
        ) && fresh.capture_digest() != prepared.capture_digest()
        {
            return Err(DeadlineError::SubmissionMismatch.into());
        }
        let at = self.clock.now().to_offset(UtcOffset::UTC);
        if !(1..=9999).contains(&at.year()) {
            return Err(inconsistent(
                "deadline capture clock is outside supported years",
            ));
        }
        let detail = DeadlineDetail {
            id: command.deadline_id,
            case_id: case,
            revision: command.result_revision()?,
            definition: fresh.definition().clone(),
            calculation: fresh.calculation().clone(),
            responsible: fresh.responsible().clone(),
            attention: fresh.attention().clone(),
            status: fresh.status(),
            reason: command.reason().cloned(),
            receipt: DeadlineReceipt {
                operation_id: command.operation_id,
                action: command.action(),
                expected_revision: command.expected_revision(),
                review_digest: fresh.review_digest(),
                capture_digest: fresh.capture_digest(),
                submission_digest: fresh.submission_digest(),
            },
            recorded_at: at,
            recorded_by: DeadlineActorSnapshot {
                id: actor,
                email: principal.email.clone(),
            },
        };
        deadline_receipt_matches(self.hasher.as_ref(), &detail)?;
        write::insert(&mut tx, &detail, command, self.hasher.as_ref())?;
        let action = match command.action() {
            DeadlineAction::Register => "deadline.registered",
            DeadlineAction::Correct => "deadline.corrected",
            DeadlineAction::SetAttention => "deadline.attention_recorded",
            DeadlineAction::Retire => "deadline.retired",
        };
        crate::audit_postgres::append_transaction(
            &mut tx,
            &principal.email,
            action,
            &format!(
                "case:{case}:deadline:{}:revision:{}:operation:{}:submission:{}:capture:{}",
                detail.id,
                detail.revision.get(),
                command.operation_id,
                detail.receipt.submission_digest.to_hex(),
                detail.receipt.capture_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
}
fn administration_follows(
    old: &CurrentCaseAdministration,
    new: &CurrentCaseAdministration,
) -> bool {
    match (old, new) {
        (CurrentCaseAdministration::Unrevised(a), CurrentCaseAdministration::Unrevised(b)) => {
            a == b
        }
        (CurrentCaseAdministration::Unrevised(_), CurrentCaseAdministration::Recorded(_)) => true,
        (CurrentCaseAdministration::Recorded(_), CurrentCaseAdministration::Unrevised(_)) => false,
        (CurrentCaseAdministration::Recorded(a), CurrentCaseAdministration::Recorded(b)) => {
            b.revision > a.revision || (a == b && a.changed_at.offset() == b.changed_at.offset())
        }
    }
}
