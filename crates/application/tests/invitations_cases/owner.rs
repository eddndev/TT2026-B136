use super::support::*;

#[test]
fn owner_issues_only_encrypted_capability_with_normalized_email_and_24_hour_expiry() {
    for role in [Role::Litigator, Role::Paralegal] {
        let mut harness = Harness::new();
        let state = harness.state.clone();
        let observed = state.clone();
        harness
            .store
            .expect_issue()
            .returning(move |actual_actor, command| {
                assert_eq!(actual_actor, actor());
                assert_eq!(command.request.email, EMAIL);
                assert_eq!(command.request.role, role);
                assert_eq!(command.request.operation_id, operation(10));
                assert_eq!(command.at, now());
                let mut state = observed.lock().unwrap();
                state.calls.push("issue");
                assert_eq!(command.token_digest, token_digest(&state));
                let sealed = &state.sealed[0];
                assert_eq!(
                    sealed.context,
                    InvitationProtectionContext {
                        purpose: InvitationPurpose::Delivery,
                        id: id(),
                        revision: 1,
                    }
                );
                assert_eq!(sealed.plaintext, RAW_TOKEN);
                assert_eq!(command.protected_token, sealed.opaque);
                assert_ne!(command.protected_token, RAW_TOKEN);
                let mut result = summary();
                result.role = role;
                Ok(result)
            });
        let service = harness.service();
        let mut request = issue_request();
        request.email = " RECIPIENT@EXAMPLE.TEST ".into();
        request.role = role;
        let result = service.issue("session", request).unwrap();
        assert_eq!(result.role, role);
        assert_eq!(
            result.expires_at - result.issued_at,
            time::Duration::hours(24)
        );
        let state = state.lock().unwrap();
        assert_eq!(state.tokens, 1);
        assert_eq!(state.enrollments, 0);
        assert_eq!(state.recoveries, 0);
        assert!(state.user.is_none());
        assert_eq!(state.pending_writes, 0);
        assert_eq!(
            state.calls.iter().filter(|call| **call == "issue").count(),
            1
        );
        assert!(
            state
                .calls
                .iter()
                .filter(|call| **call == "authenticate")
                .count()
                >= 2
        );
    }
}

#[test]
fn non_owners_cannot_issue_resend_list_or_read_invitations() {
    for role in [Role::Litigator, Role::Paralegal, Role::Client] {
        let harness = Harness::new();
        let state = harness.state.clone();
        state.lock().unwrap().principal.as_mut().unwrap().role = role;
        let service = harness.service();
        assert!(matches!(
            service.issue("session", issue_request()),
            Err(ApplicationError::PermissionDenied)
        ));
        assert!(matches!(
            service.resend("session", id(), resend_request()),
            Err(ApplicationError::PermissionDenied)
        ));
        assert!(matches!(
            service.list("session", query()),
            Err(ApplicationError::PermissionDenied)
        ));
        assert!(matches!(
            service.get("session", id()),
            Err(ApplicationError::PermissionDenied)
        ));
        let state = state.lock().unwrap();
        assert_eq!(state.tokens, 0);
        assert!(state.sealed.is_empty());
        assert_eq!(state.calls, vec!["authenticate"; 4]);
    }
}

#[test]
fn invitation_cannot_create_an_owner_or_client() {
    for role in [Role::Owner, Role::Client] {
        let harness = Harness::new();
        let state = harness.state.clone();
        let service = harness.service();
        let mut request = issue_request();
        request.role = role;
        assert!(matches!(
            service.issue("session", request),
            Err(ApplicationError::InvalidInput(_))
        ));
        assert_eq!(state.lock().unwrap().tokens, 0);
    }
}

