use super::case_report_support::*;
use application::{case_reports::*, ApplicationError};
use domain::identity::Role;
use mockall::Sequence;

#[test]
fn request_preserves_original_replay_time_and_binds_scope_and_principal() {
    for role in [Role::Owner, Role::Litigator] {
        let who = actor(role);
        let input = command();
        let expected = detail(&who, input.clone());
        let saved = expected.clone();
        let expected_actor = who.clone();
        let expected_input = input.clone();
        let mut store = MockStore::new();
        store
            .expect_request()
            .times(1)
            .withf(move |actor, scope, command, digest, at| {
                actor == &expected_actor
                    && *scope == saved.scope
                    && command == &expected_input
                    && *digest == saved.request_digest
                    && *at == now()
            })
            .returning(move |_, _, _, _, _| Ok(expected.clone()));
        let result = service(store, identity(&who, 2))
            .request("session", input)
            .unwrap();
        assert_eq!(result.requested_at, now() - time::Duration::seconds(5));
        assert_eq!(result.requester.principal, who);
    }
}

#[test]
fn denied_roles_cannot_request_list_get_download_or_acknowledge() {
    for role in [Role::Paralegal, Role::Client] {
        let who = actor(role);
        let service = service(MockStore::new(), identity(&who, 5));
        let id = CaseReportId::new();
        let query = CaseReportQuery {
            limit: 5,
            after_id: None,
            unread_only: false,
        };
        for result in [
            service.request("session", command()).map(|_| ()),
            service.list("session", query).map(|_| ()),
            service.get("session", id).map(|_| ()),
            service
                .download("session", id, CaseReportFormat::Pdf)
                .map(|_| ()),
            service.acknowledge_notice("session", id).map(|_| ()),
        ] {
            assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
        }
    }
}

#[test]
fn invalid_filters_and_pagination_never_reach_store() {
    let who = actor(Role::Owner);
    let mut bad = Vec::new();
    let mut empty = command();
    empty.filters.period_before = empty.filters.period_from;
    bad.push(empty);
    let mut long = command();
    long.filters.period_from = now() - time::Duration::days(367);
    bad.push(long);
    let mut offset = command();
    offset.filters.period_from = offset
        .filters
        .period_from
        .to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap());
    bad.push(offset);
    let service = service(MockStore::new(), identity(&who, 5));
    for input in bad {
        assert!(matches!(
            service.request("session", input),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
    for limit in [0, 101] {
        let query = CaseReportQuery {
            limit,
            after_id: None,
            unread_only: false,
        };
        assert!(matches!(
            service.list("session", query),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
}

#[test]
fn request_reauthenticates_full_principal_and_propagates_revocation() {
    let who = actor(Role::Owner);
    for mutation in 0..4 {
        let response = detail(&who, command());
        let input = response.command.clone();
        let mut store = MockStore::new();
        store
            .expect_request()
            .times(1)
            .returning(move |_, _, _, _, _| Ok(response.clone()));
        let mut auth = super::case_support::MockIdentity::new();
        let first = who.clone();
        let mut sequence = Sequence::new();
        auth.expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| Ok(first));
        let mut changed = who.clone();
        match mutation {
            0 => changed.id = domain::identity::UserId::new(),
            1 => changed.email = "changed@example.test".into(),
            2 => changed.role = Role::Litigator,
            _ => (),
        }
        auth.expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| {
                if mutation == 3 {
                    Err(ApplicationError::InvalidSession)
                } else {
                    Ok(changed)
                }
            });
        assert!(matches!(
            service(store, auth).request("session", input),
            Err(ApplicationError::InvalidSession)
        ));
    }
}

#[test]
fn forged_request_receipt_identity_scope_and_digest_are_rejected() {
    let who = actor(Role::Owner);
    for mutation in 0..4 {
        let mut response = detail(&who, command());
        let input = response.command.clone();
        match mutation {
            0 => response.requester.principal.email = "other@example.test".into(),
            1 => response.scope = CaseReportScope::AssignedCases,
            2 => response.request_digest = domain::crypto::Sha256Digest::from_array([88; 32]),
            _ => response.requester.auth_generation = i64::MAX as u64 + 1,
        }
        let mut store = MockStore::new();
        store
            .expect_request()
            .times(1)
            .returning(move |_, _, _, _, _| Ok(response.clone()));
        inconsistent(service(store, identity(&who, 1)).request("session", input));
    }
}
