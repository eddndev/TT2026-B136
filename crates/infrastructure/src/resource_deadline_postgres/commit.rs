use super::{port, preparation, replay, PostgresResourceDeadlineStore};
use crate::{
    audit_postgres::{append_transaction, begin_audited},
    procedural_resource_postgres::authorize,
};
use application::{resource_activities::*, resource_deadlines::*, ApplicationError};
use domain::{cases::CaseId, identity::UserId};
use time::UtcOffset;
impl PostgresResourceDeadlineStore {
    pub(super) fn prepare_change(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        command: &ResourceDeadlineCommand,
    ) -> Result<ResourceDeadlinePreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, true)?;
        let result = match replay::load(
            &mut tx,
            &principal,
            case,
            resource,
            command,
            self.hasher.as_ref(),
        )? {
            Some(result) => ResourceDeadlinePreparation::Replay(Box::new(result)),
            None => ResourceDeadlinePreparation::Ready(Box::new(preparation::load(
                &mut tx,
                case,
                resource,
                command,
                self.hasher.as_ref(),
            )?)),
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "resource_deadline.prepare",
            &format!(
                "case:{case}:resource:{resource}:association:{}:operation:{}",
                command.association_id,
                command.deadline.clone().into_parts().0.operation_id
            ),
            self.clock.now().to_offset(UtcOffset::UTC),
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
    pub(super) fn commit_change(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        prepared: PreparedResourceDeadline,
    ) -> Result<ResourceDeadlineResult, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, true)?;
        if let Some(result) = replay::load(
            &mut tx,
            &principal,
            case,
            resource,
            prepared.command(),
            self.hasher.as_ref(),
        )? {
            if result.submission_digest != prepared.draft().submission_digest {
                return Err(ResourceActivityError::OperationConflict.into());
            }
            append_transaction(
                &mut tx,
                &principal.email,
                "resource_deadline.replay",
                &replay::marker(&result),
                self.clock.now().to_offset(UtcOffset::UTC),
            )?;
            tx.commit().map_err(port)?;
            return Ok(result);
        }
        let material = preparation::load(
            &mut tx,
            case,
            resource,
            prepared.command(),
            self.hasher.as_ref(),
        )?;
        let fresh = prepare_resource_deadline_change(
            self.hasher.clone(),
            &principal,
            case,
            resource,
            prepared.command().clone(),
            material,
        )?;
        if fresh.material() != prepared.material() || fresh.draft() != prepared.draft() {
            return Err(ResourceActivityError::SubmissionMismatch.into());
        }
        let at = self.clock.now().to_offset(UtcOffset::UTC);
        let result = fresh.into_result(at)?;
        let deadline = &result.deadline;
        crate::deadline_postgres::write::insert(&mut tx, deadline, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &principal.email,
            "deadline.registered",
            &format!(
                "case:{case}:deadline:{}:revision:{}:operation:{}:submission:{}:capture:{}",
                deadline.id,
                deadline.revision.get(),
                deadline.receipt.operation_id,
                deadline.receipt.submission_digest.to_hex(),
                deadline.receipt.capture_digest.to_hex()
            ),
            at,
        )?;
        let association = &result.association;
        crate::resource_activity_postgres::write::insert(
            &mut tx,
            association,
            self.hasher.as_ref(),
        )?;
        append_transaction(
            &mut tx,
            &principal.email,
            "resource_activity.link",
            &crate::resource_activity_postgres::write::resource(association),
            at,
        )?;
        append_transaction(
            &mut tx,
            &principal.email,
            "resource_deadline.registered",
            &replay::marker(&result),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}
