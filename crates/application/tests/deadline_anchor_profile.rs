use crate::deadline_evaluation_support::*;
use application::{deadline_evaluations::*, deadline_inputs::*, deadline_profiles::*};
use domain::{
    deadline_arithmetic::*, deadline_profiles::*, deadline_triggers::*,
    procedural_time::DeclaredProceduralTime,
};
use time::UtcOffset;

fn anchored() -> DeadlineProfileDefinitionInput {
    let mut profile = definition();
    let anchor = DeclaredProceduralTime::second(
        date("2026-10-11"),
        13,
        27,
        42,
        Some(UtcOffset::from_hms(-6, 0, 0).unwrap()),
    )
    .unwrap();
    profile.trigger = TriggerRequirement::Qualified {
        purpose: QualifiedTriggerPurpose::HearingEnd,
        family: TriggerFamily::HearingResult,
    };
    profile.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::AtAnchor);
    profile.completion = DeadlineCompletionPolicy::ArithmeticInstant;
    profile.examples[0].anchor = anchor;
    profile.examples[0].expected =
        DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate {
            instant: anchor.instant_value().unwrap(),
        });
    profile
}

#[test]
fn anchor_profile_preserves_qualified_hearing_end_in_canonical_definition() {
    let profile = DeadlineProfileDefinition::new(anchored()).unwrap();
    let bytes = deadline_profile_definition_bytes(&profile);
    let restored = decode_deadline_profile_definition(&bytes).unwrap();
    assert_eq!(deadline_profile_definition_bytes(&restored), bytes);
    assert_eq!(
        restored.template(),
        DeadlineRuleTemplate::Fixed(ArithmeticRule::AtAnchor)
    );
    assert_eq!(restored.trigger(), profile.trigger());
}

#[test]
fn anchor_profile_rejects_unqualified_or_other_family_and_civil_completion() {
    for trigger in [
        TriggerRequirement::SourceField(TriggerField::HearingSessionEventTime),
        TriggerRequirement::Qualified {
            purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
            family: TriggerFamily::HearingResult,
        },
        TriggerRequirement::Qualified {
            purpose: QualifiedTriggerPurpose::HearingEnd,
            family: TriggerFamily::Resolution,
        },
    ] {
        let mut profile = anchored();
        profile.trigger = trigger;
        assert!(DeadlineProfileDefinition::new(profile).is_err());
    }
    for completion in [
        DeadlineCompletionPolicy::CivilCandidateOnly,
        cutoff("2026-01-01", "2026-12-31"),
    ] {
        let mut profile = anchored();
        profile.completion = completion;
        assert!(DeadlineProfileDefinition::new(profile).is_err());
    }
}

fn evaluation() -> ProfiledDeadlineEvaluation {
    let profile = anchored();
    let (_, mut input, _) = fixture();
    let source = DeadlineSourceDetail::HearingResult(Box::new(inputs::hearing(
        1,
        false,
        "2026-10-11",
        &[0],
    )));
    input.selection = inputs::request(&source).trigger;
    input.selection.qualification = Some(QualifiedTriggerTime {
        purpose: QualifiedTriggerPurpose::HearingEnd,
        at: profile.examples[0].anchor,
        statement: text("The source expressly states the hearing ended at this instant"),
        locator: label("Hearing record, closing declaration"),
    });
    evaluate(profile, &input, &inputs::material(source))
}

#[test]
fn exact_hearing_end_is_operational_and_roundtrips_without_changing_history() {
    let evaluated = evaluation();
    assert!(evaluated.blocks().is_empty());
    assert_eq!(
        evaluated.due_at(),
        anchored().examples[0].anchor.instant_value()
    );
    let captured = DeadlineEvaluationRecord::capture(&evaluated);
    let bytes = deadline_evaluation_record_bytes(&captured);
    assert_eq!(decode_deadline_evaluation_record(&bytes).unwrap(), captured);
    for end in 0..bytes.len() {
        assert!(decode_deadline_evaluation_record(&bytes[..end]).is_err());
    }
}

#[test]
fn mutually_consistent_but_shifted_trace_outcome_and_due_cannot_replace_anchor() {
    let captured = DeadlineEvaluationRecord::capture(&evaluation());
    let mut bytes = deadline_evaluation_record_bytes(&captured);
    let old = captured.due_at().unwrap().unix_timestamp().to_be_bytes();
    let new = (captured.due_at().unwrap().unix_timestamp() + 60).to_be_bytes();
    let offsets: Vec<_> = bytes
        .windows(old.len())
        .enumerate()
        .filter_map(|(i, value)| (value == old).then_some(i))
        .collect();
    assert_eq!(offsets.len(), 3);
    for index in offsets {
        bytes[index..index + 8].copy_from_slice(&new);
    }
    assert!(decode_deadline_evaluation_record(&bytes).is_err());
}
