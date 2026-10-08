use domain::crypto::Sha256Digest;
use domain::hearings::{HearingNote, HearingParticipantRef};
use domain::judicial_calendars::CivilDate;
use domain::participants::{ParticipantId, ParticipantRevision};
use domain::precautionary_measures::{
    MeasureCorrectionValues, MeasureKind, MeasureSupervision, MeasureTime, MeasureValidity,
    MeasureValues, MeasureValuesInput,
};
use domain::procedural_time::DeclaredProceduralTime;
use domain::typed_participants::{CaseSubjectId, SubjectRevision, SubjectRevisionRef};
use uuid::Uuid;

pub fn note(value: &str) -> HearingNote {
    HearingNote::new(value).unwrap()
}

pub fn day() -> CivilDate {
    "2026-10-04".parse().unwrap()
}

pub fn known(value: DeclaredProceduralTime) -> MeasureTime {
    MeasureTime::new(value, None).unwrap()
}

pub fn unknown(reason: &str) -> MeasureTime {
    MeasureTime::new(DeclaredProceduralTime::unknown(), Some(note(reason))).unwrap()
}

pub fn correction(value: &MeasureValues) -> MeasureCorrectionValues {
    let text = match value.supervision() {
        MeasureSupervision::Known { statement, .. } => statement,
        MeasureSupervision::Unknown { reason } => reason,
    };
    MeasureCorrectionValues::new(
        value.conditions().clone(),
        value.validity().clone(),
        text.clone(),
    )
}

pub fn replace_validity(
    value: &MeasureValues,
    validity: MeasureValidity,
) -> MeasureCorrectionValues {
    MeasureCorrectionValues::new(
        value.conditions().clone(),
        validity,
        correction(value).supervision_text().clone(),
    )
}

pub fn validity() -> MeasureValidity {
    MeasureValidity::new(
        known(DeclaredProceduralTime::date(day(), None).unwrap()),
        note("Declared duration"),
        None,
    )
    .unwrap()
}

pub fn participant() -> HearingParticipantRef {
    HearingParticipantRef::new(
        ParticipantId::from_uuid(Uuid::from_u128(3)),
        ParticipantRevision::new(4).unwrap(),
    )
}

pub fn input() -> MeasureValuesInput {
    MeasureValuesInput {
        subject: SubjectRevisionRef {
            id: CaseSubjectId::from_uuid(Uuid::from_u128(1)),
            revision: SubjectRevision::new(2).unwrap(),
            values_digest: Sha256Digest::from_array([0x11; 32]),
        },
        kind: MeasureKind::PeriodicAppearance,
        conditions: note("Original conditions"),
        validity: validity(),
        supervision: MeasureSupervision::Known {
            participant: participant(),
            statement: note("Original supervision statement"),
        },
    }
}
