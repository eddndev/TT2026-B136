use std::env;

use application::identity::{Principal, SessionStore, UserRecord, UserRepository};
use application::ApplicationError;
use domain::crypto::{RecoveryCodeSet, RECOVERY_CODE_COUNT};
use domain::identity::{Role, UserId};
use infrastructure::{PostgresUserRepository, RedisSessionStore};

fn user(email: &str, role: Role) -> UserRecord {
    UserRecord {
        id: UserId::new(),
        email: email.to_string(),
        password_hash: "$argon2id$test".to_string(),
        role,
        active: true,
        protected_totp_secret: vec![7; 48],
        recovery_codes: RecoveryCodeSet::from_hashes(
            (0..RECOVERY_CODE_COUNT)
                .map(|index| format!("$argon2id$recovery-{index}"))
                .collect(),
        )
        .unwrap(),
        revision: 0,
    }
}

#[test]
fn postgres_persists_users_and_closes_bootstrap_atomically() {
    let Ok(database_url) = env::var("IDENTITY_TEST_DATABASE_URL") else {
        return;
    };
    let repository = PostgresUserRepository::connect(&database_url).unwrap();
    let owner = user("owner@example.com", Role::Owner);

    assert!(!repository.has_users().unwrap());
    assert!(repository.insert_initial_owner(owner.clone()).unwrap());
    assert!(repository.has_users().unwrap());
    assert!(!repository
        .insert_initial_owner(user("other@example.com", Role::Owner))
        .unwrap());
    let stored = repository
        .find_by_email("owner@example.com")
        .unwrap()
        .unwrap();
    assert_eq!(stored.id, owner.id);
    assert_eq!(stored.role, Role::Owner);
    assert_eq!(stored.recovery_codes.remaining(), RECOVERY_CODE_COUNT);
    assert_eq!(
        repository.find_by_id(owner.id).unwrap().unwrap().email,
        stored.email
    );

    let paralegal = user("helper@example.com", Role::Paralegal);
    repository.insert(paralegal.clone()).unwrap();
    assert_eq!(
        repository.find_by_id(paralegal.id).unwrap().unwrap().email,
        paralegal.email
    );
    assert!(matches!(
        repository.insert(paralegal.clone()),
        Err(ApplicationError::UserAlreadyExists)
    ));

    let replacement = RecoveryCodeSet::from_hashes(
        (0..RECOVERY_CODE_COUNT)
            .map(|index| format!("$argon2id$replacement-{index}"))
            .collect(),
    )
    .unwrap();
    repository
        .replace_recovery_codes(paralegal.id, 0, replacement)
        .unwrap();
    assert_eq!(
        repository
            .find_by_id(paralegal.id)
            .unwrap()
            .unwrap()
            .revision,
        1
    );
    assert!(matches!(
        repository.replace_recovery_codes(paralegal.id, 0, paralegal.recovery_codes),
        Err(ApplicationError::ConcurrentModification)
    ));
}

#[test]
fn redis_sessions_are_opaque_replay_safe_and_revocable() {
    let Ok(redis_url) = env::var("IDENTITY_TEST_REDIS_URL") else {
        return;
    };
    let store = RedisSessionStore::connect(&redis_url).unwrap();
    let principal = Principal {
        id: UserId::new(),
        email: "owner@example.com".to_string(),
        role: Role::Owner,
    };

    let challenge = store.create_challenge(principal.id, 60).unwrap();
    assert_ne!(challenge, principal.id.to_string());
    assert_eq!(
        store.resolve_challenge(&challenge).unwrap(),
        Some(principal.id)
    );
    store.consume_challenge(&challenge).unwrap();
    assert_eq!(store.resolve_challenge(&challenge).unwrap(), None);

    let token = store.create_session(&principal, 60).unwrap();
    assert_eq!(store.find_session(&token).unwrap(), Some(principal.clone()));
    store.revoke_session(&token).unwrap();
    assert_eq!(store.find_session(&token).unwrap(), None);

    assert_eq!(store.failed_password_attempts(&principal.email).unwrap(), 0);
    assert_eq!(
        store.record_password_failure(&principal.email, 60).unwrap(),
        1
    );
    assert_eq!(
        store.record_password_failure(&principal.email, 60).unwrap(),
        2
    );
    assert_eq!(store.failed_password_attempts(&principal.email).unwrap(), 2);
    store.clear_password_failures(&principal.email).unwrap();
    assert_eq!(store.failed_password_attempts(&principal.email).unwrap(), 0);

    assert!(store.claim_totp(principal.id, "123456", 60).unwrap());
    assert!(!store.claim_totp(principal.id, "123456", 60).unwrap());
}
