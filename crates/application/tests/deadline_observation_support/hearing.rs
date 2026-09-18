use super::inputs;
use application::{
    case_stages::{StageDocumentFormat, StageFormatPolicy, StageSupportSnapshot},
    hearing_results::*,
    hearings::HearingParticipantSnapshot,
    typed_participants::ParticipantOverview,
};
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    participants::{DirectoryStatus, ParticipantId, ParticipantRevision},
};
use uuid::Uuid;

pub fn fixture() -> HearingResultDetail {
    let mut source = inputs::hearing(1, false, "2026-01-06", &[]);
    let participant = ParticipantId::from_uuid(Uuid::from_u128(801));
    let revision = ParticipantRevision::new(1).unwrap();
    let support = StageSupportSnapshot {
        reference: DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(802)),
            version: DocumentVersion::initial(),
        },
        digest: Sha256Digest::from_array([18; 32]),
        name: "historical.pdf".into(),
        format: StageDocumentFormat::Pdf,
        policy: StageFormatPolicy::PdfDocxV1,
    };
    let v = &source.snapshot.values;
    source.snapshot.values = HearingResultValues::new(HearingResultValuesInput {
        occurrence: v.occurrence(),
        extent: v.extent(),
        event_time: v.event_time(),
        summary: v.summary().clone(),
        attendees: vec![HearingResultAttendee::new(
            participant,
            revision,
            HearingResultCapacity::new("Declared attendee").unwrap(),
            None,
        )],
        agreements: v.agreements().to_vec(),
        provenance: HearingResultProvenance::new(
            HearingResultProvenanceKind::OperatorNote,
            None,
            Some(HearingResultSupportRef::new(
                support.reference,
                support.digest,
            )),
        )
        .unwrap(),
    })
    .unwrap();
    source.attendees.push(HearingResultAttendeeSnapshot {
        participant: HearingParticipantSnapshot {
            overview: ParticipantOverview {
                case_id: inputs::case_id(),
                id: participant,
                revision,
                display_name: "Historical attendee".into(),
                procedural_role: "Declared role".into(),
                organization: None,
                directory_status: DirectoryStatus::Active,
                kind: None,
                subject: None,
            },
            values_digest: Sha256Digest::from_array([19; 32]),
        },
        subject_digest: None,
    });
    source.support = Some(support);
    inputs::resign_hearing(&mut source);
    hearing_result_receipt_matches(inputs::hasher().as_ref(), &source).unwrap();
    source
}
