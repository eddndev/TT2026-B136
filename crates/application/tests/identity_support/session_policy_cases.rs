use application::identity::SessionPolicy;
use application::ApplicationError;

#[test]
fn default_policy_keeps_the_absolute_lifetime_without_idle_expiration() {
    let policy = SessionPolicy::default();
    assert_eq!(policy.absolute_ttl_seconds(), 86_400);
    assert_eq!(policy.idle_ttl_seconds(), None);
}

#[test]
fn explicit_idle_policy_preserves_both_durations() {
    let policy = SessionPolicy::new(300, Some(60)).unwrap();
    assert_eq!(policy.absolute_ttl_seconds(), 300);
    assert_eq!(policy.idle_ttl_seconds(), Some(60));
    assert_eq!(
        SessionPolicy::new(300, None).unwrap().idle_ttl_seconds(),
        None
    );
    assert_eq!(
        SessionPolicy::new(60, Some(60)).unwrap().idle_ttl_seconds(),
        Some(60)
    );
}

#[test]
fn invalid_session_durations_fail_instead_of_disabling_expiration() {
    for (absolute, idle) in [
        (0, None),
        (86_401, None),
        (u64::MAX, None),
        (300, Some(0)),
        (300, Some(301)),
        (300, Some(u64::MAX)),
    ] {
        assert!(matches!(
            SessionPolicy::new(absolute, idle),
            Err(ApplicationError::InvalidConfiguration(_))
        ));
    }
}
