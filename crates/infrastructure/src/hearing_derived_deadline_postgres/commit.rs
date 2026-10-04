use super::*;
use crate::audit_postgres::begin_audited;
use application::{deadlines::*, documents::StageSupportReadLimits};

impl HearingDerivedDeadlineStore for PostgresHearingDerivedDeadlineStore {
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        command: &HearingDerivedDeadlineCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<HearingDerivedDeadlinePreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case)?;
        let result = match replay::load(&mut tx, &principal, case, command)? {
            Some(record) => HearingDerivedDeadlinePreparation::Replay(Box::new(record)),
            None => HearingDerivedDeadlinePreparation::Ready(Box::new(preparation::load(
                &mut tx,
                principal,
                case,
                command,
                limits,
                self.hasher.as_ref(),
            )?)),
        };
        tx.rollback().map_err(port)?;
        Ok(result)
    }

    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedHearingDerivedDeadline,
    ) -> Result<HearingDerivedDeadlineRecord, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case)?;
        let draft = prepared.draft();
        let command = draft.command();
        if let Some(record) = replay::load(&mut tx, &principal, case, command)? {
            if record.evidence().review_digest != draft.review_digest() {
                return Err(DeadlineError::SubmissionMismatch.into());
            }
            tx.rollback().map_err(port)?;
            return Ok(record);
        }
        let observed = preparation::load(
            &mut tx,
            principal.clone(),
            case,
            command,
            &StageSupportReadLimits::standard(),
            self.hasher.as_ref(),
        )?;
        let material = draft.material();
        if principal != *draft.actor()
            || observed.result != *prepared.result().preparation()
            || observed.profile != material.profile
            || observed.profile_head != material.profile_head
            || observed.calendar != material.calendar
            || observed.calendar_head != material.calendar_head
            || observed.responsible != material.responsible
        {
            return Err(DeadlineError::SubmissionMismatch.into());
        }
        let result = crate::hearing_result_postgres::commit::insert_prepared(
            &mut tx,
            &principal,
            case,
            prepared.result(),
            self.hasher.as_ref(),
            self.clock.as_ref(),
        )?;
        let snapshot = &result.snapshot;
        let sequence: i64 = tx.query_one("SELECT sequence FROM deadline_source_events WHERE source_kind='hearing_result' AND source_id=$1 AND revision=1 AND case_id=$2 AND hearing_id=$3 AND operation_id=$4", &[&snapshot.id.as_uuid(),&case.as_uuid(),&snapshot.hearing_id.as_uuid(),&snapshot.receipt.operation_id.as_uuid()]).map_err(port)?.get(0);
        let event = crate::hearing_derived_deadline_schema::history::event(&mut tx, sequence)?;
        let creation =
            finalize_hearing_derived_deadline(self.hasher.as_ref(), draft, result, event)?;
        // Resolve the actual persisted source through the ordinary deadline path
        // before accepting the finalizer's prospective-to-captured transition.
        let (deadline_command, policies) = command.deadline.clone().into_parts();
        let deadline_inputs = crate::deadline_postgres::preparation::load(
            &mut tx,
            case,
            &deadline_command,
            self.hasher.as_ref(),
        )?;
        let ordinary = prepare_tracked_deadline_change(
            self.hasher.as_ref(),
            creation.deadline().recorded_by.clone(),
            case,
            deadline_command,
            deadline_inputs,
            policies,
            None,
        )?;
        if ordinary.receipt() != creation.deadline().receipt
            || ordinary.calculation() != &creation.deadline().calculation
            || ordinary.tracking() != creation.deadline().tracking.as_ref()
        {
            return Err(DeadlineError::SubmissionMismatch.into());
        }
        let record = restore_hearing_derived_deadline(self.hasher.as_ref(), creation.evidence())?;
        write::insert(&mut tx, &record)?;
        tx.commit().map_err(port)?;
        Ok(record)
    }
}
