use crate::case_report_support::*;
use application::{case_reports::*, identity::Principal};
use domain::clock::{Clock, OffsetDateTime};
use infrastructure::PostgresCaseReportStore;
use time::Duration;

#[test]
fn unknown_initial_estimate_survives_replay_reopen_and_new_history() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let actor = owner(&mut db);
    let clock = clock(at);
    let store = store(&db, clock.clone());
    let command = command(at);
    let first = request(&store, &actor, command.clone(), at).unwrap();
    assert_eq!(first.estimate, None);
    let claim = store.claim_next(at).unwrap().unwrap();
    let snapshot = store.capture(&claim.lease, at).unwrap();
    clock.set(at + Duration::seconds(4));
    store
        .complete(&claim.lease, &snapshot, artifacts(&snapshot), clock.now())
        .unwrap();
    drop(store);
    let reopened = crate::case_report_support::store(&db, clock.clone());
    let repeated = request(&reopened, &actor, command, clock.now()).unwrap();
    assert_eq!(repeated.id, first.id);
    assert_eq!(repeated.requested_at, first.requested_at);
    assert_eq!(repeated.estimate, None);
    assert_eq!(audits(&mut db, "case_report.request"), 1);
    assert!(db
        .runtime()
        .execute(
            "UPDATE case_report_jobs SET estimated_total_ms=4000 WHERE id=$1",
            &[&first.id.as_uuid()],
        )
        .is_err());
    assert!(db
        .admin
        .execute(
            "UPDATE case_report_jobs SET estimated_total_ms=4000 WHERE id=$1",
            &[&first.id.as_uuid()],
        )
        .is_err());
}

#[test]
fn original_prediction_uses_service_history_and_actual_uuid_queue_order() {
    let Some(mut db) = fixture() else { return };
    let actor = owner(&mut db);
    let clock = clock(db.at);
    let store = store(&db, clock.clone());
    for _ in 0..3 {
        let now = clock.now();
        request(&store, &actor, command(db.at), now).unwrap();
        let claim = store.claim_next(now).unwrap().unwrap();
        let snapshot = store.capture(&claim.lease, now).unwrap();
        clock.set(now + Duration::seconds(4));
        store
            .complete(&claim.lease, &snapshot, artifacts(&snapshot), clock.now())
            .unwrap();
        clock.set(clock.now() + Duration::seconds(1));
    }
    let mut previous = Vec::new();
    for _ in 0..6 {
        let command = command(db.at);
        let report = request(&store, &actor, command.clone(), clock.now()).unwrap();
        let ahead = previous
            .iter()
            .filter(|&&id| id < report.id.as_uuid())
            .count();
        assert_eq!(
            report.estimate.unwrap().total_ms(),
            5_000 * (ahead as u64 + 1)
        );
        previous.push(report.id.as_uuid());
        let repeated = request(&store, &actor, command, clock.now()).unwrap();
        assert_eq!(repeated.estimate, report.estimate);
    }
    let first = store.claim_next(clock.now()).unwrap().unwrap();
    store.capture(&first.lease, clock.now()).unwrap();
    clock.set(clock.now() + Duration::seconds(1));
    let requested = request(&store, &actor, command(db.at), clock.now()).unwrap();
    let waiting = previous
        .iter()
        .filter(|&&id| id != first.report.id.as_uuid() && id < requested.id.as_uuid())
        .count() as u64;
    assert_eq!(
        requested.estimate.unwrap().total_ms(),
        8_000 + waiting * 5_000
    );
}

#[test]
fn completed_retries_do_not_replace_normal_service_history_with_backoff_time() {
    let Some(mut db) = fixture() else { return };
    let actor = owner(&mut db);
    let clock = clock(db.at);
    let store = store(&db, clock.clone());
    normal_history(&store, &actor, &clock, db.at);
    for _ in 0..3 {
        let resumed = retry_claim(&store, &actor, &clock, db.at);
        let snapshot = resumed.snapshot.as_ref().unwrap();
        clock.set(clock.now() + Duration::seconds(4));
        store
            .complete(&resumed.lease, snapshot, artifacts(snapshot), clock.now())
            .unwrap();
        clock.set(clock.now() + Duration::seconds(1));
    }
    let requested = request(&store, &actor, command(db.at), clock.now()).unwrap();
    assert_eq!(
        requested.estimate.unwrap().total_ms(),
        5_000,
        "retry backoff is not single-attempt generation cost"
    );
    let service: Option<i64> = db
        .admin
        .query_one(
            "SELECT estimated_service_ms FROM case_report_jobs WHERE id=$1",
            &[&requested.id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(service, Some(4_000));
}

#[test]
fn active_retry_makes_total_unknown_without_discarding_known_service_cost() {
    let Some(mut db) = fixture() else { return };
    let actor = owner(&mut db);
    let clock = clock(db.at);
    let store = store(&db, clock.clone());
    normal_history(&store, &actor, &clock, db.at);
    let resumed = retry_claim(&store, &actor, &clock, db.at);
    assert_eq!(resumed.lease.generation, 2);
    let requested = request(&store, &actor, command(db.at), clock.now()).unwrap();
    assert_eq!(
        requested.estimate, None,
        "an immutable old capture does not identify the active retry start"
    );
    let service: Option<i64> = db
        .admin
        .query_one(
            "SELECT estimated_service_ms FROM case_report_jobs WHERE id=$1",
            &[&requested.id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(service, Some(4_000));
}

fn normal_history(
    store: &PostgresCaseReportStore,
    actor: &Principal,
    clock: &MutableClock,
    filters_at: OffsetDateTime,
) {
    for _ in 0..3 {
        let now = clock.now();
        request(store, actor, command(filters_at), now).unwrap();
        let claim = store.claim_next(now).unwrap().unwrap();
        let snapshot = store.capture(&claim.lease, now).unwrap();
        clock.set(now + Duration::seconds(4));
        store
            .complete(&claim.lease, &snapshot, artifacts(&snapshot), clock.now())
            .unwrap();
        clock.set(clock.now() + Duration::seconds(1));
    }
}

fn retry_claim(
    store: &PostgresCaseReportStore,
    actor: &Principal,
    clock: &MutableClock,
    filters_at: OffsetDateTime,
) -> CaseReportClaim {
    let at = clock.now();
    let requested = request(store, actor, command(filters_at), at).unwrap();
    let first = store.claim_next(at).unwrap().unwrap();
    assert_eq!(first.report.id, requested.id);
    let snapshot = store.capture(&first.lease, at).unwrap();
    assert_eq!(
        store
            .fail(&first.lease, CaseReportFailure::RenderUnavailable, at)
            .unwrap(),
        CaseReportWorkerRun::Deferred(requested.id)
    );
    clock.set(at + Duration::seconds(30));
    let resumed = store.claim_next(clock.now()).unwrap().unwrap();
    assert_eq!(resumed.report.id, requested.id);
    assert_eq!(resumed.snapshot, Some(snapshot));
    resumed
}
