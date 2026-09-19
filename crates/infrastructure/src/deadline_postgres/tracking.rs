use super::{administration, inconsistent};
use application::{
    deadline_inputs::{DeadlineCalendarRef, DeadlineInputHeads},
    deadline_observations::verify_captured_deadline_observations,
    deadline_profiles::{DeadlineProfileId, DeadlineProfileRevision},
    deadline_reevaluation::{
        decode_observations, DependencyFamily, ObservationEntry, ObservationRole, Observations,
    },
    deadlines::*,
    procedural_facts::{FactRevision, FactTarget, ResolutionId},
    ApplicationError,
};
use domain::{
    crypto::DocumentHasher,
    deadline_triggers::TriggerSourceRef,
    hearing_results::{HearingResultId, HearingResultRevision},
    hearings::HearingId,
    judicial_calendars::{JudicialCalendarId, JudicialCalendarRevision},
    procedural_facts::{FactHearingRef, FactResolutionRef, NotificationId},
};
use postgres::{Row, Transaction};

pub(super) fn capture(
    tx: &mut Transaction<'_>,
    row: &Row,
    detail: &DeadlineDetail,
    hasher: &dyn DocumentHasher,
) -> Result<Option<DeadlineTrackingCapture>, ApplicationError> {
    let tracking: Option<&[u8]> = row.try_get("tracking_canonical").map_err(inconsistent)?;
    let observations: Option<&[u8]> = row
        .try_get("observations_canonical")
        .map_err(inconsistent)?;
    let admin_revision: Option<i64> = row
        .try_get("tracking_administration_revision")
        .map_err(inconsistent)?;
    let (tracking, observations, receipt) = match (&detail.receipt.version, tracking, observations)
    {
        (DeadlineReceiptVersion::Legacy, None, None) if admin_revision.is_none() => {
            return Ok(None)
        }
        (DeadlineReceiptVersion::Tracked(receipt), Some(tracking), Some(observations)) => {
            (tracking, observations, receipt)
        }
        _ => {
            return Err(inconsistent(
                "deadline receipt version and tracking columns disagree",
            ))
        }
    };
    let metadata = decode_deadline_tracking_capture(tracking).map_err(inconsistent)?;
    let revision = metadata.administration_revision();
    if admin_revision != revision.map(|revision| i64::from(revision.get()))
        || hasher.hash_bytes(observations) != receipt.observations_digest
    {
        return Err(inconsistent(
            "deadline tracking projection or observation digest differs",
        ));
    }
    let observations = decode_observations(observations).map_err(inconsistent)?;
    if observations.case_id != detail.case_id {
        return Err(inconsistent("deadline observations belong to another case"));
    }
    let administration = administration::at_revision(tx, detail.case_id, revision, hasher)?;
    let profile = entry(&observations, ObservationRole::Profile)
        .ok_or_else(|| inconsistent("deadline observed profile is absent"))?;
    let profile_head = crate::deadline_profile_postgres::storage::detail(
        tx,
        DeadlineProfileId::from_uuid(profile.id),
        Some(DeadlineProfileRevision::new(profile.revision).map_err(inconsistent)?),
        hasher,
    )
    .map_err(inconsistent)?;
    let heads = heads(&observations)?;
    let input = &detail.definition.input;
    let material = crate::deadline_input_history::load_captured_material(
        tx,
        detail.calculation.profile.definition.trigger(),
        &input.selection,
        input.calendar,
        &administration,
        &heads,
        hasher,
    )
    .map_err(inconsistent)?;
    let parent = entry(&observations, ObservationRole::NotificationParent)
        .map(|parent| {
            crate::procedural_fact_postgres::storage::detail(
                tx,
                detail.case_id,
                FactTarget::Resolution(ResolutionId::from_uuid(parent.id)),
                Some(FactRevision::new(parent.revision).map_err(inconsistent)?),
                hasher,
            )
            .map_err(inconsistent)
        })
        .transpose()?;
    verify_captured_deadline_observations(
        hasher,
        &observations,
        &profile_head,
        &material,
        parent.as_ref(),
    )
    .map_err(inconsistent)?;
    metadata
        .restore(hasher, observations, administration)
        .map(Some)
        .map_err(inconsistent)
}

fn entry(observations: &Observations, role: ObservationRole) -> Option<&ObservationEntry> {
    observations.entries.iter().find(|entry| entry.role == role)
}

fn heads(observations: &Observations) -> Result<DeadlineInputHeads, ApplicationError> {
    let source = entry(observations, ObservationRole::Source)
        .map(source)
        .transpose()?;
    let calendar = entry(observations, ObservationRole::Calendar)
        .map(|entry| {
            Ok::<_, ApplicationError>(DeadlineCalendarRef {
                id: JudicialCalendarId::from_uuid(entry.id),
                revision: JudicialCalendarRevision::new(entry.revision).map_err(inconsistent)?,
            })
        })
        .transpose()?;
    Ok(DeadlineInputHeads { source, calendar })
}

fn source(entry: &ObservationEntry) -> Result<TriggerSourceRef, ApplicationError> {
    match entry.family {
        DependencyFamily::Resolution => Ok(TriggerSourceRef::Resolution(FactResolutionRef {
            id: ResolutionId::from_uuid(entry.id),
            revision: FactRevision::new(entry.revision).map_err(inconsistent)?,
        })),
        DependencyFamily::Notification => {
            let parent = entry
                .parent_resolution
                .ok_or_else(|| inconsistent("observed notification parent reference is absent"))?;
            Ok(TriggerSourceRef::Notification {
                id: NotificationId::from_uuid(entry.id),
                revision: FactRevision::new(entry.revision).map_err(inconsistent)?,
                resolution: FactResolutionRef {
                    id: ResolutionId::from_uuid(parent.id),
                    revision: FactRevision::new(parent.revision).map_err(inconsistent)?,
                },
            })
        }
        DependencyFamily::HearingResult => Ok(TriggerSourceRef::HearingResult(FactHearingRef {
            hearing_id: HearingId::from_uuid(
                entry
                    .hearing_id
                    .ok_or_else(|| inconsistent("observed hearing is absent"))?,
            ),
            result_id: HearingResultId::from_uuid(entry.id),
            revision: HearingResultRevision::new(entry.revision).map_err(inconsistent)?,
            agreement_id: None,
        })),
        _ => Err(inconsistent("unsupported observed deadline source family")),
    }
}
