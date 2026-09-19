use super::{
    authorization, inconsistent, port, preparation, write, PostgresProceduralResourceStore,
};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{documents::StageSupportReadLimits, procedural_resources::*, ApplicationError};
use domain::{cases::CaseId, identity::UserId};
use time::UtcOffset;

impl PostgresProceduralResourceStore {
    pub(super) fn prepare_change(
        &self,
        actor: UserId,
        case: CaseId,
        command: &ResourceCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<ResourcePreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, true)?;
        let result = if let Some(detail) =
            preparation::replay(&mut tx, actor, case, command, self.hasher.as_ref())?
        {
            ResourcePreparation::Replay(Box::new(detail))
        } else {
            ResourcePreparation::Ready(Box::new(preparation::load(
                &mut tx,
                case,
                command,
                limits,
                self.hasher.as_ref(),
            )?))
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "procedural_resource.prepare",
            &format!(
                "case:{case}:resource:{}:operation:{}",
                command.resource_id, command.operation_id
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
        prepared: PreparedResourceChange,
    ) -> Result<ResourceDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, true)?;
        if let Some(detail) = preparation::replay(
            &mut tx,
            actor,
            case,
            prepared.command(),
            self.hasher.as_ref(),
        )? {
            if detail.receipt.submission_digest != prepared.draft().submission_digest {
                return Err(ProceduralResourceError::OperationConflict.into());
            }
            append_transaction(
                &mut tx,
                &principal.email,
                "procedural_resource.replay",
                &write::resource(&detail),
                self.clock.now().to_offset(UtcOffset::UTC),
            )?;
            tx.commit().map_err(port)?;
            return Ok(detail);
        }
        let material = preparation::load(
            &mut tx,
            case,
            prepared.command(),
            &StageSupportReadLimits::standard(),
            self.hasher.as_ref(),
        )?;
        if material.administration != prepared.material().administration
            || material.stage != prepared.material().stage
        {
            return Err(ProceduralResourceError::RevisionConflict.into());
        }
        if material != *prepared.material() {
            return Err(ApplicationError::StageSupportChanged);
        }
        if prepared.draft().case_id != case
            || prepared.draft().recorded_by.id != actor
            || prepared.draft().recorded_by.email != principal.email
        {
            return Err(inconsistent(
                "resource prepared author differs from current account",
            ));
        }
        if self.hasher.hash_bytes(&resource_submission_bytes(
            self.hasher.as_ref(),
            prepared.draft(),
        )?) != prepared.draft().submission_digest
        {
            return Err(ProceduralResourceError::SubmissionMismatch.into());
        }
        let at = self.clock.now().to_offset(UtcOffset::UTC);
        let detail = prepared.into_detail(at)?;
        resource_receipt_matches(self.hasher.as_ref(), &detail)?;
        write::insert(&mut tx, &detail, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &principal.email,
            &format!(
                "procedural_resource.{}",
                write::action(detail.receipt.action)
            ),
            &write::resource(&detail),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
}
