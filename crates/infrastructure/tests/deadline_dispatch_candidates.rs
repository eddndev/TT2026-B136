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

use application::deadline_dispatch::DeadlineDispatchStream;
use deadline_backend_support as dl;
use deadline_dispatch_support as dispatch;
use postgres::{error::SqlState, Client};
use uuid::Uuid;

type Window = (
    Option<i64>,
    Option<Uuid>,
    Option<bool>,
    Option<Uuid>,
    Option<bool>,
    Option<i32>,
);
const SELECT: &str = "SELECT deadline_id FROM deadline_dispatch_candidates($1,$2,$3,$4,$5,$6)";

fn candidates(client: &mut Client, window: Window) -> Result<Vec<Uuid>, postgres::Error> {
    client
        .query(
            SELECT,
            &[
                &window.0, &window.1, &window.2, &window.3, &window.4, &window.5,
            ],
        )
        .map(|rows| rows.into_iter().map(|row| row.get(0)).collect())
}

#[test]
fn candidate_bounds_reject_unbounded_or_ambiguous_arguments_without_changes() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let valid = (None, None, Some(false), None, Some(false), Some(1));
    let before = dispatch::snapshot(&mut db);
    assert!(candidates(&mut db.runtime(), valid).unwrap().is_empty());
    let mut invalid = Vec::new();
    for limit in [None, Some(0), Some(-1), Some(102), Some(i32::MAX)] {
        invalid.push((valid.0, valid.1, valid.2, valid.3, valid.4, limit));
    }
    for event in [Some(0), Some(-1)] {
        invalid.push((event, valid.1, valid.2, valid.3, valid.4, valid.5));
    }
    invalid.extend([
        (None, None, None, None, Some(false), Some(1)),
        (None, None, Some(false), None, None, Some(1)),
        (
            None,
            Some(Uuid::from_u128(2)),
            Some(false),
            Some(Uuid::from_u128(1)),
            Some(false),
            Some(1),
        ),
    ]);
    for window in invalid {
        let error = candidates(&mut db.runtime(), window).unwrap_err();
        assert_eq!(
            error.as_db_error().unwrap().code(),
            &SqlState::CHECK_VIOLATION
        );
    }
    assert_eq!(dispatch::snapshot(&mut db), before);
}

#[test]
fn candidate_window_applies_uuid_bounds_and_missing_jobs_before_limiting() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let profile = dl::profile(&db);
    let source = dl::source(&db);
    for value in [0, 10, 20, 30, 40, u128::MAX] {
        dispatch::legacy(&db, &profile, &source, value);
    }
    let mut client = db.runtime();
    let selected = |values: &[u128]| {
        values
            .iter()
            .copied()
            .map(Uuid::from_u128)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        candidates(
            &mut client,
            (None, None, Some(false), None, Some(false), Some(2))
        )
        .unwrap(),
        selected(&[0, 10])
    );
    assert_eq!(
        candidates(
            &mut client,
            (
                None,
                Some(Uuid::nil()),
                Some(true),
                Some(Uuid::nil()),
                Some(false),
                Some(1)
            )
        )
        .unwrap(),
        selected(&[0])
    );
    assert!(candidates(
        &mut client,
        (
            None,
            Some(Uuid::nil()),
            Some(false),
            Some(Uuid::nil()),
            Some(false),
            Some(1)
        )
    )
    .unwrap()
    .is_empty());
    assert_eq!(
        candidates(
            &mut client,
            (
                None,
                Some(Uuid::from_u128(10)),
                Some(false),
                Some(Uuid::from_u128(30)),
                Some(false),
                Some(101)
            )
        )
        .unwrap(),
        selected(&[20, 30])
    );
    assert_eq!(
        candidates(
            &mut client,
            (
                None,
                Some(Uuid::from_u128(20)),
                Some(true),
                Some(Uuid::from_u128(20)),
                Some(false),
                Some(1)
            )
        )
        .unwrap(),
        selected(&[20])
    );
    let maximum = Uuid::from_u128(u128::MAX);
    assert_eq!(
        candidates(
            &mut client,
            (
                None,
                Some(maximum),
                Some(true),
                Some(maximum),
                Some(false),
                Some(1)
            )
        )
        .unwrap(),
        vec![maximum]
    );
    assert!(candidates(
        &mut client,
        (None, Some(maximum), Some(false), None, Some(false), Some(1))
    )
    .unwrap()
    .is_empty());
    let store = dispatch::open(&db);
    let batch = dispatch::dispatch(&store, DeadlineDispatchStream::LegacyBootstrap, 2);
    assert_eq!(
        (batch.selected, batch.inserted, batch.completed_scan),
        (2, 2, false)
    );
    assert_eq!(
        candidates(
            &mut client,
            (None, None, Some(false), None, Some(true), Some(2))
        )
        .unwrap(),
        selected(&[20, 30])
    );
    assert_eq!(
        candidates(
            &mut client,
            (None, None, Some(false), None, Some(false), Some(2))
        )
        .unwrap(),
        selected(&[0, 10])
    );
    let event = i64::try_from(dispatch::event_sequence(&mut db, &source)).unwrap();
    assert_eq!(
        candidates(
            &mut client,
            (Some(event), None, Some(false), None, Some(true), Some(2))
        )
        .unwrap(),
        selected(&[0, 10])
    );
    let before = dispatch::snapshot(&mut db);
    assert!(candidates(
        &mut client,
        (
            Some(i64::MAX),
            None,
            Some(false),
            None,
            Some(false),
            Some(1)
        )
    )
    .unwrap()
    .is_empty());
    assert_eq!(dispatch::snapshot(&mut db), before);
}
