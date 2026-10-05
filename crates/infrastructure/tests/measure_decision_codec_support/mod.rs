use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::hearings::{HearingNote, HearingParticipantRef, HearingSupportRef};
use domain::judicial_calendars::CivilDate;
use domain::participants::{ParticipantId, ParticipantRevision};
use domain::precautionary_hearings::{MeasureId, MeasureRevision, PrecautionaryMeasureRef};
use domain::precautionary_measures::{
    MeasureDecisionOutcome, MeasureDecisionOutcomeInput, MeasureDecisionValues,
    MeasureDecisionValuesInput, MeasureEffect, MeasureKind, MeasureProposal, MeasureSupervision,
    MeasureTime, MeasureValidity, MeasureValues, MeasureValuesInput,
};
use domain::procedural_time::DeclaredProceduralTime;
use domain::typed_participants::{CaseSubjectId, SubjectRevision, SubjectRevisionRef};
use infrastructure::measure_decision_codec::{
    decision_values, decision_view, measure_values, measure_view, outcome, outcome_view,
};
use time::{Date, Month, UtcOffset};
use uuid::Uuid;

pub fn note(value: &str) -> HearingNote {
    HearingNote::new(value).unwrap()
}

pub fn date(year: i32, month: Month, day: u8) -> CivilDate {
    CivilDate::from_date(Date::from_calendar_date(year, month, day).unwrap()).unwrap()
}

pub fn known_time(value: DeclaredProceduralTime) -> MeasureTime {
    MeasureTime::new(value, None).unwrap()
}

pub fn time() -> MeasureTime {
    MeasureTime::new(DeclaredProceduralTime::unknown(), Some(note("U"))).unwrap()
}

pub fn participant(id: u128, revision: u32) -> HearingParticipantRef {
    HearingParticipantRef::new(
        ParticipantId::from_uuid(Uuid::from_u128(id)),
        ParticipantRevision::new(revision).unwrap(),
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
        validity: MeasureValidity::new(
            known_time(DeclaredProceduralTime::date(date(2026, Month::October, 4), None).unwrap()),
            note("V"),
            None,
        )
        .unwrap(),
        supervision: MeasureSupervision::Known {
            participant: participant(3, 4),
            statement: note("S"),
        },
    }
}

pub fn measure() -> MeasureValues {
    MeasureValues::new(measure_input())
}

pub fn decision_input() -> MeasureDecisionValuesInput {
    MeasureDecisionValuesInput {
        authority: note("J\u{e9}"),
        declared_at: known_time(
            DeclaredProceduralTime::second(
                date(2026, Month::October, 4),
                1,
                2,
                3,
                Some(UtcOffset::from_hms(-6, 0, 0).unwrap()),
            )
            .unwrap(),
        ),
        justification: note("J"),
        support: HearingSupportRef::new(
            DocumentVersionRef {
                id: DocumentId::from_uuid(Uuid::from_u128(5)),
                version: DocumentVersion::new(6).unwrap(),
            },
            Sha256Digest::from_array([0x22; 32]),
        ),
        locator: note("L"),
    }
}

pub fn decision() -> MeasureDecisionValues {
    MeasureDecisionValues::new(decision_input())
}

pub fn reference(id: u128) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        MeasureId::from_uuid(Uuid::from_u128(id)),
        MeasureRevision::new(2).unwrap(),
        Sha256Digest::from_array([0x33; 32]),
    )
}

pub fn proposal(id: u128) -> MeasureProposal {
    MeasureProposal {
        id: MeasureId::from_uuid(Uuid::from_u128(id)),
        values: measure(),
    }
}

pub fn changes() -> MeasureDecisionOutcome {
    MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Impose(proposal(10)),
        MeasureEffect::Confirm {
            previous: reference(20),
        },
        MeasureEffect::Modify {
            previous: reference(30),
            values: measure(),
        },
        MeasureEffect::Revoke {
            previous: reference(40),
        },
        MeasureEffect::Cease {
            previous: reference(50),
        },
        MeasureEffect::Substitute {
            predecessors: vec![reference(61), reference(60)],
            successors: vec![proposal(71), proposal(70)],
        },
    ]))
    .unwrap()
}

