#[allow(dead_code)]
mod case_support;
mod deadline_profile_catalog_support;
use application::{deadline_profiles::*, ApplicationError};
use deadline_profile_catalog_support::*;
use domain::{cases::CaseId, identity::Role};
const GLOBAL: DeadlineProfileCollection = DeadlineProfileCollection::Global;

#[test]
fn all_staff_read_global_profiles_and_private_profiles_are_not_filtered_after_loading() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        let (identity, actor) = case_support::identity(role, 2);
        let row = DeadlineProfileOverview::from(&detail(actor.id, &command(), definition(None)));
        let expected = row.clone();
        let mut store = MockStore::new();
        store
            .expect_list()
            .times(1)
            .return_once(move |id, collection, query, _| {
                assert_eq!(id, actor.id);
                assert_eq!(collection, GLOBAL);
                assert_eq!(query.limit(), 20);
                Ok(DeadlineProfilePage {
                    profiles: vec![row],
                    has_more: false,
                    next_after_id: None,
                })
            });
        let (service, _) = deadline_profile_catalog_support::service(store, identity);
        assert_eq!(
            service.list("session", GLOBAL, query(20)).unwrap().profiles,
            vec![expected]
        );
    }
    let (identity, actor) = case_support::identity(Role::Owner, 1);
    let row = DeadlineProfileOverview::from(&detail(
        actor.id,
        &command(),
        definition(Some(CaseId::new())),
    ));
    let mut store = MockStore::new();
    store.expect_list().times(1).return_once(move |_, _, _, _| {
        Ok(DeadlineProfilePage {
            profiles: vec![row],
            has_more: false,
            next_after_id: None,
        })
    });
    let (service, _) = deadline_profile_catalog_support::service(store, identity);
    assert!(matches!(
        service.list("session", GLOBAL, query(20)),
        Err(ApplicationError::DeadlineProfile(
            DeadlineProfileError::StoredInconsistent(_)
        ))
    ));
}

#[test]
fn case_collection_always_reaches_case_authorization_even_for_global_only_results() {
    let case = CaseId::new();
    let collection = DeadlineProfileCollection::ForCase(case);
    let (identity, actor) = case_support::identity(Role::Litigator, 1);
    let mut store = MockStore::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |id, selected, _, _| {
            assert_eq!(id, actor.id);
            assert_eq!(selected, collection);
            Err(ApplicationError::CaseNotFound)
        });
    let (service, _) = deadline_profile_catalog_support::service(store, identity);
    assert!(matches!(
        service.list("session", collection, query(20)),
        Err(ApplicationError::CaseNotFound)
    ));

    let (identity, actor) = case_support::identity(Role::Paralegal, 2);
    let row = detail(actor.id, &command(), definition(None));
    let expected = row.clone();
    let mut store = MockStore::new();
    store
        .expect_get()
        .times(1)
        .return_once(move |id, selected, target, revision, _| {
            assert_eq!(id, actor.id);
            assert_eq!(selected, collection);
            assert_eq!(target, row.id);
            assert_eq!(revision, Some(row.revision));
            Ok(row)
        });
    let (service, _) = deadline_profile_catalog_support::service(store, identity);
    assert_eq!(
        service
            .get("session", collection, expected.id, Some(expected.revision))
            .unwrap(),
        expected
    );
}

#[test]
fn exact_reads_reject_other_root_revision_case_or_a_damaged_receipt() {
    for mutation in 0..4 {
        let (identity, actor) = case_support::identity(Role::Paralegal, 1);
        let case = CaseId::new();
        let command = command();
        let mut row = detail(actor.id, &command, definition(Some(case)));
        let id = row.id;
        let revision = row.revision;
        match mutation {
            0 => row.id = DeadlineProfileId::new(),
            1 => row.revision = DeadlineProfileRevision::new(2).unwrap(),
            2 => row = detail(actor.id, &command, definition(Some(CaseId::new()))),
            _ => row.receipt.submission_digest = domain::crypto::Sha256Digest::from_array([0; 32]),
        }
        let mut store = MockStore::new();
        store
            .expect_get()
            .times(1)
            .return_once(move |_, _, _, _, _| Ok(row));
        let (service, _) = deadline_profile_catalog_support::service(store, identity);
        assert!(matches!(
            service.get(
                "session",
                DeadlineProfileCollection::ForCase(case),
                id,
                Some(revision)
            ),
            Err(ApplicationError::DeadlineProfile(
                DeadlineProfileError::StoredInconsistent(_)
            ))
        ));
    }
}

#[test]
fn query_limits_and_page_cursor_invariants_are_enforced() {
    for limit in [0, 101] {
        assert!(DeadlineProfileQuery::new(limit, None, DeadlineProfileStatusFilter::All).is_err());
    }
    for limit in [0, 21] {
        assert!(DeadlineProfileHistoryQuery::new(limit, None).is_err());
    }
    assert!(DeadlineProfileHistoryQuery::new(10, Some(0)).is_err());
    for mutation in 0..3 {
        let (identity, actor) = case_support::identity(Role::Owner, 1);
        let row = DeadlineProfileOverview::from(&detail(actor.id, &command(), definition(None)));
        let page = match mutation {
            0 => DeadlineProfilePage {
                profiles: vec![row.clone()],
                has_more: true,
                next_after_id: Some(row.id),
            },
            1 => DeadlineProfilePage {
                profiles: vec![row.clone(), row],
                has_more: false,
                next_after_id: None,
            },
            _ => DeadlineProfilePage {
                profiles: vec![row.clone()],
                has_more: false,
                next_after_id: Some(row.id),
            },
        };
        let mut store = MockStore::new();
        store
            .expect_list()
            .times(1)
            .return_once(move |_, _, _, _| Ok(page));
        let (service, _) = deadline_profile_catalog_support::service(store, identity);
        assert!(matches!(
            service.list("session", GLOBAL, query(20)),
            Err(ApplicationError::DeadlineProfile(
                DeadlineProfileError::StoredInconsistent(_)
            ))
        ));
    }
}

#[test]
fn history_is_lightweight_contiguous_and_binds_scope_and_receipts() {
    for broken in [false, true] {
        let (identity, actor) = case_support::identity(Role::Owner, if broken { 1 } else { 2 });
        let first = detail(actor.id, &command(), definition(None));
        let retired = detail(actor.id, &retirement(&first), first.definition.clone());
        let id = first.id;
        let mut entries = vec![
            DeadlineProfileHistoryEntry::from(&retired),
            DeadlineProfileHistoryEntry::from(&first),
        ];
        if broken {
            entries[0].definition_digest = domain::crypto::Sha256Digest::from_array([0; 32]);
        }
        let expected = entries.clone();
        let mut store = MockStore::new();
        store.expect_history().times(1).return_once(
            move |actor_id, collection, profile, query, _| {
                assert_eq!(actor_id, actor.id);
                assert_eq!(collection, GLOBAL);
                assert_eq!(profile, id);
                assert_eq!(query.limit(), 10);
                Ok(DeadlineProfileHistoryPage {
                    revisions: entries,
                    has_more: false,
                    next_before_revision: None,
                })
            },
        );
        let (service, _) = deadline_profile_catalog_support::service(store, identity);
        let result = service.history(
            "session",
            GLOBAL,
            id,
            DeadlineProfileHistoryQuery::new(10, None).unwrap(),
        );
        if broken {
            assert!(matches!(
                result,
                Err(ApplicationError::DeadlineProfile(
                    DeadlineProfileError::StoredInconsistent(_)
                ))
            ));
        } else {
            assert_eq!(result.unwrap().revisions, expected);
        }
    }
}
