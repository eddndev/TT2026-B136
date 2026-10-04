use super::{port, preparation, replay, write, PostgresResourceHearingStore};
use crate::{
    audit_postgres::{append_transaction, begin_audited},
    procedural_resource_postgres::authorize,
};
use application::{resource_activities::*, resource_hearings::*, ApplicationError};
use domain::{cases::CaseId, identity::UserId};
impl PostgresResourceHearingStore {
    pub(super) fn prepare_change(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        command: &ResourceHearingCommand,
    ) -> Result<ResourceHearingPreparation, ApplicationError> {
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
            Some(result) => ResourceHearingPreparation::Replay(Box::new(result)),
            None => ResourceHearingPreparation::Ready(Box::new(preparation::load(
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
            "resource_hearing.prepare",
            &format!(
                "case:{case}:resource:{resource}:operation:{}",
                command.operation_id
            ),
            self.clock.now().to_offset(time::UtcOffset::UTC),
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
    pub(super) fn commit_change(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        prepared: PreparedResourceHearing,
    ) -> Result<ResourceHearingCreation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, true)?;
        if *prepared.actor() != principal {
            return Err(ApplicationError::InvalidSession);
        }
        let command = &prepared.draft().command;
        if let Some(result) = replay::load(
            &mut tx,
            &principal,
            case,
            resource,
            command,
            self.hasher.as_ref(),
        )? {
            if result.hearing.review != *prepared.draft() {
                return Err(ResourceActivityError::OperationConflict.into());
            }
            append_transaction(
                &mut tx,
                &principal.email,
                "resource_hearing.replay",
                &replay::marker(&result),
                self.clock.now().to_offset(time::UtcOffset::UTC),
            )?;
            tx.commit().map_err(port)?;
            return Ok(result);
        }
        let material = preparation::load(&mut tx, case, resource, command, self.hasher.as_ref())?;
        let fresh = prepare_resource_hearing_change(
            self.hasher.clone(),
            &principal,
            case,
            resource,
            command.clone(),
            material,
        )?;
        if fresh.draft() != prepared.draft() || fresh.material() != prepared.material() {
            return Err(ResourceActivityError::SubmissionMismatch.into());
        }
        let at = self.clock.now().to_offset(time::UtcOffset::UTC);
        let result = fresh.into_creation(at)?;
        write::insert(&mut tx, &result.hearing, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &principal.email,
            "resource_hearing.scheduled",
            &replay::marker(&result),
            at,
        )?;
        crate::resource_activity_postgres::write::insert(
            &mut tx,
            &result.association,
            self.hasher.as_ref(),
        )?;
        append_transaction(
            &mut tx,
            &principal.email,
            "resource_activity.link",
            &crate::resource_activity_postgres::write::resource(&result.association),
            at,
        )?;
        append_transaction(
            &mut tx,
            &principal.email,
            "resource_hearing.registered",
            &replay::marker(&result),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}
