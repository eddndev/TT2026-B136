use super::case_report_support::*;
use application::{case_reports::*, ApplicationError};
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    identity::Role,
};
use mockall::Sequence;

#[test]
fn list_accepts_empty_observation_and_passes_unread_filter_before_limit() {
    let who = actor(Role::Owner);
    let query = CaseReportQuery {
        limit: 2,
        after_id: Some(CaseReportId::new()),
        unread_only: true,
    };
    let expected_actor = who.clone();
    let mut store = MockStore::new();
    store
        .expect_list()
        .times(1)
        .withf(move |actor, seen, at| actor == &expected_actor && *seen == query && *at == now())
        .returning(|_, _, _| {
            Ok(CaseReportPage {
                checked_at: now(),
                reports: vec![],
                has_more: false,
                next_after_id: None,
            })
        });
    let result = service(store, identity(&who, 2))
        .list("session", query)
        .unwrap();
    assert!(result.reports.is_empty());
    assert_eq!(result.checked_at, now());
}

#[test]
fn list_rejects_foreign_rows_order_cursor_and_false_unread_claims() {
    let who = actor(Role::Owner);
    for mutation in 0..5 {
        let report = detail(&who, command());
        let mut page = CaseReportPage {
            checked_at: now(),
            reports: vec![report.clone()],
            has_more: false,
            next_after_id: None,
        };
        let query = CaseReportQuery {
            limit: 1,
            after_id: None,
            unread_only: mutation == 4,
        };
        match mutation {
            0 => page.reports[0].requester.principal.id = domain::identity::UserId::new(),
            1 => page.checked_at -= time::Duration::seconds(1),
            2 => page.reports.push(report),
            3 => page.has_more = true,
            _ => (),
        }
        let mut store = MockStore::new();
        store
            .expect_list()
            .times(1)
            .returning(move |_, _, _| Ok(page.clone()));
        inconsistent(service(store, identity(&who, 1)).list("session", query));
    }
    let report = detail(&who, command());
    let query = CaseReportQuery {
        limit: 1,
        after_id: Some(report.id),
        unread_only: false,
    };
    let mut store = MockStore::new();
    store.expect_list().times(1).returning(move |_, _, _| {
        Ok(CaseReportPage {
            checked_at: now(),
            reports: vec![report.clone()],
            has_more: false,
            next_after_id: None,
        })
    });
    inconsistent(service(store, identity(&who, 1)).list("session", query));
}

#[test]
fn download_verifies_exact_ready_capture_bytes_format_and_requester() {
    let who = actor(Role::Litigator);
    for mutation in 0..7 {
        let report = detail(&who, command());
        let capture = snapshot(&report);
        let mut download = ready(report, &capture);
        let id = download.report.id;
        match mutation {
            0 => download.artifact.report_id = CaseReportId::new(),
            1 => download.artifact.format = CaseReportFormat::Csv,
            2 => download.artifact.snapshot_digest = Sha256Digest::from_array([77; 32]),
            3 => download.artifact.content.push(b'x'),
            4 => download.artifact.requester.principal.email = "other@example.test".into(),
            5 => download.artifact.scope = CaseReportScope::Office,
            _ => {
                download.artifact.content = vec![b'x'; MAX_REPORT_ARTIFACT_BYTES + 1];
                download.artifact.digest = TestHasher.hash_bytes(&download.artifact.content);
            }
        }
        let mut store = MockStore::new();
        store
            .expect_download()
            .times(1)
            .returning(move |_, _, _, _| Ok(download.clone()));
        inconsistent(service(store, identity(&who, 1)).download(
            "session",
            id,
            CaseReportFormat::Pdf,
        ));
    }
}

#[test]
fn download_reauthenticates_after_read_and_never_marks_notice_read() {
    let who = actor(Role::Owner);
    let report = detail(&who, command());
    let capture = snapshot(&report);
    let download = ready(report, &capture);
    let id = download.report.id;
    let expected = download.clone();
    let mut store = MockStore::new();
    store
        .expect_download()
        .times(1)
        .returning(move |_, _, _, _| Ok(download.clone()));
    assert_eq!(
        service(store, identity(&who, 2))
            .download("session", id, CaseReportFormat::Pdf)
            .unwrap(),
        expected
    );
    assert!(expected.report.notice.unwrap().read_at.is_none());

    let mut store = MockStore::new();
    let report = detail(&who, command());
    let capture = snapshot(&report);
    let download = ready(report, &capture);
    let id = download.report.id;
    store
        .expect_download()
        .times(1)
        .returning(move |_, _, _, _| Ok(download.clone()));
    let mut auth = super::case_support::MockIdentity::new();
    let mut sequence = Sequence::new();
    let first = who.clone();
    let mut changed = who;
    changed.email = "changed@example.test".into();
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(first));
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(changed));
    assert!(matches!(
        service(store, auth).download("session", id, CaseReportFormat::Pdf),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn notice_acknowledgement_is_explicit_and_preserves_original_read_timestamp() {
    let who = actor(Role::Owner);
    let report = detail(&who, command());
    let mut capture = snapshot(&report);
    capture.checked_at -= time::Duration::seconds(4);
    capture.digest = case_report_snapshot_digest(&TestHasher, &capture).unwrap();
    let mut result = ready(report, &capture).report;
    result.notice.as_mut().unwrap().created_at -= time::Duration::seconds(3);
    result.notice.as_mut().unwrap().read_at = Some(now() - time::Duration::seconds(2));
    let expected = result.clone();
    let id = result.id;
    let mut store = MockStore::new();
    store
        .expect_acknowledge_notice()
        .times(1)
        .returning(move |_, _, _| Ok(result.clone()));
    assert_eq!(
        service(store, identity(&who, 2))
            .acknowledge_notice("session", id)
            .unwrap(),
        expected
    );
}

#[test]
fn durable_revocation_is_not_replaced_by_an_empty_report_or_download() {
    let who = actor(Role::Owner);
    let id = CaseReportId::new();
    let mut store = MockStore::new();
    store
        .expect_get()
        .times(1)
        .returning(|_, _, _| Err(CaseReportError::AccessRevoked.into()));
    store
        .expect_download()
        .times(1)
        .returning(|_, _, _, _| Err(CaseReportError::AccessRevoked.into()));
    let service = service(store, identity(&who, 2));
    assert!(matches!(
        service.get("session", id),
        Err(ApplicationError::CaseReport(CaseReportError::AccessRevoked))
    ));
    assert!(matches!(
        service.download("session", id, CaseReportFormat::Pdf),
        Err(ApplicationError::CaseReport(CaseReportError::AccessRevoked))
    ));
}

#[test]
fn get_reauthenticates_the_entire_principal_before_returning_metadata() {
    let who = actor(Role::Owner);
    let response = detail(&who, command());
    let id = response.id;
    let mut store = MockStore::new();
    store
        .expect_get()
        .times(1)
        .return_once(move |_, _, _| Ok(response));
    let mut auth = super::case_support::MockIdentity::new();
    let mut sequence = Sequence::new();
    let first = who.clone();
    let mut changed = who;
    changed.role = Role::Litigator;
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(first));
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(changed));
    assert!(matches!(
        service(store, auth).get("session", id),
        Err(ApplicationError::InvalidSession)
    ));
}
