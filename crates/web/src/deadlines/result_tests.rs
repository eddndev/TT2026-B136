use super::{result, result_test_support::*};
use serde_json::json;

#[test]
fn natural_days_projection_keeps_historical_candidate_without_recalculation() {
    let value = decoded(record(&[vec![10]], "2026-01-09"));
    assert_eq!(
        result::project(&value).unwrap(),
        json!({
            "requirement":{"kind":"source_field","field":"resolution_issued_at"},
            "trigger_outcome":{"kind":"extracted","at":time_json()},
            "rule":days_rule_json(),
            "arithmetic":{"rule":days_rule_json(),"anchor":time_json(),
                "outcome":{"kind":"civil_candidate","date":"2026-01-09"},
                "trace":[{"kind":"natural_days","first_included":"2026-01-06","quantity":2,"candidate":"2026-01-09"}]},
            "due_at":null,"blocks":[{"kind":"civil_cutoff_missing"}]
        })
    );
}

#[test]
fn monthly_projection_preserves_requested_day_missing_candidate_and_block_details() {
    let output = result::project(&monthly()).unwrap();
    assert_eq!(
        output["rule"],
        json!({"kind":"civil_months","quantity":1,"final_day":"preserve"})
    );
    assert_eq!(
        output["arithmetic"]["trace"],
        json!([{"kind":"civil_months","anchor":"2026-01-31","quantity":1,
        "target_year":2026,"target_month":2,"requested_day":31,"candidate":null}])
    );
    let cause = json!({"kind":"missing_homologous_day","year":2026,"month":2,"requested_day":31});
    assert_eq!(
        output["arithmetic"]["outcome"],
        json!({"kind":"blocked","block":cause})
    );
    assert_eq!(
        output["blocks"],
        json!([{"kind":"arithmetic","block":cause}])
    );
    assert!(output["due_at"].is_null());
}

#[test]
fn hourly_projection_keeps_nanos_offsets_and_original_declared_precision_separate() {
    for offset in [-93_599, 93_599] {
        let output = result::project(&hourly(offset)).unwrap();
        assert_eq!(output["rule"], json!({"kind":"elapsed_hours","quantity":1}));
        assert_eq!(output["arithmetic"]["anchor"], time_json());
        assert_eq!(
            output["arithmetic"]["trace"],
            json!([{"kind":"elapsed_hours","start":instant_json(0,123,offset),
            "quantity":1,"candidate":instant_json(3600,123,offset)}])
        );
        assert_eq!(
            output["arithmetic"]["outcome"],
            json!({"kind":"instant_candidate","instant":instant_json(3600,123,offset)})
        );
        assert_eq!(output["due_at"], instant_json(3600, 123, offset));
        assert_eq!(output["blocks"], json!([]));
    }
}

#[test]
fn calendar_trace_exposes_accumulation_exact_origins_sources_and_outside_coverage() {
    let output = result::project(&calendar(false)).unwrap();
    for (index, tag, origin) in [
        (
            0,
            "counted_days",
            json!({"kind":"weekly_pattern","weekday":2}),
        ),
        (
            1,
            "final_day",
            json!({"kind":"exception","id":"00000000-0000-0000-0000-000000000000"}),
        ),
    ] {
        assert_eq!(
            output["arithmetic"]["trace"][index],
            json!({"kind":tag,"count":{
            "first_included":"2026-01-06","quantity":1,"accumulated":1,
            "outcome":{"kind":"candidate","date":"2026-01-06"},
            "trace":[{"day":{"date":"2026-01-06","origin":origin,"classification":"countable",
                "explanation":"Declared countable.","source_ids":["00000000-0000-0000-0000-000000000000"]},"accumulated":1}]}})
        );
    }
    let outside = result::project(&calendar(true)).unwrap();
    let count = &outside["arithmetic"]["trace"][0]["count"];
    assert_eq!(
        count["outcome"],
        json!({"kind":"outside_coverage","date":"2026-01-06"})
    );
    assert_eq!(count["accumulated"], 0);
    assert_eq!(
        count["trace"],
        json!([{"day":{"date":"2026-01-06","origin":null,"classification":null,
        "explanation":null,"source_ids":[]},"accumulated":0}])
    );
}

#[test]
fn blocked_trigger_and_missing_rule_keep_absent_outputs_and_declared_block_order() {
    let mut condition = vec![4];
    condition.extend([0; 16]);
    let output = result::project(&decoded(envelope(
        &[1, 0],
        None,
        None,
        None,
        &[vec![0], vec![2], condition, vec![7, 0], vec![8, 0]],
    )))
    .unwrap();
    assert_eq!(
        output["trigger_outcome"],
        json!({"kind":"blocked","block":{"kind":"unknown_source"}})
    );
    for field in ["rule", "arithmetic", "due_at"] {
        assert!(output[field].is_null());
    }
    assert_eq!(
        output["blocks"],
        json!([{"kind":"scope_unknown"},{"kind":"incident_unknown"},
        {"kind":"condition_missing","id":"00000000-0000-0000-0000-000000000000"},
        {"kind":"rule","block":{"kind":"missing_ordered_quantity"}},
        {"kind":"trigger","block":{"kind":"unknown_source"}}])
    );
}

#[test]
fn trigger_block_variants_use_named_fields_instead_of_debug_strings() {
    let cases = [
        (
            vec![1, 2],
            json!({"kind":"absent_field","field":"notification_received_at"}),
        ),
        (
            vec![2, 0, 2],
            json!({"kind":"incompatible_family","expected":"resolution","actual":"hearing_result"}),
        ),
        (
            vec![3, 0],
            json!({"kind":"missing_qualification","purpose":"hearing_end"}),
        ),
        (
            vec![4, 0, 1],
            json!({"kind":"qualification_mismatch","expected":"hearing_end","actual":"ordered_period_start"}),
        ),
        (vec![5], json!({"kind":"unexpected_qualification"})),
    ];
    for (cause, expected) in cases {
        let mut trigger = vec![1];
        trigger.extend(&cause);
        let mut block = vec![8];
        block.extend(cause);
        let output = result::project(&decoded(envelope(
            &trigger,
            Some(&natural_rule()),
            None,
            None,
            &[block],
        )))
        .unwrap();
        assert_eq!(
            output["trigger_outcome"],
            json!({"kind":"blocked","block":expected})
        );
        assert_eq!(
            output["blocks"],
            json!([{"kind":"trigger","block":expected}])
        );
    }
}