pub fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

pub fn maximal_note() -> String {
    "\u{1f642}".repeat(1000)
}

pub fn maximal_measure() -> MeasureValues {
    let text = maximal_note();
    let unknown = MeasureTime::new(DeclaredProceduralTime::unknown(), Some(note(&text))).unwrap();
    let mut input = measure_input();
    input.conditions = note(&text);
    input.validity = MeasureValidity::new(unknown.clone(), note(&text), Some(unknown)).unwrap();
    input.supervision = MeasureSupervision::Known {
        participant: participant(3, 4),
        statement: note(&text),
    };
    MeasureValues::new(input)
}

pub fn maximal_decision() -> MeasureDecisionValues {
    let text = maximal_note();
    let mut input = decision_input();
    input.authority = note(&text);
    input.declared_at =
        MeasureTime::new(DeclaredProceduralTime::unknown(), Some(note(&text))).unwrap();
    input.justification = note(&text);
    input.locator = note(&text);
    MeasureDecisionValues::new(input)
}

#[test]
fn maximum_unicode_values_fit_the_exact_canonical_and_projection_bounds() {
    let declared = maximal_decision();
    assert_eq!(declared.canonical_bytes().len(), 16076);
    assert!(serde_json::to_vec(&decision_view(&declared)).unwrap().len() <= 32768);
    assert_eq!(
        decision_values(&declared.canonical_bytes(), &decision_view(&declared)).unwrap(),
        declared
    );
    let terms = maximal_measure();
    assert_eq!(terms.validity().canonical_bytes().len(), 12022);
    assert_eq!(terms.canonical_bytes().len(), 20113);
    assert!(serde_json::to_vec(&measure_view(&terms)).unwrap().len() <= 32768);
    assert_eq!(
        measure_values(&terms.canonical_bytes(), &measure_view(&terms)).unwrap(),
        terms
    );
}

#[test]
fn thirty_two_maximum_modify_and_impose_effects_fit_derived_storage_bounds() {
    for (modify, expected) in [(true, 645450), (false, 644298)] {
        let effects = (1..=32)
            .map(|id| {
                if modify {
                    MeasureEffect::Modify {
                        previous: reference(id),
                        values: maximal_measure(),
                    }
                } else {
                    let mut proposal = proposal(id);
                    proposal.values = maximal_measure();
                    MeasureEffect::Impose(proposal)
                }
            })
            .collect();
        let value =
            MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap();
        assert_eq!(value.canonical_bytes().len(), expected);
        let projection = outcome_view(&value);
        assert!(serde_json::to_vec(&projection).unwrap().len() <= 1048576);
        assert_eq!(
            outcome(&value.canonical_bytes(), &projection).unwrap(),
            value
        );
    }
}

#[test]
fn maximal_substitution_and_no_change_are_admitted_without_lower_storage_caps() {
    let successors = (2..=32)
        .map(|id| {
            let mut proposal = proposal(id);
            proposal.values = maximal_measure();
            proposal
        })
        .collect();
    let value = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Substitute {
            predecessors: vec![reference(1)],
            successors,
        },
    ]))
    .unwrap();
    assert_eq!(value.canonical_bytes().len(), 624194);
    assert_eq!(
        outcome(&value.canonical_bytes(), &outcome_view(&value)).unwrap(),
        value
    );
    let value = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::NoMeasureChange(note(
        &maximal_note(),
    )))
    .unwrap();
    assert_eq!(value.canonical_bytes().len(), 4010);
    assert_eq!(
        outcome(&value.canonical_bytes(), &outcome_view(&value)).unwrap(),
        value
    );
}
