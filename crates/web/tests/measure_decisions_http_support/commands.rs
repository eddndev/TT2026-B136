use super::*;
use crate::{measure_decision_fixtures::Fixture, record_decision_support::FixtureV2};
use domain::{
    hearings::HearingNote, judicial_calendars::CivilDate, procedural_time::DeclaredProceduralTime,
};

async fn prepared(row: MeasureDecisionRecordReceipt, input: Value) -> Value {
    let expected = review(&row);
    let returned = expected.clone();
    let mut write = MockWrite::new();
    write
        .expect_prepare()
        .times(1)
        .return_once(move |token, case, command| {
            assert_eq!((token, case), ("staff-token", case_id()));
            assert_eq!(&command, returned.command());
            Ok(returned)
        });
    let (status, value) = request(
        write,
        MockRead::new(),
        "POST",
        &format!("{}/prepare", base()),
        Some(input),
    )
    .await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(value["review"]["command"], command(expected.command()));
    value
}

#[tokio::test]
async fn exact_legacy_replay_preserves_g1_review_and_receipt_family() {
    let row = g1();
    let value = prepared(row.clone(), command(row.command())).await;
    assert_eq!(value["family"], "g1");
    let mut previous = None;
    for _ in 0..2 {
        let returned = row.clone();
        let mut write = MockWrite::new();
        write
            .expect_submit()
            .times(1)
            .return_once(move |_, _, input, c| {
                assert_eq!(&input, returned.command());
                assert_eq!(c.submission_digest, returned.submission_digest());
                assert_eq!(c.review_digest, returned.review_digest());
                Ok(returned)
            });
        let (status, value) = request(
            write,
            MockRead::new(),
            "POST",
            &format!("{}/submit", base()),
            Some(submission(&row)),
        )
        .await;
        assert_eq!(status, 201, "{value}");
        assert_eq!(value["family"], "g1");
        assert_eq!(value["group"]["family"], "g1");
        assert_eq!(value["group"]["measures"][0]["family"], "m1");
        assert_eq!(value["measure_history"], json!({"groups":[]}));
        assert!(value.get("record_history").is_none());
        if let Some(prior) = previous {
            assert_eq!(value, prior);
        }
        previous = Some(value);
    }
}

#[tokio::test]
async fn every_effect_and_no_change_reaches_the_workflow_as_one_declared_outcome() {
    let initial = Fixture::single().capture();
    for action in [
        "impose",
        "confirm",
        "modify",
        "revoke",
        "cease",
        "substitute",
        "no_measure_change",
    ] {
        let mut fixture = FixtureV2::from_v1(&initial, &MeasureHistoryEvidence { groups: vec![] });
        let prior = crate::measure_decision_fixtures::reference(&initial.measures[0]);
        match action {
            "impose" => fixture = FixtureV2::initial(Fixture::single()),
            "no_measure_change" => fixture = FixtureV2::initial(Fixture::no_change()),
            "confirm" => {}
            "modify" => {
                let mut input =
                    crate::effect_support::values_input(&initial.measures[0].result.values);
                input.conditions = HearingNote::new("Modified declared conditions").unwrap();
                fixture.effects(vec![MeasureEffect::Modify {
                    previous: prior,
                    values: MeasureValues::new(input),
                }]);
            }
            "revoke" => fixture.effects(vec![MeasureEffect::Revoke { previous: prior }]),
            "cease" => fixture.effects(vec![MeasureEffect::Cease { previous: prior }]),
            _ => {
                let source = initial.measures[0].result.clone();
                fixture.effects(vec![MeasureEffect::Substitute {
                    predecessors: vec![prior],
                    successors: vec![MeasureProposal {
                        id: crate::measure_decision_fixtures::id(80),
                        values: source.values,
                    }],
                }]);
                fixture.material.result_sources.push(MeasureResultSources {
                    id: crate::measure_decision_fixtures::id(80),
                    sources: source.sources,
                });
            }
        }
        let row = from_g2(fixture);
        let value = prepared(row.clone(), command(row.command())).await;
        if action == "no_measure_change" {
            assert_eq!(value["review"]["command"]["outcome"]["kind"], action);
            assert_eq!(value["review"]["results"], json!([]));
        } else {
            assert_eq!(
                value["review"]["command"]["outcome"]["effects"][0]["action"],
                action
            );
            assert_eq!(
                value["review"]["results"].as_array().unwrap().len(),
                if action == "substitute" { 2 } else { 1 }
            );
        }
    }
}

#[tokio::test]
async fn declared_precision_and_normalized_text_and_order_are_preserved() {
    let day: CivilDate = "2020-01-02".parse().unwrap();
    let offset = time::UtcOffset::from_hms(-6, 0, 0).unwrap();
    for declared_at in [
        DeclaredProceduralTime::date(day, None).unwrap(),
        DeclaredProceduralTime::minute(day, 10, 11, None).unwrap(),
        DeclaredProceduralTime::second(day, 10, 11, 12, Some(offset)).unwrap(),
    ] {
        let mut fixture = Fixture::multiple();
        let mut values = crate::measure_decision_fixtures::decision_input(&fixture.command.values);
        values.authority = HearingNote::new("Court\nRegistry").unwrap();
        values.declared_at = MeasureTime::new(declared_at, None).unwrap();
        fixture.command.values = MeasureDecisionValues::new(values);
        let row = from_g2(FixtureV2::initial(fixture));
        let mut input = command(row.command());
        input["values"]["authority"] = json!("  Court\r\nRegistry  ");
        input["outcome"]["effects"]
            .as_array_mut()
            .unwrap()
            .reverse();
        let value = prepared(row, input).await;
        assert_eq!(
            value["review"]["command"]["values"]["authority"],
            "Court\nRegistry"
        );
        let projected = &value["review"]["command"]["values"]["declared_at"];
        assert!(projected.get("reason").is_none());
        assert_eq!(
            projected["offset_seconds"],
            json!(declared_at.offset().map(|v| v.whole_seconds()))
        );
    }
}

#[tokio::test]
async fn initial_and_precautionary_anchors_keep_exact_selectors_and_full_material() {
    for kind in ["initial", "precautionary"] {
        let mut fixture = Fixture::no_change();
        if kind == "initial" {
            crate::decision_anchor_support::attach_initial(
                &mut fixture,
                crate::decision_anchor_support::cancelled_initial(),
            );
        } else {
            crate::decision_anchor_support::attach_precautionary(
                &mut fixture,
                crate::precautionary_receipt_support::scheduled(),
            );
        }
        let row = from_g2(FixtureV2::initial(fixture));
        let value = prepared(row.clone(), command(row.command())).await;
        assert_eq!(
            value["review"]["command"]["anchor"],
            command(row.command())["anchor"]
        );
        let source = &value["review"]["material"]["anchor"];
        assert_eq!(source["kind"], kind);
        if kind == "initial" {
            assert_eq!(source["hearing"]["status"], "cancelled");
            assert_eq!(source["hearing"]["revision"], 2);
        } else {
            assert_eq!(
                source["capture"]["review"]["resolved_values"]["purpose"],
                "imposition"
            );
            assert_eq!(
                source["capture"]["capture_digest"],
                command(row.command())["anchor"]["capture_digest"]
            );
        }
    }
}
