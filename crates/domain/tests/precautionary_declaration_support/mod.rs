use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::hearings::{HearingNote, HearingParticipantRef, HearingSupportRef};
use domain::judicial_calendars::CivilDate;
use domain::participants::{ParticipantId, ParticipantRevision};
use domain::precautionary_measures::{
    MeasureDecisionValuesInput, MeasureKind, MeasureSupervision, MeasureTime, MeasureValidity,
    MeasureValuesInput,
};
use domain::procedural_time::DeclaredProceduralTime;
use domain::typed_participants::{CaseSubjectId, SubjectRevision, SubjectRevisionRef};
use time::{Date, Month, UtcOffset};
use uuid::Uuid;

pub fn note(value: &str) -> HearingNote {
    HearingNote::new(value).unwrap()
}

pub fn date(year: i32, month: Month, day: u8) -> CivilDate {
    CivilDate::from_date(Date::from_calendar_date(year, month, day).unwrap()).unwrap()
}

pub fn day() -> CivilDate {
    date(2026, Month::October, 4)
}

pub fn known_time(value: DeclaredProceduralTime) -> MeasureTime {
    MeasureTime::new(value, None).unwrap()
}

pub fn unknown_time(reason: &str) -> MeasureTime {
    MeasureTime::new(DeclaredProceduralTime::unknown(), Some(note(reason))).unwrap()
}

pub fn validity() -> MeasureValidity {
    MeasureValidity::new(
        known_time(DeclaredProceduralTime::date(day(), None).unwrap()),
        note("V"),
        None,
    )
    .unwrap()
}

pub fn participant(id: u128, revision: u32) -> HearingParticipantRef {
    HearingParticipantRef::new(
        ParticipantId::from_uuid(Uuid::from_u128(id)),
        ParticipantRevision::new(revision).unwrap(),
    )
}

pub fn support(id: u128, version: u32, digest: u8) -> HearingSupportRef {
    HearingSupportRef::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::from_u128(id)),
            version: DocumentVersion::new(version).unwrap(),
        },
        Sha256Digest::from_array([digest; 32]),
    )
}

pub fn measure_input() -> MeasureValuesInput {
    MeasureValuesInput {
        subject: SubjectRevisionRef {
            id: CaseSubjectId::from_uuid(Uuid::from_u128(1)),
            revision: SubjectRevision::new(2).unwrap(),
            values_digest: Sha256Digest::from_array([0x11; 32]),
        },
        kind: MeasureKind::PretrialDetention,
        conditions: note("C\u{e9}"),
        validity: validity(),
        supervision: MeasureSupervision::Known {
            participant: participant(3, 4),
            statement: note("S"),
        },
    }
}

pub fn decision_input() -> MeasureDecisionValuesInput {
    MeasureDecisionValuesInput {
        authority: note("J\u{e9}"),
        declared_at: known_time(
            DeclaredProceduralTime::second(
                day(),
                1,
                2,
                3,
                Some(UtcOffset::from_hms(-6, 0, 0).unwrap()),
            )
            .unwrap(),
        ),
        justification: note("J"),
        support: support(5, 6, 0x22),
        locator: note("L"),
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
