use super::{authorization, port, preparation, storage, stored, PostgresDeadlineStore};
use application::{
    deadline_currentness::{evaluate_deadline_currentness, DeadlineCurrent},
    deadline_technical::DeadlineReevaluationInputs,
    deadline_tracking::DeadlineReviewState,
    deadlines::{DeadlineDetail, DeadlineError, DeadlineId, DeadlineStatus},
    ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher, identity::UserId};
use postgres::Transaction;

impl PostgresDeadlineStore {
    pub(super) fn current_detail(
        &self,
        actor: UserId,
        case: CaseId,
        id: DeadlineId,
    ) -> Result<DeadlineCurrent, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, case, false, self.hasher.as_ref())?;
        if !authorization::visible(&mut tx, case, id)? {
            return Err(DeadlineError::NotFound.into());
        }
        let detail = storage::detail(&mut tx, case, id, None, self.hasher.as_ref())?;
        let current = self.current_projection(&mut tx, &detail)?;
        crate::audit_postgres::append_transaction(
            &mut tx,
            &principal.email,
            "deadline.current_read",
            &format!(
                "case:{case}:deadline:{id}:revision:{}:capture:{}",
                detail.revision.get(),
                detail.receipt.capture_digest.to_hex()
            ),
            self.clock.now(),
        )?;
        tx.commit().map_err(port)?;
        Ok(current)
    }

    /// The caller owns case authorization and the common audited transaction lock.
    pub(super) fn current_projection(
        &self,
        tx: &mut Transaction<'_>,
        detail: &DeadlineDetail,
    ) -> Result<DeadlineCurrent, ApplicationError> {
        current_in_transaction(tx, detail, self.hasher.as_ref(), self.clock.now())
    }
}

/// Resolve current evidence inside the caller's authorized audited transaction.
pub(crate) fn current_in_transaction(
    tx: &mut Transaction<'_>,
    detail: &DeadlineDetail,
    hasher: &dyn DocumentHasher,
    checked_at: OffsetDateTime,
) -> Result<DeadlineCurrent, ApplicationError> {
    let inputs = if detail.status == DeadlineStatus::Retired
        || detail.review_state() == DeadlineReviewState::LegacyUndeclared
    {
        None
    } else {
        let definition = &detail.definition;
        Some(DeadlineReevaluationInputs {
            profile_head: crate::deadline_profile_postgres::storage::detail(
                tx,
                definition.profile.id,
                None,
                hasher,
            )
            .map_err(stored)?,
            material: crate::deadline_input_postgres::load_material(
                tx,
                &definition.input.selection,
                definition.input.calendar,
                hasher,
            )
            .map_err(stored)?,
            notification_parent_head: preparation::notification_parent_for_definition(
                tx,
                detail.case_id,
                definition,
                hasher,
            )
            .map_err(stored)?,
        })
    };
    evaluate_deadline_currentness(hasher, detail, inputs.as_ref(), checked_at).map_err(stored)
}
