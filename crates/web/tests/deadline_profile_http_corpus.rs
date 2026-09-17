mod deadline_profile_http_support;
use deadline_profile_http_support::*;
use serde_json::{json, Value};
use std::sync::Arc;

async fn round_trip(definition: Value) -> Value {
    let mut c = command_json(None);
    c["change"]["definition"] = definition.clone();
    let (status, body) = prepare_json(Arc::new(Workflow::default()), c).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["definition"], definition);
    body
}
#[tokio::test]
async fn ordered_quantities_reproduce_missing_excess_and_success_without_defaults() {
    let mut d = definition_json(None);
    d["template"] = json!({"kind":"ordered","unit":{"kind":"days","inclusion":"on_anchor","basis":"natural","final_day":"preserve"},"maximum":3});
    let base = d["examples"][0].clone();
    d["examples"]=json!((0..3).map(|n| {
        let mut e=base.clone();e["id"]=json!(uuid::Uuid::from_u128(n+2));
        match n {
            0=>{e["ordered_quantity"]=json!(2);e["expected"]["outcome"]["date"]=json!("2026-01-07");},
            1=>e["expected"]=json!({"kind":"rule_blocked","block":{"kind":"missing_ordered_quantity"}}),
            _=>{e["ordered_quantity"]=json!(4);e["expected"]=json!({"kind":"rule_blocked","block":{"kind":"ordered_quantity_exceeds_maximum","maximum":3,"supplied":4}});},
        } e
    }).collect::<Vec<_>>());
    round_trip(d).await;
}
#[tokio::test]
async fn hour_examples_preserve_second_zero_absent_offset_and_original_expected_offset() {
    let mut d = definition_json(None);
    d["template"] = json!({"kind":"fixed","rule":{"kind":"elapsed_hours","quantity":1}});
    d["completion"] = json!({"kind":"arithmetic_instant"});
    let base = d["examples"][0].clone();
    d["examples"]=json!((0..4).map(|n| {
        let mut e=base.clone();e["id"]=json!(uuid::Uuid::from_u128(n+2));
        e["anchor"]=json!({"precision":"second","year":1970,"month":1,"day":1,"hour":0,"minute":0,"second":0,"offset_seconds":0});
        e["expected"]=json!({"kind":"arithmetic","outcome":{"kind":"instant_candidate","instant":{"unix_seconds":3600,"nanosecond":0,"offset_seconds":93000}}});
        if n==1 {e["anchor"]["offset_seconds"]=Value::Null;e["expected"]["outcome"]=json!({"kind":"blocked","block":{"kind":"missing_offset"}});}
        if n==2 {e["anchor"]["precision"]=json!("minute");e["anchor"].as_object_mut().unwrap().remove("second");e["expected"]["outcome"]=json!({"kind":"blocked","block":{"kind":"insufficient_precision","observed":"minute"}});}
        if n==3 {e["anchor"]=json!({"precision":"unknown"});e["expected"]["outcome"]=json!({"kind":"blocked","block":{"kind":"unknown_anchor"}});}
        e
    }).collect::<Vec<_>>());
    let body = round_trip(d).await;
    assert_eq!(
        body["definition"]["examples"][0]["expected"]["outcome"]["instant"]["offset_seconds"],
        93000
    );
}
#[tokio::test]
async fn civil_month_examples_keep_missing_homologue_and_explicit_cutoff() {
    let mut d = definition_json(None);
    d["template"] =
        json!({"kind":"fixed","rule":{"kind":"civil_months","quantity":1,"final_day":"preserve"}});
    d["completion"] = json!({"kind":"civil_cutoff","time":"18:30:00","offset_seconds":-21600,"from":"2026-01-01","through":"2026-12-31","channel":"Declared counter","reference_id":ID});
    d["examples"][0]["expected"]["outcome"]["date"] = json!("2026-02-06");
    let mut missing = d["examples"][0].clone();
    missing["id"] = json!(CASE);
    missing["anchor"]["day"] = json!(31);
    missing["expected"]["outcome"] = json!({"kind":"blocked","block":{"kind":"missing_homologous_day","year":2026,"month":2,"requested_day":31}});
    d["examples"].as_array_mut().unwrap().push(missing);
    round_trip(d).await;
}
#[tokio::test]
async fn source_fields_and_qualified_purposes_remain_explicit_configuration() {
    let mut triggers = [
        "resolution_issued_at",
        "notification_practiced_at",
        "notification_received_at",
        "notification_stated_effect_at",
        "hearing_session_event_time",
    ]
    .map(|field| json!({"kind":"source_field","field":field}))
    .to_vec();
    for purpose in ["hearing_end", "ordered_period_start"] {
        for family in ["resolution", "notification", "hearing_result"] {
            triggers.push(json!({"kind":"qualified","purpose":purpose,"family":family}));
        }
    }
    for trigger in triggers {
        let mut d = definition_json(None);
        d["trigger"] = trigger;
        round_trip(d).await;
    }
}
