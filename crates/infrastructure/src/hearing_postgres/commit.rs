use super::{authorization, inconsistent, port, preparation, write, PostgresHearingStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::case_stages::{StageFormatPolicy, StageSupportSnapshot};
use application::cases::CaseActorSnapshot;
use application::documents::StageSupportReadLimits;
use application::{hearings::*, ApplicationError};
use domain::{cases::CaseId, identity::UserId};
use time::UtcOffset;

impl PostgresHearingStore {
    pub(super) fn commit_change(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedHearingChange,
    ) -> Result<HearingDetail, ApplicationError> {
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
            return Err(HearingError::RevisionConflict.into());
        }
        if observed.participants != prepared.preparation().participants {
            return Err(HearingError::ParticipantChanged.into());
        }
        if observed.records != prepared.preparation().records {
            return Err(HearingError::SupportChanged.into());
        }
        if command.action() != HearingAction::Cancel
            && observed.context != prepared.preparation().context
        {
            return Err(HearingError::ContextConflict.into());
        }
        let admin = observed
            .context
            .administration
            .snapshot()
            .ok_or(HearingError::ContextRequired)?;
        let support = if command.action() == HearingAction::Cancel {
            observed
                .base
                .as_ref()
                .ok_or(HearingError::NotFound)?
                .support
                .clone()
        } else {
            match (observed.records.as_slice(), prepared.formats()) {
                ([], []) => None,
                ([record], [format]) => Some(StageSupportSnapshot {
                    reference: application::documents::DocumentVersionRef {
                        id: record.id,
                        version: record.version,
                    },
                    digest: record.digest,
                    name: record.name.clone(),
                    format: *format,
                    policy: StageFormatPolicy::PdfDocxV1,
                }),
                _ => return Err(inconsistent("hearing admitted support count differs")),
            }
        };
        let at = self.clock.now().to_offset(UtcOffset::UTC);
        let detail = HearingDetail {
            snapshot: HearingSnapshot {
                case_id: case,
                id: command.hearing_id,
                revision: command.result_revision()?,
                values: prepared.values().clone(),
                values_digest: prepared.values_digest(),
                status: if command.action() == HearingAction::Cancel {
                    HearingStatus::Cancelled
                } else {
                    HearingStatus::Scheduled
                },
                reason: command.reason().cloned(),
                receipt: HearingReceipt {
                    operation_id: command.operation_id,
                    action: command.action(),
                    expected_revision: command.expected_revision(),
                    expected_context: command.expected_context(),
                    submission_digest: prepared.submission_digest(),
                },
                scheduling_context: prepared.scheduling_context(),
                recorded_administration_revision: admin.revision,
                recorded_administration_digest: admin.values_digest,
                recorded_at: at,
                recorded_by: CaseActorSnapshot {
                    id: principal.id,
                    email: principal.email.clone(),
                },
            },
            participants: observed.participants,
            support,
        };
        hearing_receipt_matches(self.hasher.as_ref(), &detail)?;
        write::insert(&mut tx, &detail, command)?;
        crate::alerts_postgres::invalidate(
            &mut tx,
            application::alerts::AlertSubject::Hearing {
                case_id: case,
                id: detail.snapshot.id,
            },
            self.hasher.as_ref(),
        )?;
        let action = match command.action() {
            HearingAction::Schedule => "hearing.scheduled",
            HearingAction::Replace => "hearing.replaced",
            HearingAction::Cancel => "hearing.cancelled",
        };
        append_transaction(
            &mut tx,
            &principal.email,
            action,
            &format!(
                "case:{case}:hearing:{}:revision:{}:operation:{}:sha256:{}",
                detail.snapshot.id,
                detail.snapshot.revision.get(),
                command.operation_id,
                detail.snapshot.receipt.submission_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
}
