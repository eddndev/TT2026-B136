use domain::cases::{can_create_case, can_manage_members, can_read_case, CaseId, CaseMetadata};
use domain::identity::{Permission, Role};
use domain::DomainError;
use uuid::Uuid;

#[test]
fn case_id_round_trips_through_uuid_and_json() {
    let raw = Uuid::from_u128(0x00112233_4455_6677_8899_aabbccddeeff);
    let id = CaseId::from_uuid(raw);
    let encoded = serde_json::to_string(&id).unwrap();

    assert_eq!(id.as_uuid(), raw);
    assert_eq!(id.to_string(), "00112233-4455-6677-8899-aabbccddeeff");
    assert_eq!(encoded, "\"00112233-4455-6677-8899-aabbccddeeff\"");
    assert_eq!(serde_json::from_str::<CaseId>(&encoded).unwrap(), id);
    assert_ne!(CaseId::new(), CaseId::default());
}

#[test]
fn case_metadata_trims_and_retains_human_text() {
    let metadata = CaseMetadata::new("  Defensa de Jos\u{e9}  ", "  NUC-2026/136  ").unwrap();

    assert_eq!(metadata.title(), "Defensa de Jos\u{e9}");
    assert_eq!(metadata.reference(), "NUC-2026/136");
}

#[test]
fn case_metadata_requires_both_fields() {
    for title in ["", "   ", "\u{2003}"] {
        assert!(matches!(
            CaseMetadata::new(title, "NUC-1"),
            Err(DomainError::InvalidCaseMetadata { field: "title", .. })
        ));
    }
    for reference in ["", "   ", "\u{2003}"] {
        assert!(matches!(
            CaseMetadata::new("Defense", reference),
            Err(DomainError::InvalidCaseMetadata {
                field: "reference",
                ..
            })
        ));
    }
}

#[test]
fn case_metadata_counts_characters_after_trimming() {
    let title = "\u{e9}".repeat(200);
    let reference = "\u{e9}".repeat(100);

    assert!(CaseMetadata::new(&format!(" {title} "), &format!(" {reference} ")).is_ok());
    assert!(CaseMetadata::new(&format!("{title}a"), &reference).is_err());
    assert!(CaseMetadata::new(&title, &format!("{reference}a")).is_err());
}

#[test]
fn case_metadata_rejects_controls_even_around_text() {
    for value in [
        "\ntext",
        "text\r",
        "te\0xt",
        "te\txt",
        "text\u{7f}",
        "text\u{85}",
    ] {
        assert!(CaseMetadata::new(value, "NUC-1").is_err());
        assert!(CaseMetadata::new("Defense", value).is_err());
    }
}

#[test]
fn case_policy_covers_every_role_and_membership_state() {
    for (role, may_create, may_manage, may_read_unassigned) in [
        (Role::Owner, true, true, true),
        (Role::Litigator, true, false, false),
        (Role::Paralegal, false, false, false),
        (Role::Client, false, false, false),
    ] {
        assert_eq!(can_create_case(role), may_create);
        assert_eq!(can_manage_members(role), may_manage);
        assert_eq!(can_read_case(role, false), may_read_unassigned);
        assert!(can_read_case(role, true));
    }
}

#[test]
fn client_case_visibility_does_not_grant_document_permissions() {
    assert!(can_read_case(Role::Client, true));
    for permission in [
        Permission::CreateDocument,
        Permission::SealDocument,
        Permission::VerifyDocument,
        Permission::ExportEvidence,
        Permission::VerifyAudit,
        Permission::CreateUser,
    ] {
        assert!(!Role::Client.allows(permission));
    }
}
