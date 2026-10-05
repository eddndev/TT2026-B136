use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::hearings::{
    HearingModality, HearingNote, HearingParticipantRef, HearingSupportRef, HearingTime,
    HearingVenue,
};
use domain::participants::{ParticipantId, ParticipantRevision};
use domain::precautionary_hearings::{
    MeasureId, MeasureRevision, PrecautionaryHearingPurpose, PrecautionaryHearingSchedulingBasis,
    PrecautionaryHearingValues, PrecautionaryHearingValuesInput, PrecautionaryMeasureRef,
};
use infrastructure::precautionary_hearing_codec::{values, view};
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

#[path = "precautionary_hearing_codec_support/strictness.rs"]
mod strictness;
#[path = "precautionary_hearing_codec_support/vectors.rs"]
mod vectors;

fn participant(id: u128) -> HearingParticipantRef {
    HearingParticipantRef::new(
        ParticipantId::from_uuid(Uuid::from_u128(id)),
        ParticipantRevision::new(2).unwrap(),
    )
}

fn target(id: u128) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        MeasureId::from_uuid(Uuid::from_u128(id)),
        MeasureRevision::new(3).unwrap(),
        Sha256Digest::from_array([0x55; 32]),
    )
}

fn input(review: bool) -> PrecautionaryHearingValuesInput {
    PrecautionaryHearingValuesInput {
        purpose: if review {
            PrecautionaryHearingPurpose::Review
        } else {
            PrecautionaryHearingPurpose::Imposition
        },
        scheduled_at: HearingTime::new(OffsetDateTime::UNIX_EPOCH).unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Court A").unwrap(),
        note: Some(HearingNote::new("Note").unwrap()),
        participants: vec![participant(9)],
        scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
            HearingNote::new("Set by order").unwrap(),
            HearingSupportRef::new(
                DocumentVersionRef {
                    id: DocumentId::from_uuid(Uuid::from_u128(77)),
                    version: DocumentVersion::new(4).unwrap(),
                },
                Sha256Digest::from_array([0x42; 32]),
            ),
            HearingNote::new("Page 2").unwrap(),
        ),
        review_targets: if review { vec![target(11)] } else { vec![] },
    }
}

fn fixture(review: bool) -> PrecautionaryHearingValues {
    PrecautionaryHearingValues::new(input(review)).unwrap()
}

#[test]
fn imposition_and_review_values_round_trip_without_receipt_material() {
    for review in [false, true] {
        let original = fixture(review);
        assert_eq!(
            values(&original.canonical_bytes(), &view(&original)).unwrap(),
            original
        );
    }
}

#[test]
fn projection_has_only_exact_declared_fields_and_scalar_types() {
    assert_eq!(
        view(&fixture(true)),
        json!({
            "purpose":"review",
            "time":{"seconds":0,"offset_seconds":0},
            "modality":"in_person",
            "venue":"Court A",
            "note":"Note",
            "participants":[{"id":"00000000-0000-0000-0000-000000000009","revision":2}],
            "scheduling_basis":{
                "statement":"Set by order",
                "document_id":"00000000-0000-0000-0000-00000000004d",
                "version":4,
                "digest":"4242424242424242424242424242424242424242424242424242424242424242",
                "locator":"Page 2"
            },
            "review_targets":[{
                "id":"00000000-0000-0000-0000-00000000000b",
                "revision":3,
                "digest":"5555555555555555555555555555555555555555555555555555555555555555"
            }]
        })
    );
}

#[test]
fn null_note_empty_participants_and_original_schedule_offset_round_trip() {
    for modality in [HearingModality::InPerson, HearingModality::Videoconference] {
        let mut source = input(false);
        source.note = None;
        source.participants.clear();
        source.modality = modality;
        source.scheduled_at = HearingTime::new(
            OffsetDateTime::UNIX_EPOCH.to_offset(time::UtcOffset::from_hms(-6, 0, 0).unwrap()),
        )
        .unwrap();
        let original = PrecautionaryHearingValues::new(source).unwrap();
        let projection = view(&original);
        assert!(projection["note"].is_null());
        assert_eq!(projection["time"]["offset_seconds"], -21600);
        assert_eq!(
            values(&original.canonical_bytes(), &projection).unwrap(),
            original
        );
    }
}