#[test]
fn owner_is_reauthenticated_after_secret_preparation_before_committing() {
    let harness = Harness::new();
    let state = harness.state.clone();
    state.lock().unwrap().change_owner_on_protect = true;
    let service = harness.service();
    assert!(matches!(
        service.issue("session", issue_request()),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(state.lock().unwrap().tokens, 1);
}

#[test]
fn existing_active_or_inactive_accounts_are_never_reactivated_or_reenrolled() {
    for active in [true, false] {
        let mut harness = Harness::new();
        let state = harness.state.clone();
        let mut existing = owner_record();
        existing.id = UserId::from_uuid(operation(2));
        existing.email = EMAIL.into();
        existing.active = active;
        existing.role = Role::Paralegal;
        state.lock().unwrap().user = Some(existing.clone());
        harness
            .store
            .expect_issue()
            .returning(|_, _| Err(ApplicationError::UserAlreadyExists));
        let service = harness.service();
        assert!(matches!(
            service.issue("session", issue_request()),
            Err(ApplicationError::UserAlreadyExists)
        ));
        let state = state.lock().unwrap();
        let saved = state.user.as_ref().unwrap();
        assert_eq!(saved.id, existing.id);
        assert_eq!(saved.active, active);
        assert_eq!(saved.password_hash, existing.password_hash);
        assert_eq!(saved.role, existing.role);
        assert_eq!(state.enrollments, 0);
        assert_eq!(state.pending_writes, 0);
    }
}

#[test]
fn explicit_resend_prepares_fresh_revision_and_honors_old_link_invalidation() {
    let mut harness = Harness::new();
    let state = harness.state.clone();
    state.lock().unwrap().token_value = [0x6b; 32];
    harness
        .store
        .expect_get()
        .returning(|actual_actor, invitation| {
            assert_eq!(actual_actor, actor());
            assert_eq!(invitation, id());
            Ok(summary())
        });
    let observed = state.clone();
    harness
        .store
        .expect_resend()
        .returning(move |actual_actor, invitation, command| {
            assert_eq!(actual_actor, actor());
            assert_eq!(invitation, id());
            assert_eq!(command.request, resend_request());
            let mut state = observed.lock().unwrap();
            state.calls.push("resend");
            let sealed = &state.sealed[0];
            assert_eq!(
                sealed.context,
                InvitationProtectionContext {
                    purpose: InvitationPurpose::Delivery,
                    id: id(),
                    revision: 2,
                }
            );
            assert_eq!(sealed.plaintext, [0x6b; 32]);
            assert_eq!(command.protected_token, sealed.opaque);
            state.candidate = None;
            let mut result = summary();
            result.revision = 2;
            result.operation_id = operation(11);
            Ok(result)
        });
    let service = harness.service();
    assert_eq!(
        service
            .resend("session", id(), resend_request())
            .unwrap()
            .revision,
        2
    );
    assert!(matches!(
        service.prepare(&RAW_TOKEN, operation(20), PASSWORD),
        Err(ApplicationError::InvalidCredentials)
    ));
    assert!(matches!(
        service.finish(&RAW_TOKEN, operation(30), PASSWORD, CODE),
        Err(ApplicationError::InvalidCredentials)
    ));
    let state = state.lock().unwrap();
    assert_eq!(state.enrollments, 0);
    assert_eq!(state.pending_writes, 0);
    assert!(state.finish_commands.is_empty());
}

#[test]
fn commit_authorization_or_intent_conflict_is_returned_without_automatic_retry() {
    let mut harness = Harness::new();
    let state = harness.state.clone();
    let observed = state.clone();
    harness
        .store
        .expect_issue()
        .returning(move |actual_actor, _| {
            assert_eq!(actual_actor, actor());
            observed.lock().unwrap().calls.push("issue");
            Err(ApplicationError::ConcurrentModification)
        });
    let service = harness.service();
    assert!(matches!(
        service.issue("session", issue_request()),
        Err(ApplicationError::ConcurrentModification)
    ));
    assert_eq!(
        state
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|call| **call == "issue")
            .count(),
        1
    );
}

#[test]
fn owner_pages_return_only_public_metadata_after_current_authorization() {
    let mut harness = Harness::new();
    harness
        .store
        .expect_list()
        .returning(|actual_actor, actual_query| {
            assert_eq!(actual_actor, actor());
            assert_eq!(actual_query, query());
            Ok(InvitationPage {
                items: vec![summary()],
                next: None,
            })
        });
    harness
        .store
        .expect_get()
        .returning(|actual_actor, invitation| {
            assert_eq!(actual_actor, actor());
            assert_eq!(invitation, id());
            Ok(summary())
        });
    let state = harness.state.clone();
    let service = harness.service();
    assert_eq!(
        service.list("session", query()).unwrap().items,
        vec![summary()]
    );
    assert_eq!(service.get("session", id()).unwrap(), summary());
    let state = state.lock().unwrap();
    assert!(state.sealed.is_empty());
    assert_eq!(state.tokens, 0);
}
