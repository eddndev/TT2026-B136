use crate::case_report_support::*;
use application::{case_reports::*, cases::*, ApplicationError};
use domain::{case_administration::CaseAdministrativeStatus, crypto::DocumentHasher};
use std::sync::{Arc, Barrier};
use time::Duration;

#[test]
fn exact_operation_replay_is_stable_and_changed_filter_conflicts_without_new_audit() {
    let Some(mut db) = fixture() else { return };
    let owner = owner(&mut db);
    let timer = clock(db.at);
    let store = store(&db, timer.clone());
    let command = command(db.at);
    let first = request(&store, &owner, command.clone(), db.at).unwrap();
    timer.set(db.at + Duration::seconds(1));
    let repeated = request(
        &store,
        &owner,
        command.clone(),
        db.at + Duration::seconds(1),
    )
    .unwrap();
    assert_eq!(repeated, first);
    assert_eq!(first.requester.principal, owner);
    assert_eq!(first.scope, CaseReportScope::Office);
    let mut changed = command;
    changed.filters.status = CaseStatusFilter::Closed;
    assert!(matches!(
        request(&store, &owner, changed, db.at),
        Err(ApplicationError::CaseReport(
            CaseReportError::OperationConflict
        ))
    ));
    assert_eq!(count(&mut db, "case_report_jobs"), 1);
    assert_eq!(audits(&mut db, "case_report.request"), 1);
}

