use super::case_report_support::*;
use application::{case_reports::*, ApplicationError};
use domain::identity::Role;

#[test]
fn expired_lease_is_rejected_even_when_no_other_worker_claimed_it() {
    let report = detail(&actor(Role::Owner), command());
    let capture = snapshot(&report);
    let mut claimed = claim(report, Some(capture));
    claimed.lease.expires_at = now();
    let mut store = MockWorkerStore::new();
    store
        .expect_claim_next()
        .times(1)
        .return_once(move |_| Ok(Some(claimed)));
    assert!(matches!(
        worker(store, MockRenderer::new()).run_next(),
        Err(ApplicationError::CaseReport(CaseReportError::LeaseLost))
    ));
}

#[test]
fn renewal_cannot_change_fencing_identity_before_renderer() {
    for mutation in 0..4 {
        let report = detail(&actor(Role::Owner), command());
        let capture = snapshot(&report);
        let claimed = claim(report, Some(capture));
        let mut renewed = claimed.lease.clone();
        renewed.expires_at += time::Duration::seconds(60);
        match mutation {
            0 => renewed.report_id = CaseReportId::new(),
            1 => renewed.attempt_id = CaseReportAttemptId::new(),
            2 => renewed.token = CaseReportLeaseToken::new(),
            _ => renewed.generation += 1,
        }
        let mut store = MockWorkerStore::new();
        store
            .expect_claim_next()
            .times(1)
            .return_once(move |_| Ok(Some(claimed)));
        store
            .expect_renew()
            .times(1)
            .return_once(move |_, _| Ok(renewed));
        inconsistent(worker(store, MockRenderer::new()).run_next());
    }
}

#[test]
fn renderer_failure_records_safe_failure_and_publishes_no_partial_artifact() {
    let report = detail(&actor(Role::Owner), command());
    let capture = snapshot(&report);
    let claimed = claim(report, Some(capture));
    let id = claimed.report.id;
    let mut renewed = claimed.lease.clone();
    renewed.expires_at += time::Duration::seconds(60);
    let mut expected = renewed.clone();
    expected.expires_at += time::Duration::seconds(60);
    let final_lease = expected.clone();
    let mut store = MockWorkerStore::new();
    store
        .expect_claim_next()
        .times(1)
        .return_once(move |_| Ok(Some(claimed)));
    let mut sequence = mockall::Sequence::new();
    store
        .expect_renew()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_, _| Ok(renewed));
    store
        .expect_renew()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_, _| Ok(final_lease));
    store
        .expect_fail()
        .times(1)
        .withf(move |lease, failure, at| {
            *lease == expected && *failure == CaseReportFailure::RenderFailed && *at == now()
        })
        .return_once(move |_, _, _| Ok(CaseReportWorkerRun::Failed(id)));
    let mut renderer = MockRenderer::new();
    renderer
        .expect_render()
        .times(1)
        .withf(|_, format| *format == CaseReportFormat::Pdf)
        .return_once(|_, _| Ok(b"pdf".to_vec()));
    renderer
        .expect_render()
        .times(1)
        .withf(|_, format| *format == CaseReportFormat::Csv)
        .return_once(|_, _| Err(CaseReportError::RenderFailed.into()));
    assert_eq!(
        worker(store, renderer).run_next().unwrap(),
        CaseReportWorkerRun::Failed(id)
    );
}

#[test]
fn lost_fence_during_renewal_does_not_try_to_fail_newer_attempt() {
    let report = detail(&actor(Role::Owner), command());
    let capture = snapshot(&report);
    let claimed = claim(report, Some(capture));
    let mut store = MockWorkerStore::new();
    store
        .expect_claim_next()
        .times(1)
        .return_once(move |_| Ok(Some(claimed)));
    store
        .expect_renew()
        .times(1)
        .return_once(|_, _| Err(CaseReportError::LeaseLost.into()));
    assert!(matches!(
        worker(store, MockRenderer::new()).run_next(),
        Err(ApplicationError::CaseReport(CaseReportError::LeaseLost))
    ));
}

#[test]
fn invalid_capture_is_not_rendered_and_records_only_its_safe_failure() {
    let report = detail(&actor(Role::Owner), command());
    let mut capture = snapshot(&report);
    capture.cases[0].title = "Tampered".into();
    let claimed = claim(report, Some(capture));
    let id = claimed.report.id;
    let mut store = MockWorkerStore::new();
    store
        .expect_claim_next()
        .times(1)
        .return_once(move |_| Ok(Some(claimed)));
    store
        .expect_fail()
        .times(1)
        .withf(|_, failure, _| *failure == CaseReportFailure::InvalidStoredCapture)
        .return_once(move |_, _, _| Ok(CaseReportWorkerRun::Failed(id)));
    assert_eq!(
        worker(store, MockRenderer::new()).run_next().unwrap(),
        CaseReportWorkerRun::Failed(id)
    );
}

#[test]
fn completion_lease_loss_cannot_publish_or_fail_a_newer_attempt() {
    let report = detail(&actor(Role::Owner), command());
    let capture = snapshot(&report);
    let claimed = claim(report, Some(capture));
    let mut renewed = claimed.lease.clone();
    renewed.expires_at += time::Duration::seconds(60);
    let mut last = renewed.clone();
    last.expires_at += time::Duration::seconds(60);
    let mut store = MockWorkerStore::new();
    let mut sequence = mockall::Sequence::new();
    store
        .expect_claim_next()
        .times(1)
        .return_once(move |_| Ok(Some(claimed)));
    store
        .expect_renew()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_, _| Ok(renewed));
    store
        .expect_renew()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_, _| Ok(last));
    store
        .expect_complete()
        .times(1)
        .return_once(|_, _, _, _| Err(CaseReportError::LeaseLost.into()));
    let mut renderer = MockRenderer::new();
    renderer
        .expect_render()
        .times(2)
        .returning(|_, _| Ok(b"rendered".to_vec()));
    assert!(matches!(
        worker(store, renderer).run_next(),
        Err(ApplicationError::CaseReport(CaseReportError::LeaseLost))
    ));
}

#[test]
fn oversized_renderer_result_fails_before_second_format_or_publication() {
    let report = detail(&actor(Role::Owner), command());
    let capture = snapshot(&report);
    let claimed = claim(report, Some(capture));
    let id = claimed.report.id;
    let mut renewed = claimed.lease.clone();
    renewed.expires_at += time::Duration::seconds(60);
    let mut store = MockWorkerStore::new();
    store
        .expect_claim_next()
        .times(1)
        .return_once(move |_| Ok(Some(claimed)));
    store
        .expect_renew()
        .times(1)
        .return_once(move |_, _| Ok(renewed));
    store
        .expect_fail()
        .times(1)
        .withf(|_, failure, _| *failure == CaseReportFailure::CapacityExceeded)
        .return_once(move |_, _, _| Ok(CaseReportWorkerRun::Failed(id)));
    let mut renderer = MockRenderer::new();
    renderer
        .expect_render()
        .times(1)
        .withf(|_, format| *format == CaseReportFormat::Pdf)
        .return_once(|_, _| Ok(vec![b'x'; MAX_REPORT_ARTIFACT_BYTES + 1]));
    assert_eq!(
        worker(store, renderer).run_next().unwrap(),
        CaseReportWorkerRun::Failed(id)
    );
}
