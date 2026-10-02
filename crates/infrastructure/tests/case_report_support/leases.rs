use crate::case_report_support::*;
use application::{case_reports::*, cases::*, ApplicationError};
use domain::case_administration::CaseAdministrativeStatus;
use std::sync::{Arc, Barrier};
use time::Duration;

#[test]
fn simultaneous_workers_get_one_live_claim_and_expired_claim_is_fenced_after_reopen() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    seed_case(&mut db, "Lease case", at);
    let actor = owner(&mut db);
    let timer = clock(at);
    let first = store(&db, timer.clone());
    let report = request(&first, &actor, command(at), at).unwrap();
    let other = store(&db, timer.clone());
    let barrier = Arc::new(Barrier::new(2));
    let claims = std::thread::scope(|scope| {
        let tasks: Vec<_> = [&first, &other]
            .into_iter()
            .map(|store| {
                let barrier = barrier.clone();
                scope.spawn(move || {
                    barrier.wait();
                    store.claim_next(at).unwrap()
                })
            })
            .collect();
        tasks
            .into_iter()
            .filter_map(|task| task.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(claims.len(), 1);
    let claim = &claims[0];
    assert_eq!(claim.lease.report_id, report.id);
    let snapshot = first.capture(&claim.lease, at).unwrap();
    drop(first);
    drop(other);
    let later = claim.lease.expires_at + Duration::seconds(1);
    timer.set(later);
    let resumed = store(&db, timer);
    let replacement = resumed.claim_next(later).unwrap().unwrap();
    assert_eq!(replacement.lease.report_id, report.id);
    assert!(replacement.lease.generation > claim.lease.generation);
    assert_ne!(replacement.lease.token, claim.lease.token);
    assert_ne!(replacement.lease.attempt_id, claim.lease.attempt_id);
    assert_eq!(replacement.snapshot, Some(snapshot.clone()));
    assert_lease_lost(resumed.renew(&claim.lease, later));
    assert_lease_lost(resumed.complete(&claim.lease, &snapshot, artifacts(&snapshot), later));
    assert_lease_lost(resumed.fail(&claim.lease, CaseReportFailure::RenderFailed, later));
    let ready = resumed
        .complete(&replacement.lease, &snapshot, artifacts(&snapshot), later)
        .unwrap();
    assert!(matches!(ready.state, CaseReportState::Ready { .. }));
    assert_eq!(count(&mut db, "case_report_notices"), 1);
}

#[test]
fn expired_owner_cannot_complete_even_before_another_worker_reclaims() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    seed_case(&mut db, "Expiry case", at);
    let actor = owner(&mut db);
    let timer = clock(at);
    let store = store(&db, timer.clone());
    request(&store, &actor, command(at), at).unwrap();
    let claim = store.claim_next(at).unwrap().unwrap();
    let snapshot = store.capture(&claim.lease, at).unwrap();
    let expiry = claim.lease.expires_at;
    timer.set(expiry);
    assert_lease_lost(store.complete(&claim.lease, &snapshot, artifacts(&snapshot), expiry));
    assert_eq!(count(&mut db, "case_report_artifacts"), 0);
    assert_eq!(count(&mut db, "case_report_notices"), 0);
}

#[test]
fn captured_case_values_and_workload_survive_later_case_changes_and_worker_restart() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let case = seed_case(&mut db, "Captured active", at);
    db.case = case;
    db.user("litigator", true);
    let actor = owner(&mut db);
    let timer = clock(at);
    let first = store(&db, timer.clone());
    request(&first, &actor, command(at), at).unwrap();
    let claim = first.claim_next(at).unwrap().unwrap();
    let snapshot = first.capture(&claim.lease, at).unwrap();
    assert_eq!(snapshot.cases[0].status, CaseAdministrativeStatus::Active);
    db.store()
        .change_administrative_status(
            db.owner,
            case,
            CaseRevisionExpectation::Revision(CaseRevision::FIRST),
            CaseAdministrativeStatus::Closed,
            at,
        )
        .unwrap();
    assert_eq!(first.capture(&claim.lease, at).unwrap(), snapshot);
    drop(first);
    let later = claim.lease.expires_at + Duration::seconds(1);
    timer.set(later);
    let restarted = store(&db, timer);
    let next = restarted.claim_next(later).unwrap().unwrap();
    assert_eq!(next.snapshot, Some(snapshot.clone()));
    assert_eq!(restarted.capture(&next.lease, later).unwrap(), snapshot);
    let ready = restarted
        .complete(&next.lease, &snapshot, artifacts(&snapshot), later)
        .unwrap();
    match ready.state {
        CaseReportState::Ready {
            checked_at,
            snapshot_digest,
            ..
        } => {
            assert_eq!(checked_at, snapshot.checked_at);
            assert_eq!(snapshot_digest, snapshot.digest);
        }
        state => panic!("unexpected report state: {state:?}"),
    }
    assert_eq!(audits(&mut db, "case_report.capture"), 1);
}

