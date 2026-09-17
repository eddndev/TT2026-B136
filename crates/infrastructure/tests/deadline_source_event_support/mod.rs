#![allow(dead_code)]

pub use super::case_administration_support::Fixture;
use super::{
    hearing_database_support as hearings, hearing_result_database_support as results,
    judicial_calendar_database_support as calendars, procedural_fact_backend_support as facts,
};
use application::{
    hearing_results::*, judicial_calendars::JudicialCalendarDetail, procedural_facts::FactDetail,
};
use domain::identity::Role;
use serde_json::Value;

pub const KINDS: [&str; 4] = ["resolution", "notification", "hearing_result", "calendar"];

pub enum Saved {
    Fact(Box<FactDetail>),
    Hearing(Box<HearingResultDetail>),
    Calendar(Box<JudicialCalendarDetail>),
}

pub fn record(db: &mut Fixture, kind: &str) -> Saved {
    match kind {
        "resolution" | "notification" => {
            let service = facts::service(db, db.owner, Role::Owner);
            let first = facts::persist(&service, db.case, facts::record());
            Saved::Fact(Box::new(if kind == "notification" {
                facts::persist(
                    &service,
                    db.case,
                    facts::notify(facts::resolution_ref(&first)),
                )
            } else {
                first
            }))
        }
        "hearing_result" => {
            hearings::complete(db);
            let hearing = hearings::persist(
                &hearings::service(db, db.owner, Role::Owner),
                db.case,
                hearings::schedule(),
            );
            Saved::Hearing(Box::new(results::persist(
                &results::service(db, db.owner, Role::Owner),
                db.case,
                results::record(hearing.snapshot.id),
            )))
        }
        "calendar" => Saved::Calendar(Box::new(calendars::persist(
            &calendars::service(db, db.owner, Role::Owner),
            calendars::publish(),
        ))),
        _ => panic!("unknown source family"),
    }
}

pub fn change(db: &Fixture, saved: &Saved, withdraw: bool) -> Saved {
    match saved {
        Saved::Fact(base) => Saved::Fact(Box::new(facts::persist(
            &facts::service(db, db.owner, Role::Owner),
            db.case,
            if withdraw {
                facts::withdraw(base)
            } else {
                facts::correct(base)
            },
        ))),
        Saved::Hearing(base) => Saved::Hearing(Box::new(results::persist(
            &results::service(db, db.owner, Role::Owner),
            db.case,
            hearing_change(base, withdraw),
        ))),
        Saved::Calendar(base) => Saved::Calendar(Box::new(calendars::persist(
            &calendars::service(db, db.owner, Role::Owner),
            if withdraw {
                calendars::retire(base)
            } else {
                calendars::replace(base)
            },
        ))),
    }
}

pub fn hearing_change(base: &HearingResultDetail, withdraw: bool) -> HearingResultCommand {
    let s = &base.snapshot;
    HearingResultCommand {
        operation_id: HearingResultOperationId::new(),
        hearing_id: s.hearing_id,
        result_id: s.id,
        change: if withdraw {
            HearingResultChange::Withdraw {
                expected_revision: s.revision,
                reason: HearingResultText::new("Withdraw declaration").unwrap(),
            }
        } else {
            HearingResultChange::Correct {
                expected_revision: s.revision,
                values: results::values("Corrected session"),
                reason: HearingResultText::new("Correct transcription").unwrap(),
            }
        },
    }
}

pub fn event_rows(db: &mut Fixture) -> Value {
    db.admin.query_one("SELECT coalesce(jsonb_agg(to_jsonb(e) ORDER BY source_kind,source_id,revision),'[]') FROM (SELECT source_kind,source_id,revision,case_id,hearing_id,operation_id FROM deadline_source_events) e", &[]).unwrap().get(0)
}

pub fn assert_events_match_sources(db: &mut Fixture) {
    let expected: Value = db.admin.query_one("SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY source_kind,source_id,revision),'[]') FROM (
        SELECT family AS source_kind,id AS source_id,revision,case_id,NULL::uuid AS hearing_id,operation_id FROM case_procedural_fact_revisions
        UNION ALL SELECT 'hearing_result',result_id,revision,case_id,hearing_id,operation_id FROM case_hearing_result_revisions
        UNION ALL SELECT 'calendar',calendar_id,revision,NULL::uuid,NULL::uuid,operation_id FROM judicial_calendar_revisions) r", &[]).unwrap().get(0);
    assert_eq!(event_rows(db), expected);
    let sequences: Vec<i64> = db
        .admin
        .query(
            "SELECT sequence FROM deadline_source_events ORDER BY sequence",
            &[],
        )
        .unwrap()
        .iter()
        .map(|r| r.get(0))
        .collect();
    assert!(!sequences.is_empty());
    assert!(sequences[0] > 0);
    assert!(sequences.windows(2).all(|pair| pair[0] < pair[1]));
}

mod atomicity;
mod integrity;
pub use atomicity::*;
pub use integrity::*;
mod concurrency;
