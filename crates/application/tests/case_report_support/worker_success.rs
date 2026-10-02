use super::case_report_support::*;
use application::case_reports::*;
use domain::{crypto::DocumentHasher, identity::Role};
use mockall::Sequence;

#[test]
fn idle_worker_does_not_render_capture_or_publish() {
    let mut store = MockWorkerStore::new();
    store.expect_claim_next().times(1).returning(|_| Ok(None));
    assert_eq!(
        worker(store, MockRenderer::new()).run_next().unwrap(),
        CaseReportWorkerRun::Idle
    );
}

#[test]
fn worker_renders_both_formats_from_one_capture_and_restart_reuses_it() {
    for resumed in [false, true] {
        let report = detail(&actor(Role::Litigator), command());
        let capture = snapshot(&report);
        let claimed = claim(report.clone(), resumed.then(|| capture.clone()));
        let lease = claimed.lease.clone();
        let mut completed = ready(report, &capture).report;
        if let CaseReportState::Ready { artifacts, .. } = &mut completed.state {
            for metadata in artifacts {
                let bytes = if metadata.format == CaseReportFormat::Pdf {
                    b"pdf"
                } else {
                    b"csv"
                };
                metadata.bytes = bytes.len() as u64;
                metadata.digest = TestHasher.hash_bytes(bytes);
            }
        }
        let id = completed.id;
        let mut store = MockWorkerStore::new();
        let mut sequence = Sequence::new();
        store
            .expect_claim_next()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| Ok(Some(claimed)));
        if !resumed {
            let expected_lease = lease.clone();
            let captured = capture.clone();
            store
                .expect_capture()
                .times(1)
                .in_sequence(&mut sequence)
                .withf(move |seen, at| *seen == expected_lease && *at == now())
                .return_once(move |_, _| Ok(captured));
        }
        let mut renderer = MockRenderer::new();
        for (index, format) in [CaseReportFormat::Pdf, CaseReportFormat::Csv]
            .into_iter()
            .enumerate()
        {
            let expected = lease.clone();
            let mut renewed = lease.clone();
            renewed.expires_at += time::Duration::seconds((index as i64 + 1) * 60);
            store
                .expect_renew()
                .times(1)
                .in_sequence(&mut sequence)
                .withf(move |seen, at| {
                    seen.report_id == expected.report_id
                        && seen.attempt_id == expected.attempt_id
                        && seen.token == expected.token
                        && seen.generation == expected.generation
                        && *at == now()
                })
                .return_once(move |_, _| Ok(renewed));
            let expected_capture = capture.clone();
            renderer
                .expect_render()
                .times(1)
                .in_sequence(&mut sequence)
                .withf(move |seen, actual| *seen == expected_capture && *actual == format)
                .return_once(move |_, _| {
                    Ok(if format == CaseReportFormat::Pdf {
                        b"pdf".to_vec()
                    } else {
                        b"csv".to_vec()
                    })
                });
        }
        let expected_capture = capture;
        store
            .expect_complete()
            .times(1)
            .in_sequence(&mut sequence)
            .withf(move |seen, snapshot, artifacts, at| {
                seen.report_id == id
                    && seen.expires_at == lease.expires_at + time::Duration::seconds(120)
                    && *snapshot == expected_capture
                    && *at == now()
                    && artifacts.len() == 2
                    && artifacts.iter().all(|artifact| {
                        artifact.report_id == id
                            && artifact.snapshot_digest == expected_capture.digest
                            && artifact.requester == expected_capture.requester
                            && artifact.scope == expected_capture.scope
                            && artifact.digest == TestHasher.hash_bytes(&artifact.content)
                    })
                    && artifacts[0].format == CaseReportFormat::Pdf
                    && artifacts[1].format == CaseReportFormat::Csv
            })
            .return_once(move |_, _, _, _| Ok(completed));
        assert_eq!(
            worker(store, renderer).run_next().unwrap(),
            CaseReportWorkerRun::Ready(id)
        );
    }
}
