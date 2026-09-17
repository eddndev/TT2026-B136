use super::{authorization, inconsistent, port, preparation, write, PostgresHearingResultStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{
    case_stages::{StageFormatPolicy, StageSupportSnapshot},
    cases::CaseActorSnapshot,
    documents::StageSupportReadLimits,
    hearing_results::*,
    ApplicationError,
};
use domain::{cases::CaseId, identity::UserId};
use time::UtcOffset;

impl PostgresHearingResultStore {
    pub(super) fn commit_change(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedHearingResultChange,
    ) -> Result<HearingResultDetail, ApplicationError> {
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
            return Err(HearingResultError::RevisionConflict.into());
        }
        if observed.anchor != prepared.preparation().anchor
            || observed.continuation != prepared.preparation().continuation
            || observed.attendees != prepared.preparation().attendees
        {
            return Err(inconsistent(
                "immutable result sources changed after preparation",
            ));
        }
        if observed.records != prepared.preparation().records {
            return Err(HearingResultError::SupportChanged.into());
        }
        let administration = observed
            .administration
            .snapshot()
            .ok_or_else(|| inconsistent("result has no captured administration"))?;
        let support = if command.action() == HearingResultAction::Withdraw {
            observed
                .base
                .as_ref()
                .ok_or(HearingResultError::NotFound)?
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
                _ => return Err(inconsistent("result admitted support count differs")),
            }
        };
        let at = self.clock.now().to_offset(UtcOffset::UTC);
        if !(1..=9999).contains(&at.year()) {
            return Err(inconsistent(
                "result capture clock is outside supported years",
            ));
        }
        if command.action() != HearingResultAction::Withdraw
            && prepared.values().event_time().lower_bound() > at
        {
            return Err(HearingResultError::FutureTime.into());
        }
        let detail = HearingResultDetail {
            snapshot: HearingResultSnapshot {
                case_id: case,
                hearing_id: command.hearing_id,
                id: command.result_id,
                revision: command.result_revision()?,
                values: prepared.values().clone(),
                values_digest: prepared.values_digest(),
                status: if command.action() == HearingResultAction::Withdraw {
                    HearingResultStatus::Withdrawn
                } else {
                    HearingResultStatus::Recorded
                },
                reason: command.reason().cloned(),
                receipt: HearingResultReceipt {
                    operation_id: command.operation_id,
                    action: command.action(),
                    expected_revision: command.expected_revision(),
                    submission_digest: prepared.submission_digest(),
                },
                anchor: prepared.anchor(),
                continuation: prepared.continuation(),
                recorded_administration_revision: administration.revision,
                recorded_administration_digest: administration.values_digest,
                recorded_at: at,
                recorded_by: CaseActorSnapshot {
                    id: principal.id,
                    email: principal.email.clone(),
                },
            },
            anchor: HearingResultAnchorSnapshot::from(&observed.anchor),
            continuation: observed
                .continuation
                .as_ref()
                .map(HearingResultContinuationSnapshot::from),
            attendees: observed.attendees,
            support,
        };
        hearing_result_receipt_matches(self.hasher.as_ref(), &detail)?;
        write::insert(&mut tx, &detail, command)?;
        let action = match command.action() {
            HearingResultAction::Record => "hearing_result.recorded",
            HearingResultAction::Correct => "hearing_result.corrected",
            HearingResultAction::Withdraw => "hearing_result.withdrawn",
        };
        append_transaction(
            &mut tx,
            &principal.email,
            action,
            &format!(
                "case:{case}:hearing:{}:result:{}:revision:{}:operation:{}:sha256:{}",
                command.hearing_id,
                command.result_id,
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
