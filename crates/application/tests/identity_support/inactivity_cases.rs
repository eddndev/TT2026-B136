use super::{ports, MemorySessions, MemoryUsers};
use application::identity::{IdentityService, SessionPolicy, SessionResult, SessionStore};
use application::ApplicationError;
use domain::identity::{Permission, Role};
use std::sync::{atomic::Ordering, Arc};

fn logged_in(
    policy: SessionPolicy,
) -> (
    IdentityService,
    Arc<MemoryUsers>,
    Arc<MemorySessions>,
    SessionResult,
) {
    let users = Arc::new(MemoryUsers::default());
    let sessions = Arc::new(MemorySessions::default());
    sessions.set_now(100_000);
    let service =
        IdentityService::with_session_policy(ports(users.clone(), sessions.clone()), policy);
    let owner = service
        .bootstrap_owner("owner@example.com", "correct horse battery")
        .unwrap();
    let challenge = service
        .start_login("owner@example.com", "correct horse battery")
        .unwrap();
    let result = service
        .complete_recovery(&challenge.challenge_token, &owner.recovery_codes[0])
        .unwrap();
    (service, users, sessions, result)
}

#[test]
fn session_reads_and_authorization_never_record_activity() {
    let policy = SessionPolicy::new(300, Some(60)).unwrap();
    let (service, _, sessions, issued) = logged_in(policy);
    sessions.set_now(120_000);
    assert_eq!(
        service.authenticate(&issued.access_token).unwrap(),
        issued.principal
    );
    service
        .authorize(&issued.access_token, Permission::CreateUser)
        .unwrap();
    let status = service.session_status(&issued.access_token).unwrap();
    assert_eq!(status.server_now_unix_ms, 120_000);
    assert_eq!(status.absolute_expires_at_unix_ms, 400_000);
    assert_eq!(status.idle_expires_at_unix_ms, Some(160_000));
    assert_eq!(status.policy, policy);
    assert_eq!(sessions.activity_calls.load(Ordering::SeqCst), 0);
    sessions.set_now(160_000);
    assert!(matches!(
        service.authenticate(&issued.access_token),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn explicit_activity_extends_idle_but_never_the_absolute_deadline() {
    let policy = SessionPolicy::new(300, Some(120)).unwrap();
    let (service, _, sessions, issued) = logged_in(policy);
    for now in [200_000, 300_000, 399_999] {
        sessions.set_now(now);
        let status = service.record_activity(&issued.access_token).unwrap();
        assert_eq!(status.absolute_expires_at_unix_ms, 400_000);
        assert_eq!(
            status.idle_expires_at_unix_ms,
            Some((now + 120_000).min(400_000))
        );
    }
    sessions.set_now(400_000);
    assert!(matches!(
        service.record_activity(&issued.access_token),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(sessions.activity_calls.load(Ordering::SeqCst), 3);
}

#[test]
fn default_sessions_keep_the_absolute_deadline_after_explicit_activity() {
    let (service, _, sessions, issued) = logged_in(SessionPolicy::default());
    sessions.set_now(160_000);
    let status = service.record_activity(&issued.access_token).unwrap();
    assert_eq!(status.absolute_expires_at_unix_ms, 86_500_000);
    assert_eq!(status.idle_expires_at_unix_ms, None);
}

#[test]
fn expired_or_revoked_sessions_during_user_lookup_are_not_admitted() {
    for revoke in [false, true] {
        let (service, users, sessions, issued) =
            logged_in(SessionPolicy::new(300, Some(60)).unwrap());
        let token = issued.access_token.clone();
        let store = sessions.clone();
        users.after_next_find(move || {
            if revoke {
                store.revoke_session(&token).unwrap();
            } else {
                store.set_now(160_000);
            }
        });
        assert!(matches!(
            service.authenticate(&issued.access_token),
            Err(ApplicationError::InvalidSession)
        ));
        assert_eq!(sessions.activity_calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn changed_accounts_are_rejected_before_activity_is_written() {
    for change in 0..5 {
        let (service, users, sessions, issued) =
            logged_in(SessionPolicy::new(300, Some(60)).unwrap());
        match change {
            0 => users.deactivate(issued.principal.id),
            1 => users.set_role(issued.principal.id, Role::Paralegal),
            2 => {
                users.records.lock().unwrap().remove(&issued.principal.id);
            }
            3 => {
                users
                    .records
                    .lock()
                    .unwrap()
                    .get_mut(&issued.principal.id)
                    .unwrap()
                    .email = "changed@example.com".into();
            }
            _ => {
                users
                    .records
                    .lock()
                    .unwrap()
                    .get_mut(&issued.principal.id)
                    .unwrap()
                    .auth_generation += 1;
            }
        }
        assert!(matches!(
            service.record_activity(&issued.access_token),
            Err(ApplicationError::InvalidSession)
        ));
        assert_eq!(sessions.activity_calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn activity_store_failure_is_propagated_without_returning_new_deadlines() {
    let (service, _, sessions, issued) = logged_in(SessionPolicy::new(300, Some(60)).unwrap());
    sessions.fail_activity.store(true, Ordering::SeqCst);
    assert!(matches!(
        service.record_activity(&issued.access_token),
        Err(ApplicationError::Port(_))
    ));
}

#[test]
fn both_mfa_paths_return_the_store_deadlines_and_effective_lifetime() {
    let policy = SessionPolicy::new(300, Some(60)).unwrap();
    let (service, _, _, issued) = logged_in(policy);
    let challenge = service
        .start_login("owner@example.com", "correct horse battery")
        .unwrap();
    let totp = service
        .complete_totp(&challenge.challenge_token, "123456")
        .unwrap();
    for result in [issued, totp] {
        assert_eq!(result.session.principal, result.principal);
        assert_eq!(result.session.policy, policy);
        assert_eq!(result.session.server_now_unix_ms, 100_000);
        assert_eq!(result.session.absolute_expires_at_unix_ms, 400_000);
        assert_eq!(result.session.idle_expires_at_unix_ms, Some(160_000));
        assert_eq!(result.expires_in_seconds, 60);
    }
}

#[test]
fn changing_policy_requires_a_fresh_authentication() {
    let (_, users, sessions, issued) = logged_in(SessionPolicy::default());
    let service = IdentityService::with_session_policy(
        ports(users, sessions),
        SessionPolicy::new(86_400, Some(60)).unwrap(),
    );
    assert!(matches!(
        service.authenticate(&issued.access_token),
        Err(ApplicationError::InvalidSession)
    ));
}
