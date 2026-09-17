use super::evidence::{actor, instant, optional, text};
use crate::{
    case_stages::StageSupportSnapshot,
    hearing_results::{HearingResultAnchor, HearingResultContinuation, HearingResultDetail},
    hearings::HearingParticipantSnapshot,
};

pub(super) fn hearing(bytes: &mut Vec<u8>, detail: &HearingResultDetail) {
    let snapshot = &detail.snapshot;
    bytes.extend_from_slice(snapshot.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(snapshot.hearing_id.as_uuid().as_bytes());
    bytes.extend_from_slice(snapshot.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&snapshot.revision.get().to_be_bytes());
    bytes.extend_from_slice(snapshot.values_digest.as_bytes());
    bytes.push(snapshot.status.tag());
    optional(bytes, snapshot.reason.as_ref(), |bytes, reason| {
        text(bytes, reason.as_str())
    });
    bytes.extend_from_slice(snapshot.receipt.operation_id.as_uuid().as_bytes());
    bytes.push(snapshot.receipt.action.tag());
    bytes.extend_from_slice(&snapshot.receipt.expected_revision.to_be_bytes());
    bytes.extend_from_slice(snapshot.receipt.submission_digest.as_bytes());
    anchor(bytes, snapshot.anchor);
    optional(bytes, snapshot.continuation, continuation);
    bytes.extend_from_slice(
        &snapshot
            .recorded_administration_revision
            .get()
            .to_be_bytes(),
    );
    bytes.extend_from_slice(snapshot.recorded_administration_digest.as_bytes());
    actor(
        bytes,
        snapshot.recorded_by.id,
        &snapshot.recorded_by.email,
        snapshot.recorded_at,
    );

    let view = detail.anchor;
    anchor(bytes, view.reference);
    bytes.push(view.status.tag());
    bytes.push(view.kind.tag());
    instant(bytes, view.scheduled_at.value());
    let context = view.scheduling_context;
    bytes.extend_from_slice(&context.administration_revision.get().to_be_bytes());
    bytes.extend_from_slice(context.administration_digest.as_bytes());
    bytes.extend_from_slice(&context.stage_revision.get().to_be_bytes());
    text(bytes, context.stage.as_str());
    optional(bytes, context.stage_digest, |bytes, digest| {
        bytes.extend_from_slice(digest.as_bytes())
    });
    optional(bytes, detail.continuation, |bytes, view| {
        continuation(bytes, view.reference);
        bytes.push(view.status.tag());
    });
    bytes.extend_from_slice(&(detail.attendees.len() as u64).to_be_bytes());
    for attendee in &detail.attendees {
        participant(bytes, &attendee.participant);
        optional(bytes, attendee.subject_digest, |bytes, digest| {
            bytes.extend_from_slice(digest.as_bytes())
        });
    }
    optional(bytes, detail.support.as_ref(), support);
}
fn anchor(bytes: &mut Vec<u8>, value: HearingResultAnchor) {
    bytes.extend_from_slice(value.hearing_id.as_uuid().as_bytes());
    bytes.extend_from_slice(&value.revision.get().to_be_bytes());
    bytes.extend_from_slice(value.values_digest.as_bytes());
    bytes.extend_from_slice(value.submission_digest.as_bytes());
}
fn continuation(bytes: &mut Vec<u8>, value: HearingResultContinuation) {
    bytes.extend_from_slice(value.hearing_id.as_uuid().as_bytes());
    bytes.extend_from_slice(value.result_id.as_uuid().as_bytes());
    bytes.extend_from_slice(&value.revision.get().to_be_bytes());
    bytes.extend_from_slice(value.values_digest.as_bytes());
    bytes.extend_from_slice(value.submission_digest.as_bytes());
}
fn participant(bytes: &mut Vec<u8>, value: &HearingParticipantSnapshot) {
    let overview = &value.overview;
    bytes.extend_from_slice(overview.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(overview.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&overview.revision.get().to_be_bytes());
    text(bytes, &overview.display_name);
    text(bytes, &overview.procedural_role);
    optional(bytes, overview.organization.as_deref(), text);
    text(bytes, overview.directory_status.as_str());
    optional(bytes, overview.kind, |bytes, kind| bytes.push(kind.tag()));
    optional(bytes, overview.subject, |bytes, subject| {
        bytes.extend_from_slice(subject.id.as_uuid().as_bytes());
        bytes.extend_from_slice(&subject.revision.get().to_be_bytes());
        bytes.extend_from_slice(subject.values_digest.as_bytes());
    });
    bytes.extend_from_slice(value.values_digest.as_bytes());
}
fn support(bytes: &mut Vec<u8>, value: &StageSupportSnapshot) {
    bytes.extend_from_slice(value.reference.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&value.reference.version.get().to_be_bytes());
    bytes.extend_from_slice(value.digest.as_bytes());
    text(bytes, &value.name);
    text(bytes, value.format.as_str());
    text(bytes, value.policy.as_str());
}