#[test]
fn assignment_loss_during_render_prevents_publication_and_current_reads() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let case = seed_case(&mut db, "Assigned report", at);
    db.case = case;
    let id = db.user("litigator", true);
    let actor = principal(&mut db, id);
    let store = store(&db, clock(at));
    let report = request(&store, &actor, command(at), at).unwrap();
    let claim = store.claim_next(at).unwrap().unwrap();
    let snapshot = store.capture(&claim.lease, at).unwrap();
    db.store().remove_member(case, id, db.owner, at).unwrap();
    assert_revoked(store.complete(&claim.lease, &snapshot, artifacts(&snapshot), at));
    assert_revoked(store.get(&actor, report.id, at));
    assert_eq!(count(&mut db, "case_report_artifacts"), 0);
}

#[test]
fn restoring_account_role_cannot_revive_a_ready_report_or_operation_replay() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    seed_case(&mut db, "Original office scope", at);
    let actor = owner(&mut db);
    let store = store(&db, clock(at));
    let command = command(at);
    let ready = finish(&store, &actor, command.clone(), at);
    db.user("owner", false);
    db.admin.execute("UPDATE users SET role='litigator',revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
        &[&actor.id.as_uuid()]).unwrap();
    db.admin.execute("UPDATE users SET role='owner',revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
        &[&actor.id.as_uuid()]).unwrap();
    assert_revoked(request(&store, &actor, command, at));
    assert_revoked(store.get(&actor, ready.id, at));
    assert_revoked(store.download(&actor, ready.id, CaseReportFormat::Pdf, at));
    assert!(store.list(&actor, query(), at).unwrap().reports.is_empty());
}

#[test]
fn losing_one_assignment_denies_the_entire_ready_report_before_artifact_exposure() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let first = seed_case(&mut db, "First", at);
    let second = seed_case(&mut db, "Second", at);
    db.case = first;
    let id = db.user("litigator", true);
    db.store().add_member(second, id, db.owner, at).unwrap();
    let actor = principal(&mut db, id);
    let store = store(&db, clock(at));
    let ready = finish(&store, &actor, command(at), at);
    db.store().remove_member(second, id, db.owner, at).unwrap();
    for format in [CaseReportFormat::Pdf, CaseReportFormat::Csv] {
        assert_revoked(store.download(&actor, ready.id, format, at));
    }
    assert_eq!(audits(&mut db, "case_report.download"), 0);
}

fn assert_lease_lost<T>(result: Result<T, ApplicationError>) {
    assert!(matches!(
        result,
        Err(ApplicationError::CaseReport(CaseReportError::LeaseLost))
    ));
}
fn assert_revoked<T>(result: Result<T, ApplicationError>) {
    assert!(matches!(
        result,
        Err(ApplicationError::CaseReport(CaseReportError::AccessRevoked))
    ));
}
