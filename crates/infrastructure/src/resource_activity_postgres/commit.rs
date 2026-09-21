use super::{authorize, inconsistent, port, preparation, write, PostgresResourceActivityStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{resource_activities::*, ApplicationError};
use domain::{cases::CaseId, identity::UserId};
use time::UtcOffset;

impl PostgresResourceActivityStore {
    pub(super) fn prepare_change(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        command: &ResourceActivityCommand,
    ) -> Result<ResourceActivityPreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, true)?;
        let result = match preparation::replay(
            &mut tx,
            actor,
            case,
            resource,
            command,
            self.hasher.as_ref(),
        )? {
            Some(detail) => ResourceActivityPreparation::Replay(Box::new(detail)),
            None => ResourceActivityPreparation::Ready(Box::new(preparation::load(
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
            "resource_activity.prepare",
            &format!(
                "case:{case}:resource:{resource}:association:{}:operation:{}",
                command.association_id, command.operation_id
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
        prepared: PreparedResourceActivityChange,
    ) -> Result<ResourceActivityDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, true)?;
        if let Some(detail) = preparation::replay(
            &mut tx,
            actor,
            case,
            resource,
            prepared.command(),
            self.hasher.as_ref(),
        )? {
            if detail.receipt.submission_digest != prepared.draft().submission_digest {
                return Err(ResourceActivityError::OperationConflict.into());
            }
            append_transaction(
                &mut tx,
                &principal.email,
                "resource_activity.replay",
                &write::resource(&detail),
                self.clock.now().to_offset(UtcOffset::UTC),
            )?;
            tx.commit().map_err(port)?;
            return Ok(detail);
        }
        let material = preparation::load(
            &mut tx,
            case,
            resource,
            prepared.command(),
            self.hasher.as_ref(),
        )?;
        if material != *prepared.material() {
            return Err(ResourceActivityError::RevisionConflict.into());
        }
        let draft = prepared.draft();
        if draft.case_id != case
            || draft.resource_id != resource
            || draft.recorded_by.id != actor
            || draft.recorded_by.email != principal.email
        {
            return Err(inconsistent(
                "association prepared scope or current author differs",
            ));
        }
        if self.hasher.hash_bytes(&resource_activity_submission_bytes(
            self.hasher.as_ref(),
            draft,
        )?) != draft.submission_digest
        {
            return Err(ResourceActivityError::SubmissionMismatch.into());
        }
        let at = self.clock.now().to_offset(UtcOffset::UTC);
        let detail = prepared.into_detail(at)?;
        resource_activity_receipt_matches(self.hasher.as_ref(), &detail)?;
        write::insert(&mut tx, &detail, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &principal.email,
            &format!("resource_activity.{}", detail.receipt.action.as_str()),
            &write::resource(&detail),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
}
