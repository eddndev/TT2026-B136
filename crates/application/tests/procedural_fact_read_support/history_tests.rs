use super::*;

#[test]
fn history_validates_exact_receipts_in_descending_order_and_exclusive_cursor() {
    let case = CaseId::new();
    let (identity, actor) = case_support::identity(Role::Paralegal, 1);
    let entries: Vec<_> = [3, 2]
        .into_iter()
        .map(|revision| {
            FactHistoryEntry::from(&resolution_detail(case, actor.id, revision).snapshot)
        })
        .collect();
    let page = FactHistoryPage {
        revisions: entries,
        has_more: true,
        next_before_revision: Some(FactRevision::new(2).unwrap()),
    };
    let expected = page.clone();
    let mut store = MockReads::new();
    store
        .expect_history()
        .returning(move |_, _, _, _, _| Ok(page.clone()));
    let (service, _) = service(store, identity);
    assert_eq!(
        service
            .history(
                "session",
                case,
                FactTarget::Resolution(id(10)),
                FactHistoryQuery::new(2, Some(4)).unwrap()
            )
            .unwrap(),
        expected
    );
}
#[test]
fn history_rejects_scope_target_receipt_order_limits_and_cursor_contradictions() {
    let case = CaseId::new();
    let actor = UserId::new();
    for defect in 0..11 {
        let mut page = FactHistoryPage {
            revisions: [3, 2]
                .into_iter()
                .map(|revision| {
                    FactHistoryEntry::from(&resolution_detail(case, actor, revision).snapshot)
                })
                .collect(),
            has_more: true,
            next_before_revision: Some(FactRevision::new(2).unwrap()),
        };
        match defect {
            0 => page.revisions[0].case_id = CaseId::new(),
            1 => page.revisions[0].target = FactTarget::Resolution(id(99)),
            2 => {
                page.revisions[0].metadata.receipt.submission_digest =
                    procedural_fact_service_support::digest(99)
            }
            3 => page.revisions.reverse(),
            4 => page.revisions[1] = page.revisions[0].clone(),
            5 => page.revisions.push(FactHistoryEntry::from(
                &resolution_detail(case, actor, 1).snapshot,
            )),
            6 => {
                page.revisions[0] =
                    FactHistoryEntry::from(&resolution_detail(case, actor, 4).snapshot)
            }
            7 => {
                page.revisions.pop();
            }
            8 => page.next_before_revision = None,
            9 => page.next_before_revision = Some(FactRevision::new(99).unwrap()),
            _ => page.has_more = false,
        }
        let mut store = MockReads::new();
        store
            .expect_history()
            .returning(move |_, _, _, _, _| Ok(page.clone()));
        let (identity, _) = case_support::identity(Role::Owner, 1);
        let (service, _) = service(store, identity);
        assert_bad(service.history(
            "session",
            case,
            FactTarget::Resolution(id(10)),
            FactHistoryQuery::new(2, Some(4)).unwrap(),
        ));
    }
}

#[test]
fn closed_historical_capture_remains_readable_without_write_validation() {
    use application::cases::{
        case_administration_digest, CaseAdministrationSnapshot, CurrentCaseAdministration,
    };
    use domain::case_administration::{
        CaseAdministrationValues, CaseAdministrativeStatus, CaseRevision,
    };
    use procedural_fact_service_support::{hasher, snapshot_mut};
    let case = CaseId::new();
    let (identity, actor) = case_support::identity(Role::Paralegal, 2);
    let mut detail = resolution_detail(case, actor.id, 1);
    let values =
        CaseAdministrationValues::basic(domain::cases::CaseMetadata::new("Case", "REF-1").unwrap())
            .with_status(CaseAdministrativeStatus::Closed);
    let snapshot = snapshot_mut(&mut detail);
    snapshot.metadata.recorded_administration =
        CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
            case_id: case,
            revision: CaseRevision::new(1).unwrap(),
            values_digest: case_administration_digest(hasher().as_ref(), &values),
            values,
            changed_at: snapshot.metadata.recorded_at,
            changed_by: snapshot.metadata.recorded_by.clone(),
        }));
    let entry = FactHistoryEntry::from(&detail.snapshot);
    let expected = detail.clone();
    let mut store = MockReads::new();
    store
        .expect_get()
        .returning(move |_, _, _, _, _| Ok(detail.clone()));
    store.expect_history().returning(move |_, _, _, _, _| {
        Ok(FactHistoryPage {
            revisions: vec![entry.clone()],
            has_more: false,
            next_before_revision: None,
        })
    });
    let (service, _) = service(store, identity);
    assert_eq!(
        service
            .get("session", case, FactTarget::Resolution(id(10)), None)
            .unwrap(),
        expected
    );
    assert_eq!(
        service
            .history(
                "session",
                case,
                FactTarget::Resolution(id(10)),
                FactHistoryQuery::new(20, None).unwrap()
            )
            .unwrap()
            .revisions
            .len(),
        1
    );
}

#[test]
fn withdrawn_heads_are_readable_with_explicit_and_all_status_filters() {
    let case = CaseId::new();
    let (identity, _) = case_support::identity(Role::Paralegal, 4);
    let mut resolution = resolution(case, 0);
    resolution.status = FactStatus::Withdrawn;
    resolution.revision = FactRevision::new(2).unwrap();
    let mut notification = notification(case, id(10), 0);
    notification.status = FactStatus::Withdrawn;
    notification.revision = FactRevision::new(2).unwrap();
    let mut store = MockReads::new();
    store
        .expect_list_resolutions()
        .times(2)
        .returning(move |_, _, _, _| {
            Ok(ResolutionPage {
                resolutions: vec![resolution.clone()],
                has_more: false,
                next_after_id: None,
            })
        });
    store
        .expect_list_notifications()
        .times(2)
        .returning(move |_, _, _, _, _| {
            Ok(NotificationPage {
                notifications: vec![notification.clone()],
                has_more: false,
                next_after_id: None,
            })
        });
    let (service, _) = service(store, identity);
    for status in [FactStatusFilter::All, FactStatusFilter::Withdrawn] {
        assert_eq!(
            service
                .list_resolutions(
                    "session",
                    case,
                    ResolutionQuery::new(2, None, status).unwrap()
                )
                .unwrap()
                .resolutions[0]
                .status,
            FactStatus::Withdrawn
        );
        assert_eq!(
            service
                .list_notifications(
                    "session",
                    case,
                    id(10),
                    NotificationQuery::new(2, None, status).unwrap()
                )
                .unwrap()
                .notifications[0]
                .status,
            FactStatus::Withdrawn
        );
    }
}
