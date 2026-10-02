use application::identity::{
    LoginChallengeIdentity, Principal, SessionIdentity, SessionPolicy, SessionStore,
};
use application::ApplicationError;
use domain::crypto::DocumentHasher;
use domain::identity::{Role, UserId};
use infrastructure::{RedisSessionStore, RingSha256Hasher};
use redis::Commands;

fn key(namespace: &str, token: &str) -> String {
    format!(
        "identity:{namespace}:{}",
        RingSha256Hasher.hash_bytes(token.as_bytes()).to_hex()
    )
}

#[test]
fn legacy_or_invalid_authentication_values_never_become_generation_zero() {
    let Ok(url) = std::env::var("IDENTITY_TEST_REDIS_URL") else {
        return;
    };
    let store = RedisSessionStore::connect(&url).unwrap();
    let mut redis = redis::Client::open(url).unwrap().get_connection().unwrap();
    let principal = Principal {
        id: UserId::new(),
        email: "owner@example.test".into(),
        role: Role::Owner,
    };
    let token = uuid::Uuid::new_v4().to_string();
    for value in [
        serde_json::to_string(&principal).unwrap(),
        "{}".into(),
        "invalid-json".into(),
        serde_json::json!({"principal":{"id":principal.id,"email":principal.email,"role":"owner","unexpected":true},"auth_generation":0}).to_string(),
        serde_json::json!({"principal":principal,"auth_generation":u64::MAX}).to_string(),
        serde_json::json!({"principal":principal,"auth_generation":0,"unexpected":true})
            .to_string(),
    ] {
        redis
            .set_ex::<_, _, ()>(key("session", &token), value, 60)
            .unwrap();
        assert!(matches!(store.find_session(&token, SessionPolicy::default()), Ok(None)));
    }
    for value in [
        principal.id.to_string(),
        "{}".into(),
        serde_json::json!({"user_id":principal.id,"auth_generation":u64::MAX}).to_string(),
    ] {
        redis
            .set_ex::<_, _, ()>(key("challenge", &token), value, 60)
            .unwrap();
        assert!(matches!(store.take_challenge(&token), Ok(None)));
        assert!(!redis.exists::<_, bool>(key("challenge", &token)).unwrap());
    }
}

#[test]
fn redis_roundtrips_generations_and_rejects_out_of_range_values_on_creation() {
    let Ok(url) = std::env::var("IDENTITY_TEST_REDIS_URL") else {
        return;
    };
    let store = RedisSessionStore::connect(&url).unwrap();
    let principal = Principal {
        id: UserId::new(),
        email: "owner@example.test".into(),
        role: Role::Owner,
    };
    let challenge = LoginChallengeIdentity {
        user_id: principal.id,
        auth_generation: 7,
    };
    let token = store.create_challenge(&challenge, 60).unwrap();
    assert!(store.take_challenge(&token).unwrap() == Some(challenge));
    assert!(store.take_challenge(&token).unwrap().is_none());
    let session = SessionIdentity {
        principal,
        auth_generation: 7,
    };
    let policy = SessionPolicy::new(60, None).unwrap();
    let token = store.create_session(&session, policy).unwrap().access_token;
    assert!(
        store
            .find_session(&token, policy)
            .unwrap()
            .map(|state| state.identity)
            == Some(session.clone())
    );
    store.revoke_session(&token).unwrap();
    assert!(store.find_session(&token, policy).unwrap().is_none());
    let invalid = SessionIdentity {
        auth_generation: u64::MAX,
        ..session
    };
    assert!(matches!(
        store.create_session(&invalid, policy),
        Err(ApplicationError::InvalidInput(_))
    ));
    let invalid = LoginChallengeIdentity {
        user_id: invalid.principal.id,
        auth_generation: u64::MAX,
    };
    assert!(matches!(
        store.create_challenge(&invalid, 60),
        Err(ApplicationError::InvalidInput(_))
    ));
}
