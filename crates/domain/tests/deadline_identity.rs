use domain::{
    deadlines::{DeadlineId, DeadlineOperationId, DeadlineRevision, DeadlineStatus},
    identity::{Permission, Role},
};
use uuid::Uuid;

#[test]
fn nil_is_an_explicit_deadline_identity_and_operation() {
    let id = DeadlineId::from_uuid(Uuid::nil());
    let operation = DeadlineOperationId::from_uuid(Uuid::nil());
    assert_eq!(id.as_uuid(), Uuid::nil());
    assert_eq!(operation.as_uuid(), Uuid::nil());
    assert_eq!(
        serde_json::from_str::<DeadlineId>(&serde_json::to_string(&id).unwrap()).unwrap(),
        id
    );
}

#[test]
fn historical_revision_is_positive_and_cannot_wrap() {
    assert!(DeadlineRevision::new(0).is_err());
    assert!(serde_json::from_str::<DeadlineRevision>("0").is_err());
    assert_eq!(DeadlineRevision::initial().next().unwrap().get(), 2);
    assert!(DeadlineRevision::new(u32::MAX).unwrap().next().is_err());
}

#[test]
fn registry_status_does_not_encode_attention_or_clock_based_expiration() {
    assert_eq!(
        "active".parse::<DeadlineStatus>().unwrap(),
        DeadlineStatus::Active
    );
    assert_eq!(
        "retired".parse::<DeadlineStatus>().unwrap(),
        DeadlineStatus::Retired
    );
    for invalid in ["overdue", "attended", "suspended", "", "Active"] {
        assert!(invalid.parse::<DeadlineStatus>().is_err());
    }
}

#[test]
fn deadline_management_requires_owner_or_litigator_and_clients_cannot_read() {
    for (role, read, manage) in [
        (Role::Owner, true, true),
        (Role::Litigator, true, true),
        (Role::Paralegal, true, false),
        (Role::Client, false, false),
    ] {
        assert_eq!(role.allows(Permission::ReadDeadline), read);
        assert_eq!(role.allows(Permission::ManageDeadline), manage);
    }
}
