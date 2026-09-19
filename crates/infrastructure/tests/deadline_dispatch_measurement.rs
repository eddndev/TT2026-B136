mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
#[allow(dead_code)]
mod deadline_dispatch_support;
mod deadline_profile_database_support;
mod procedural_fact_backend_support;

use application::deadline_dispatch::*;
use deadline_backend_support as dl;
use deadline_dispatch_support as dispatch;
use domain::identity::Role;
use serde_json::{json, Value};
use std::time::Instant;

#[test]
fn measured_pages_preserve_all_jobs_at_minimum_default_and_maximum_limits() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let profile = dl::profile(&db);
    let mut source = dl::source(&db);
    let store = dispatch::open(&db);
    dispatch::drain_existing(&mut db, &store);
    let workflow = dl::service(&db, db.owner, Role::Owner);
    let mut expected = Vec::new();
    for value in 0..240 {
        let command = dispatch::command(&db, &profile, &source, value);
        expected.push(command.deadline_id);
        dl::persist(&workflow, db.case, command);
    }
    let mut measured = Vec::new();
    for limit in [1, 20, 100] {
        source = dispatch::advance(&db, &source);
        let sequence = dispatch::event_sequence(&mut db, &source);
        let plan: Value = db
            .runtime()
            .query_one(
                "EXPLAIN (ANALYZE,BUFFERS,FORMAT JSON)
            SELECT c.deadline_id,c.case_id FROM deadline_dispatch_candidates(
            $1,$2,FALSE,NULL,FALSE,$3) c ORDER BY c.deadline_id",
                &[
                    &i64::try_from(sequence).unwrap(),
                    &dispatch::id(119).as_uuid(),
                    &(i32::try_from(limit).unwrap() + 1),
                ],
            )
            .unwrap()
            .get(0);
        let mut pages = Vec::new();
        let mut total = 0;
        loop {
            let start = Instant::now();
            let batch = dispatch::dispatch(&store, DeadlineDispatchStream::Events, limit);
            pages.push(start.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(batch.event.unwrap().sequence, sequence);
            assert_eq!(batch.selected, batch.inserted);
            assert!(batch.selected <= limit);
            total += batch.inserted;
            if batch.completed_scan {
                break;
            }
            assert!(pages.len() <= 240, "dispatch must make bounded progress");
        }
        assert_eq!(total, 240);
        assert_eq!(dispatch::event_jobs(&mut db, sequence), expected);
        measured.push(json!({"limit": limit, "pages_ms": pages, "query_plan": plan}));
    }
    println!(
        "dispatch_measurement={}",
        json!({"deadlines": 240, "campaigns": measured})
    );
}
