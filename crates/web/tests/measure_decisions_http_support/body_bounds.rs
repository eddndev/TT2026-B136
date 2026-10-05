use super::*;
use crate::{
    measure_decision_fixtures::{id, Fixture},
    record_decision_support::FixtureV2,
};
use domain::{hearings::HearingNote, procedural_time::DeclaredProceduralTime};

#[tokio::test]
async fn maximal_32_measure_unicode_escape_body_fits_the_declared_4_mib_limit() {
    let text = "\u{1f600}".repeat(1000);
    let note = HearingNote::new(&text).unwrap();
    let unknown = MeasureTime::new(DeclaredProceduralTime::unknown(), Some(note.clone())).unwrap();
    let mut fixture = Fixture::single();
    let mut source = crate::measure_source_support::Fixture::unknown(false);
    let original = crate::effect_support::values_input(&source.values);
    source.values = MeasureValues::new(MeasureValuesInput {
        conditions: note.clone(),
        validity: MeasureValidity::new(unknown.clone(), note.clone(), Some(unknown.clone()))
            .unwrap(),
        supervision: MeasureSupervision::Unknown {
            reason: note.clone(),
        },
        ..original
    });
    let mut decision = crate::measure_decision_fixtures::decision_input(&fixture.command.values);
    decision.authority = note.clone();
    decision.justification = note.clone();
    decision.locator = note;
    decision.declared_at = unknown;
    fixture.command.values = MeasureDecisionValues::new(decision);
    fixture.command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(
        (0..32)
            .map(|n| {
                MeasureEffect::Impose(MeasureProposal {
                    id: id(1000 + n),
                    values: source.values.clone(),
                })
            })
            .collect(),
    ))
    .unwrap();
    fixture.material.result_sources = (0..32)
        .map(|n| MeasureResultSources {
            id: id(1000 + n),
            sources: source.sources.clone(),
        })
        .collect();
    let row = from_g2(FixtureV2::initial(fixture));
    let input = command(row.command())
        .to_string()
        .replace('\u{1f600}', "\\ud83d\\ude00");
    assert!(input.len() > 1024 * 1024);
    assert!(input.len() < 2 * 1024 * 1024);
    let returned = review(&row);
    let mut write = MockWrite::new();
    write
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, parsed| {
            assert_eq!(&parsed, returned.command());
            Ok(returned)
        });
    let (status, value) = raw(
        (write, MockRead::new()),
        "POST",
        &format!("{}/prepare", base()),
        "staff-token",
        input,
        "application/json",
    )
    .await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(value["review"]["results"].as_array().unwrap().len(), 32);
    assert_eq!(value["review"]["results"][0]["values"]["conditions"], text);
}
