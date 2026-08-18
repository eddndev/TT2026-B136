use std::str::FromStr;

use domain::identity::{Permission, Role, UserId};
use uuid::Uuid;

#[test]
fn user_id_round_trips_through_uuid() {
    let raw = Uuid::from_u128(0x00112233_4455_6677_8899_aabbccddeeff);
    let id = UserId::from_uuid(raw);

    assert_eq!(id.as_uuid(), raw);
    assert_eq!(id.to_string(), raw.to_string());
}

#[test]
fn roles_have_stable_storage_names() {
    let cases = [
        (Role::Owner, "owner"),
        (Role::Litigator, "litigator"),
        (Role::Paralegal, "paralegal"),
        (Role::Client, "client"),
    ];

    for (role, name) in cases {
        assert_eq!(role.as_str(), name);
        assert_eq!(Role::from_str(name).unwrap(), role);
    }
    assert!(Role::from_str("administrator").is_err());
}

#[test]
fn role_permissions_follow_the_conservative_matrix() {
    use Permission::*;

    for permission in [
        CreateDocument,
        SealDocument,
        VerifyDocument,
        ExportEvidence,
        VerifyAudit,
        CreateUser,
    ] {
        assert!(Role::Owner.allows(permission));
    }
    for permission in [CreateDocument, SealDocument, VerifyDocument, ExportEvidence] {
        assert!(Role::Litigator.allows(permission));
    }
    assert!(!Role::Litigator.allows(VerifyAudit));
    assert!(!Role::Litigator.allows(CreateUser));

    for permission in [CreateDocument, VerifyDocument, ExportEvidence] {
        assert!(Role::Paralegal.allows(permission));
    }
    assert!(!Role::Paralegal.allows(SealDocument));
    assert!(!Role::Paralegal.allows(VerifyAudit));
    assert!(!Role::Paralegal.allows(CreateUser));

    for permission in [
        CreateDocument,
        SealDocument,
        VerifyDocument,
        ExportEvidence,
        VerifyAudit,
        CreateUser,
    ] {
        assert!(!Role::Client.allows(permission));
    }
}
