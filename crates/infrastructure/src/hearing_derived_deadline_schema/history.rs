use super::{incomplete, port};
use application::{
    deadline_profiles::{DeadlineProfileId, DeadlineProfileRevision},
    deadline_reevaluation::{DependencyFamily, ObservationRole, SourceEventReference},
    deadlines::*,
    hearing_derived_deadlines::*,
    hearing_results::*,
    identity::Principal,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::Sha256Digest, hearings::HearingId, identity::UserId};
use postgres::{Row, Transaction};

/// Rebuild only the exact captured revisions. Current roles and dependency heads
/// never replace the authority or evidence bound by the original review.
pub(super) fn restore(
    tx: &mut Transaction<'_>,
    row: &Row,
) -> Result<HearingDerivedDeadlineRecord, ApplicationError> {
    let hasher = &crate::RingSha256Hasher;
    let case = CaseId::from_uuid(row.get("case_id"));
    let result = crate::hearing_result_postgres::storage::detail(
        tx,
        case,
        HearingId::from_uuid(row.get("hearing_id")),
        HearingResultId::from_uuid(row.get("result_id")),
        Some(HearingResultRevision::initial()),
        hasher,
    )?;
    let deadline = crate::deadline_postgres::storage::detail(
        tx,
        case,
        DeadlineId::from_uuid(row.get("deadline_id")),
        Some(DeadlineRevision::initial()),
        hasher,
    )?;
    let actor = Principal {
        id: UserId::from_uuid(row.get("actor_id")),
        email: row.get("actor_email"),
        role: row
            .get::<_, String>("actor_role")
            .parse()
            .map_err(|_| incomplete())?,
    };
    let source = &result.snapshot;
    if row.get::<_, i64>("result_revision") != 1
        || row.get::<_, i64>("deadline_revision") != 1
        || row.get::<_, uuid::Uuid>("operation_id") != source.receipt.operation_id.as_uuid()
        || row.get::<_, uuid::Uuid>("deadline_operation_id")
            != deadline.receipt.operation_id.as_uuid()
    {
        return Err(incomplete());
    }
    let result_command = HearingResultCommand {
        operation_id: source.receipt.operation_id,
        hearing_id: source.hearing_id,
        result_id: source.id,
        change: HearingResultChange::Record {
            anchor_revision: source.anchor.revision,
            continuation: source
                .continuation
                .map(|value| HearingResultContinuationRef::new(value.result_id, value.revision)),
            values: source.values.clone(),
        },
    };
    let result_draft = HearingResultDraft {
        case_id: case,
        actor: actor.id,
        command: result_command.clone(),
        result_revision: source.revision,
        values: source.values.clone(),
        values_digest: source.values_digest,
        submission_digest: source.receipt.submission_digest,
        anchor: result.anchor,
        continuation: result.continuation,
        observed_administration: deadline.calculation.material.administration.clone(),
        attendees: result.attendees.clone(),
        support: result.support.clone(),
    };
    let tracking = deadline.tracking.as_ref().ok_or_else(incomplete)?;
    let profile = tracking
        .observations
        .entries
        .iter()
        .find(|entry| entry.role == ObservationRole::Profile)
        .ok_or_else(incomplete)?;
    let profile_head = crate::deadline_profile_postgres::storage::detail(
        tx,
        DeadlineProfileId::from_uuid(profile.id),
        Some(DeadlineProfileRevision::new(profile.revision).map_err(|_| incomplete())?),
        hasher,
    )?;
    let command = HearingDerivedDeadlineCommand {
        result: result_command,
        deadline: DeadlineHumanCommand::new(
            DeadlineCommand {
                operation_id: deadline.receipt.operation_id,
                deadline_id: deadline.id,
                change: DeadlineChange::Register {
                    definition: deadline.definition.clone(),
                },
            },
            Some(tracking.policies),
        )?,
    };
    let material = HearingDerivedDeadlineMaterial {
        result: result_draft,
        profile: deadline.calculation.profile.clone(),
        profile_head,
        calendar: deadline.calculation.material.calendar.clone(),
        calendar_head: deadline.calculation.material.calendar_head.clone(),
        responsible: deadline.responsible.clone(),
    };
    let evidence = HearingDerivedDeadlineEvidence {
        actor,
        command,
        material,
        result,
        deadline,
        source_event: event(tx, row.get("source_event_sequence"))?,
        review_digest: digest(row, "review_digest")?,
        capture_digest: digest(row, "capture_digest")?,
    };
    let record = restore_hearing_derived_deadline(hasher, evidence)?;
    if record.review_bytes() != row.get::<_, &[u8]>("review_canonical")
        || record.capture_bytes() != row.get::<_, &[u8]>("capture_canonical")
    {
        return Err(incomplete());
    }
    Ok(record)
}

fn event(
    tx: &mut Transaction<'_>,
    sequence: i64,
) -> Result<SourceEventReference, ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT source_kind,source_id,revision,case_id,hearing_id,operation_id
        FROM deadline_source_events WHERE sequence=$1",
            &[&sequence],
        )
        .map_err(port)?
        .ok_or_else(incomplete)?;
    if sequence <= 0 || row.get::<_, String>("source_kind") != "hearing_result" {
        return Err(incomplete());
    }
    Ok(SourceEventReference {
        sequence: u64::try_from(sequence).map_err(|_| incomplete())?,
        family: DependencyFamily::HearingResult,
        source_id: row.get("source_id"),
        revision: u32::try_from(row.get::<_, i64>("revision")).map_err(|_| incomplete())?,
        case_id: row
            .get::<_, Option<uuid::Uuid>>("case_id")
            .map(CaseId::from_uuid),
        hearing_id: row.get("hearing_id"),
        operation_id: row.get("operation_id"),
    })
}

fn digest(row: &Row, name: &str) -> Result<Sha256Digest, ApplicationError> {
    let value: &[u8] = row.get(name);
    Ok(Sha256Digest::from_array(
        value.try_into().map_err(|_| incomplete())?,
    ))
}
