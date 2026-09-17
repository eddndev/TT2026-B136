use super::{
    authorization, inconsistent, port, preparation, sources, write, PostgresProceduralFactStore,
};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{
    cases::CaseActorSnapshot, documents::StageSupportReadLimits, procedural_facts::*,
    ApplicationError,
};
use domain::{cases::CaseId, identity::UserId};
use time::UtcOffset;

impl PostgresProceduralFactStore {
    pub(super) fn commit_change(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedFactChange,
    ) -> Result<FactDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, true)?;
        let command = prepared.command();
        let observed = preparation::load(
            &mut tx,
            case,
            command,
            &StageSupportReadLimits::standard(),
            self.hasher.as_ref(),
        )?;
        if observed.base != prepared.preparation().base {
            return Err(ProceduralFactError::RevisionConflict.into());
        }
        if observed.source_material != prepared.preparation().source_material {
            return Err(inconsistent(
                "immutable fact material changed after admission",
            ));
        }
        if observed.records != prepared.preparation().records {
            return Err(ProceduralFactError::SupportChanged.into());
        }
        validate_fact_administration(
            self.hasher.as_ref(),
            case,
            &observed.observed_administration,
            Some(&prepared.preparation().observed_administration),
        )?;
        if fact_submission_digest(
            self.hasher.as_ref(),
            actor,
            case,
            command,
            prepared.values_digest(),
            prepared.sources_digest(),
        )? != prepared.submission_digest()
        {
            return Err(ProceduralFactError::SubmissionMismatch.into());
        }
        let at = self.clock.now().to_offset(UtcOffset::UTC);
        if !(1..=9999).contains(&at.year()) {
            return Err(inconsistent(
                "fact capture clock is outside supported years",
            ));
        }
        let metadata = FactRevisionMetadata {
            revision: command.result_revision()?,
            values_digest: prepared.values_digest(),
            status: command.action().resulting_status(),
            reason: command.reason().cloned(),
            receipt: FactReceipt {
                operation_id: command.operation_id(),
                action: command.action(),
                expected_revision: command.expected_revision(),
                sources_digest: prepared.sources_digest(),
                submission_digest: prepared.submission_digest(),
            },
            recorded_administration: observed.observed_administration,
            recorded_at: at,
            recorded_by: CaseActorSnapshot {
                id: principal.id,
                email: principal.email.clone(),
            },
        };
        let snapshot = match (command.target(), prepared.values()) {
            (FactTarget::Resolution(id), ProceduralFactValues::Resolution(values)) => {
                ProceduralFactSnapshot::Resolution(Box::new(ResolutionSnapshot {
                    root: ResolutionRoot::new(id, case),
                    metadata,
                    values: *values.clone(),
                }))
            }
            (
                FactTarget::Notification { id, resolution_id },
                ProceduralFactValues::Notification(values),
            ) => ProceduralFactSnapshot::Notification(Box::new(NotificationSnapshot {
                root: NotificationRoot::new(id, case, resolution_id),
                metadata,
                values: *values.clone(),
            })),
            _ => return Err(inconsistent("prepared fact family differs")),
        };
        let detail = FactDetail {
            snapshot,
            sources: prepared.sources().clone(),
        };
        fact_receipt_matches(self.hasher.as_ref(), &detail)?;
        sources::validate(&mut tx, &detail, self.hasher.as_ref())?;
        if let Some(base) = &observed.base {
            validate_fact_retained_sources(&base.sources, &detail.sources)?;
            if command.action() == FactAction::Withdraw
                && (base.snapshot.values() != detail.snapshot.values()
                    || base.sources != detail.sources)
            {
                return Err(inconsistent("withdrawal changed its prepared base"));
            }
        }
        write::insert(&mut tx, &detail, command)?;
        let action = match command.action() {
            FactAction::Record => "procedural_fact.recorded",
            FactAction::Correct => "procedural_fact.corrected",
            FactAction::Withdraw => "procedural_fact.withdrawn",
        };
        append_transaction(
            &mut tx,
            &principal.email,
            action,
            &write::resource(&detail.snapshot),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
}