#[test]
fn concurrent_identical_requests_share_one_job_and_creation_event() {
    let Some(mut db) = fixture() else { return };
    let owner = owner(&mut db);
    let command = command(db.at);
    let stores = [store(&db, clock(db.at)), store(&db, clock(db.at))];
    let barrier = Arc::new(Barrier::new(2));
    let ids = std::thread::scope(|scope| {
        let tasks: Vec<_> = stores
            .into_iter()
            .map(|store| {
                let actor = owner.clone();
                let command = command.clone();
                let barrier = barrier.clone();
                let at = db.at;
                scope.spawn(move || {
                    barrier.wait();
                    request(&store, &actor, command, at).unwrap().id
                })
            })
            .collect();
        tasks
            .into_iter()
            .map(|task| task.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(ids[0], ids[1]);
    assert_eq!(count(&mut db, "case_report_jobs"), 1);
    assert_eq!(audits(&mut db, "case_report.request"), 1);
}

#[test]
fn owner_and_assigned_litigator_snapshots_never_mix_foreign_case_metadata() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let visible = seed_case(&mut db, "Visible", at);
    let hidden = seed_case(&mut db, "Foreign secret title", at);
    db.case = visible;
    let litigant = db.user("litigator", true);
    let colleague = db.user("litigator", true);
    db.case = hidden;
    let foreign = db.user("litigator", true);
    let owner = owner(&mut db);
    let actor = principal(&mut db, litigant);
    let store = store(&db, clock(at));
    let all = capture_request(&store, &owner, command(at), at);
    assert_eq!(
        all.cases.iter().map(|row| row.case_id).collect::<Vec<_>>(),
        sorted([visible, hidden])
    );
    let assigned = capture_request(&store, &actor, command(at), at);
    assert_eq!(assigned.scope, CaseReportScope::AssignedCases);
    assert_eq!(assigned.cases.len(), 1);
    assert_eq!(assigned.cases[0].case_id, visible);
    assert_eq!(assigned.cases[0].title, "Visible");
    assert_eq!(assigned.workload.len(), 2);
    assert!(assigned
        .workload
        .iter()
        .all(|row| [litigant, colleague].contains(&row.litigator.user_id)
            && row.active_cases == 1
            && row.closed_cases == 0));
    assert!(assigned
        .workload
        .iter()
        .all(|row| row.litigator.user_id != foreign));
    assert!(!format!("{assigned:?}").contains("Foreign secret title"));
}

#[test]
fn denied_roles_and_other_owners_cannot_create_read_or_download_private_reports() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let owner = owner(&mut db);
    let store = store(&db, clock(at));
    let report = request(&store, &owner, command(at), at).unwrap();
    for role in ["client", "paralegal"] {
        let id = db.user(role, true);
        let actor = principal(&mut db, id);
        assert!(matches!(
            store.request(
                &actor,
                CaseReportScope::Office,
                command(at),
                infrastructure::RingSha256Hasher.hash_bytes(b"unauthorized"),
                at
            ),
            Err(ApplicationError::PermissionDenied)
        ));
        assert!(matches!(
            store.list(&actor, query(), at),
            Err(ApplicationError::PermissionDenied)
        ));
    }
    let second_owner = db.user("owner", false);
    let other = principal(&mut db, second_owner);
    assert!(store.list(&other, query(), at).unwrap().reports.is_empty());
    assert!(matches!(
        store.get(&other, report.id, at),
        Err(ApplicationError::CaseReport(CaseReportError::NotFound))
    ));
    assert!(matches!(
        store.download(&other, report.id, CaseReportFormat::Csv, at),
        Err(ApplicationError::CaseReport(CaseReportError::NotFound))
    ));
}

#[test]
fn filters_use_original_half_open_creation_interval_and_current_state_and_assignment() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let before = seed_case(&mut db, "Before", at - Duration::microseconds(1));
    let first = seed_case(&mut db, "At start", at);
    let last = seed_case(
        &mut db,
        "Before end",
        at + Duration::days(1) - Duration::microseconds(1),
    );
    let after = seed_case(&mut db, "At end", at + Duration::days(1));
    db.case = first;
    let litigator = db.user("litigator", true);
    let observed = at + Duration::days(2);
    let cases = db.store();
    cases
        .add_member(last, litigator, db.owner, observed)
        .unwrap();
    cases
        .change_administrative_status(
            db.owner,
            last,
            CaseRevisionExpectation::Revision(CaseRevision::FIRST),
            CaseAdministrativeStatus::Closed,
            observed,
        )
        .unwrap();
    let expected = cases.get_administration(db.owner, last, observed).unwrap();
    let CurrentCaseAdministration::Recorded(expected) = expected.administration else {
        panic!("status change must create an administration revision");
    };
    let owner = owner(&mut db);
    let store = store(&db, clock(observed));
    let mut filtered = command(at);
    filtered.filters.created_before = at + Duration::days(1);
    filtered.filters.assigned_litigator = Some(litigator);
    let snapshot = capture_request(&store, &owner, filtered.clone(), observed);
    assert_eq!(
        snapshot
            .cases
            .iter()
            .map(|row| row.case_id)
            .collect::<Vec<_>>(),
        sorted([first, last])
    );
    assert!(snapshot
        .cases
        .iter()
        .all(|row| ![before, after].contains(&row.case_id)));
    assert_eq!(snapshot.workload.len(), 1);
    assert_eq!(
        (
            snapshot.workload[0].active_cases,
            snapshot.workload[0].closed_cases
        ),
        (1, 1)
    );
    filtered.operation_id = CaseReportOperationId::new();
    filtered.filters.status = CaseStatusFilter::Closed;
    let closed = capture_request(&store, &owner, filtered, observed);
    assert_eq!(closed.cases.len(), 1);
    assert_eq!(closed.cases[0].case_id, last);
    assert_eq!(
        closed.cases[0].created_at,
        at + Duration::days(1) - Duration::microseconds(1)
    );
    assert_eq!(closed.cases[0].administration_revision.unwrap().get(), 2);
    assert_eq!(
        closed.cases[0].administration_digest,
        Some(expected.values_digest)
    );
    assert_eq!(closed.checked_at, observed);
}

#[test]
fn selected_case_capacity_rejects_instead_of_publishing_a_truncated_capture() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    for number in 0..=MAX_REPORT_CASES {
        seed_case(&mut db, &format!("Capacity {number}"), at);
    }
    let owner = owner(&mut db);
    let store = store(&db, clock(at));
    request(&store, &owner, command(at), at).unwrap();
    let claim = store.claim_next(at).unwrap().unwrap();
    assert!(matches!(
        store.capture(&claim.lease, at),
        Err(ApplicationError::CaseReport(
            CaseReportError::CapacityExceeded
        ))
    ));
    assert_eq!(count(&mut db, "case_report_snapshots"), 0);
    assert_eq!(count(&mut db, "case_report_artifacts"), 0);
    assert_eq!(audits(&mut db, "case_report.capture"), 0);
}
