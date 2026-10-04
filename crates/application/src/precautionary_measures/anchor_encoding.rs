use super::{
    decision_wire::{blob, bounded, support, timestamp},
    MeasureDecisionAnchorMaterial, MeasureDecisionAnchorRef,
};
use crate::{
    hearings::{HearingDetail, HearingParticipantSnapshot},
    precautionary_hearings::precautionary_hearing_capture_bytes,
    ApplicationError,
};
use domain::{case_stages::CaseStage, hearings::HearingStatus, participants::DirectoryStatus};

pub(super) fn reference(bytes: &mut Vec<u8>, anchor: Option<&MeasureDecisionAnchorRef>) {
    match anchor {
        None => bytes.push(0),
        Some(MeasureDecisionAnchorRef::Initial {
            hearing_id,
            revision,
            values_digest,
            submission_digest,
        }) => {
            bytes.push(1);
            bytes.extend_from_slice(hearing_id.as_uuid().as_bytes());
            bytes.extend_from_slice(&revision.get().to_be_bytes());
            bytes.extend_from_slice(values_digest.as_bytes());
            bytes.extend_from_slice(submission_digest.as_bytes());
        }
        Some(MeasureDecisionAnchorRef::Precautionary {
            hearing_id,
            revision,
            capture_digest,
        }) => {
            bytes.push(2);
            bytes.extend_from_slice(hearing_id.as_uuid().as_bytes());
            bytes.extend_from_slice(&revision.get().to_be_bytes());
            bytes.extend_from_slice(capture_digest.as_bytes());
        }
    }
}

pub(super) fn material(
    bytes: &mut Vec<u8>,
    anchor: Option<&MeasureDecisionAnchorMaterial>,
) -> Result<(), ApplicationError> {
    match anchor {
        None => bytes.push(0),
        Some(MeasureDecisionAnchorMaterial::Initial(detail)) => {
            bytes.push(1);
            blob(bytes, &initial(detail)?)?;
        }
        Some(MeasureDecisionAnchorMaterial::Precautionary(capture)) => {
            bytes.push(2);
            blob(bytes, &precautionary_hearing_capture_bytes(capture)?)?;
            bytes.extend_from_slice(capture.capture_digest.as_bytes());
        }
    }
    Ok(())
}

fn initial(detail: &HearingDetail) -> Result<Vec<u8>, ApplicationError> {
    bounded(detail.participants.len())?;
    let mut bytes = b"MHIA1".to_vec();
    let snapshot = &detail.snapshot;
    bytes.extend_from_slice(snapshot.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(snapshot.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&snapshot.revision.get().to_be_bytes());
    blob(&mut bytes, &snapshot.values.canonical_bytes())?;
    bytes.extend_from_slice(snapshot.values_digest.as_bytes());
    bytes.push(match snapshot.status {
        HearingStatus::Scheduled => 0,
        HearingStatus::Cancelled => 1,
    });
    bytes.push(u8::from(snapshot.reason.is_some()));
    if let Some(reason) = &snapshot.reason {
        blob(&mut bytes, reason.as_str().as_bytes())?;
    }
    let receipt = &snapshot.receipt;
    bytes.extend_from_slice(receipt.operation_id.as_uuid().as_bytes());
    bytes.push(receipt.action.tag());
    bytes.extend_from_slice(&receipt.expected_revision.to_be_bytes());
    bytes.push(u8::from(receipt.expected_context.is_some()));
    if let Some(context) = receipt.expected_context {
        bytes.extend_from_slice(&context.case_revision.get().to_be_bytes());
        bytes.extend_from_slice(&context.stage_revision.get().to_be_bytes());
    }
    bytes.extend_from_slice(receipt.submission_digest.as_bytes());
    let context = snapshot.scheduling_context;
    bytes.extend_from_slice(&context.administration_revision.get().to_be_bytes());
    bytes.extend_from_slice(context.administration_digest.as_bytes());
    bytes.extend_from_slice(&context.stage_revision.get().to_be_bytes());
    bytes.push(match context.stage {
        CaseStage::Investigation => 0,
        CaseStage::Intermediate => 1,
        CaseStage::Trial => 2,
    });
    bytes.push(u8::from(context.stage_digest.is_some()));
    if let Some(digest) = context.stage_digest {
        bytes.extend_from_slice(digest.as_bytes());
    }
    bytes.extend_from_slice(
        &snapshot
            .recorded_administration_revision
            .get()
            .to_be_bytes(),
    );
    bytes.extend_from_slice(snapshot.recorded_administration_digest.as_bytes());
    timestamp(&mut bytes, snapshot.recorded_at);
    bytes.extend_from_slice(snapshot.recorded_by.id.as_uuid().as_bytes());
    blob(&mut bytes, snapshot.recorded_by.email.as_bytes())?;
    bytes.extend_from_slice(&(detail.participants.len() as u32).to_be_bytes());
    for item in &detail.participants {
        participant(&mut bytes, item)?;
    }
    bytes.push(u8::from(detail.support.is_some()));
    if let Some(value) = &detail.support {
        support(&mut bytes, value)?;
    }
    Ok(bytes)
}

fn participant(
    bytes: &mut Vec<u8>,
    participant: &HearingParticipantSnapshot,
) -> Result<(), ApplicationError> {
    let overview = &participant.overview;
    bytes.extend_from_slice(overview.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(overview.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&overview.revision.get().to_be_bytes());
    blob(bytes, overview.display_name.as_bytes())?;
    blob(bytes, overview.procedural_role.as_bytes())?;
    bytes.push(u8::from(overview.organization.is_some()));
    if let Some(organization) = &overview.organization {
        blob(bytes, organization.as_bytes())?;
    }
    bytes.push(match overview.directory_status {
        DirectoryStatus::Active => 0,
        DirectoryStatus::Archived => 1,
    });
    bytes.push(u8::from(overview.kind.is_some()));
    if let Some(kind) = overview.kind {
        bytes.push(kind.tag());
    }
    bytes.push(u8::from(overview.subject.is_some()));
    if let Some(subject) = overview.subject {
        bytes.extend_from_slice(subject.id.as_uuid().as_bytes());
        bytes.extend_from_slice(&subject.revision.get().to_be_bytes());
        bytes.extend_from_slice(subject.values_digest.as_bytes());
    }
    bytes.extend_from_slice(participant.values_digest.as_bytes());
    Ok(())
}
