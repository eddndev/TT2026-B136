use domain::crypto::Sha256Digest;
use domain::hearings::HearingNote;
use domain::precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef};
use domain::precautionary_measures::{
    MeasureDecisionOutcome, MeasureDecisionOutcomeInput, MeasureEffect, MeasureKind,
    MeasureProposal, MeasureSupervision, MeasureTime, MeasureValidity, MeasureValues,
    MeasureValuesInput,
};
use domain::procedural_time::DeclaredProceduralTime;
use domain::typed_participants::{CaseSubjectId, SubjectRevision, SubjectRevisionRef};
use uuid::Uuid;

pub fn note(value: &str) -> HearingNote {
    HearingNote::new(value).unwrap()
}

pub fn id(value: u128) -> MeasureId {
    MeasureId::from_uuid(Uuid::from_u128(value))
}

pub fn previous(value: u128) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        id(value),
        MeasureRevision::new(2).unwrap(),
        Sha256Digest::from_array([0x22; 32]),
    )
}

pub fn values_input() -> MeasureValuesInput {
    MeasureValuesInput {
        subject: SubjectRevisionRef {
            id: CaseSubjectId::from_uuid(Uuid::from_u128(100)),
            revision: SubjectRevision::new(1).unwrap(),
            values_digest: Sha256Digest::from_array([0x11; 32]),
        },
        kind: MeasureKind::PeriodicAppearance,
        conditions: note("C"),
        validity: MeasureValidity::new(
            MeasureTime::new(DeclaredProceduralTime::unknown(), Some(note("U"))).unwrap(),
            note("V"),
            None,
        )
        .unwrap(),
        supervision: MeasureSupervision::Unknown { reason: note("S") },
    }
}

pub fn proposal(value: u128) -> MeasureProposal {
    MeasureProposal {
        id: id(value),
        values: MeasureValues::new(values_input()),
    }
}

pub fn changes(effects: Vec<MeasureEffect>) -> MeasureDecisionOutcome {
    MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap()
}

pub fn mixed() -> Vec<MeasureEffect> {
    vec![
        MeasureEffect::Impose(proposal(1)),
        MeasureEffect::Confirm {
            previous: previous(2),
        },
        MeasureEffect::Modify {
            previous: previous(3),
            values: MeasureValues::new(values_input()),
        },
        MeasureEffect::Revoke {
            previous: previous(4),
        },
        MeasureEffect::Cease {
            previous: previous(5),
        },
        MeasureEffect::Substitute {
            predecessors: vec![previous(6), previous(7)],
            successors: vec![proposal(8), proposal(9)],
        },
    ]
}

pub fn rejects(effects: Vec<MeasureEffect>) {
    assert!(MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).is_err());
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
