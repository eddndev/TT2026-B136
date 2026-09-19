use super::{object::Object, request::Command};
use application::{deadline_tracking::TrackingPolicy, deadlines::*};
use serde_json::{json, Value};

use super::tracking_response_test_support as fixtures;

fn policies() -> Value {
    json!({"profile":"fixed","source":"follow","calendar":"undetermined"})
}
fn qualification(correcting: bool) -> Value {
    let mut value = fixtures::command();
    value["change"]["tracking"] = policies();
    if correcting {
        value["change"]["action"] = json!("correct");
        value["change"]["expected_revision"] = json!(3);
        value["change"]["reason"] = json!("Review exact dependencies");
    }
    value
}
fn parse(text: &str) -> Result<DeadlineHumanCommand, ()> {
    serde_json::from_str::<Object<Command>>(text)
        .map_err(|_| ())?
        .0
        .validate()
        .map_err(|_| ())
}
fn reject(value: Value) {
    assert!(parse(&value.to_string()).is_err(), "{value}");
}

#[test]
fn qualifications_preserve_explicit_policies_and_command() {
    for correcting in [false, true] {
        for profile in ["fixed", "follow"] {
            for source in ["fixed", "follow"] {
                let mut value = qualification(correcting);
                value["change"]["tracking"]["profile"] = json!(profile);
                value["change"]["tracking"]["source"] = json!(source);
                let (command, policies) = parse(&value.to_string()).unwrap().into_parts();
                let policies = policies.unwrap();
                assert_eq!(
                    policies.profile == TrackingPolicy::Fixed,
                    profile == "fixed"
                );
                assert_eq!(
                    policies.source == TrackingPolicy::Follow,
                    source == "follow"
                );
                assert_eq!(policies.calendar, TrackingPolicy::Undetermined);
                assert_eq!(command.expected_revision(), if correcting { 3 } else { 0 });
                assert_eq!(command.deadline_id.to_string(), fixtures::ID);
                assert_eq!(command.operation_id.to_string(), fixtures::ID);
            }
        }
    }
}

#[test]
fn qualifications_require_a_named_complete_strict_tracking_object() {
    for correcting in [false, true] {
        for tracking in [
            Value::Null,
            json!([]),
            json!(true),
            json!("follow"),
            json!({}),
        ] {
            let mut value = qualification(correcting);
            value["change"]["tracking"] = tracking;
            reject(value);
        }
        for absent in ["tracking", "profile", "source", "calendar"] {
            let mut value = qualification(correcting);
            if absent == "tracking" {
                value["change"].as_object_mut().unwrap().remove(absent);
            } else {
                value["change"]["tracking"]
                    .as_object_mut()
                    .unwrap()
                    .remove(absent);
            }
            reject(value);
        }
        for field in ["profile", "source", "calendar"] {
            for invalid in [json!("automatic"), json!("Follow"), Value::Null, json!(1)] {
                let mut value = qualification(correcting);
                value["change"]["tracking"][field] = invalid;
                reject(value);
            }
        }
        let mut value = qualification(correcting);
        value["change"]["tracking"]["review_state"] = json!("accepted");
        reject(value);
    }
}

#[test]
fn tracking_policy_meaning_matches_present_dependencies() {
    for correcting in [false, true] {
        for (field, invalid) in [
            ("profile", "undetermined"),
            ("source", "undetermined"),
            ("calendar", "fixed"),
        ] {
            let mut value = qualification(correcting);
            value["change"]["tracking"][field] = json!(invalid);
            reject(value);
        }
        let mut value = qualification(correcting);
        value["change"]["definition"]["input"]["selection"]["source"] =
            json!({"kind":"unknown","reason":"Exact source not identified"});
        reject(value.clone());
        value["change"]["tracking"]["source"] = json!("undetermined");
        value["change"]["definition"]["input"]["calendar"] =
            json!({"id":fixtures::ID,"revision":2});
        reject(value.clone());
        value["change"]["tracking"]["calendar"] = json!("follow");
        let (_, policies) = parse(&value.to_string()).unwrap().into_parts();
        let policies = policies.unwrap();
        assert_eq!(policies.source, TrackingPolicy::Undetermined);
        assert_eq!(policies.calendar, TrackingPolicy::Follow);
    }
}

#[test]
fn attention_and_retirement_never_accept_replacement_policies() {
    for action in ["set_attention", "retire"] {
        let mut value = json!({"operation_id":fixtures::ID,"deadline_id":fixtures::ID,
            "change":{"action":action,"expected_revision":1,"reason":"Declared action"}});
        if action == "set_attention" {
            value["change"]["attention"] = json!({"status":"pending"});
        }
        let (command, policies) = parse(&value.to_string()).unwrap().into_parts();
        assert_eq!(command.action().as_str(), action);
        assert!(policies.is_none());
        for replacement in [Value::Null, json!({}), self::policies()] {
            value["change"]["tracking"] = replacement;
            reject(value.clone());
        }
    }
}

#[test]
fn duplicate_policies_and_client_supplied_provenance_are_rejected() {
    let value = qualification(false).to_string();
    let duplicate = value.replace(
        "\"profile\":\"fixed\"",
        "\"profile\":\"fixed\",\"profile\":\"follow\"",
    );
    assert_ne!(duplicate, value);
    assert!(parse(&duplicate).is_err());
    for field in [
        "author",
        "observations",
        "cause",
        "review_state",
        "predecessor",
    ] {
        let mut value = qualification(false);
        value["change"][field] = json!({});
        reject(value);
    }
    let mut value = qualification(false);
    value["change"]["action"] = json!("reevaluate");
    reject(value);
}
