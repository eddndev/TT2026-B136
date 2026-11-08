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

#[test]
fn document_metadata_reads_are_available_only_to_internal_roles() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        assert!(role.allows(Permission::ReadDocument));
    }
    assert!(!Role::Client.allows(Permission::ReadDocument));
}

#[test]
fn appending_versions_keeps_the_internal_document_role_boundary() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        assert!(role.allows(Permission::AppendDocument));
    }
    assert!(!Role::Client.allows(Permission::AppendDocument));
}

mod display_names {
    use domain::identity::display_name::DISPLAY_NAME_MAX_CHARS;
    use domain::identity::DisplayName;
    use domain::DomainError;

    #[test]
    fn display_names_are_trimmed_and_keep_interior_spaces() {
        let name = DisplayName::new("  Ana  Garc\u{ed}a ").unwrap();
        assert_eq!(name.as_str(), "Ana  Garc\u{ed}a");
        assert_eq!(name.clone().into_string(), "Ana  Garc\u{ed}a");
        let longest = "\u{e9}".repeat(DISPLAY_NAME_MAX_CHARS);
        assert_eq!(DisplayName::new(&longest).unwrap().as_str(), longest);
        assert_eq!(DISPLAY_NAME_MAX_CHARS, 120);
    }

    #[test]
    fn display_names_reject_empty_and_overlong_values() {
        for value in ["", "   ", "\u{3000}"] {
            assert_eq!(
                DisplayName::new(value),
                Err(DomainError::InvalidDisplayName("length")),
                "{value:?}"
            );
        }
        assert_eq!(
            DisplayName::new(&"a".repeat(121)),
            Err(DomainError::InvalidDisplayName("length"))
        );
    }

    #[test]
    fn display_names_reject_control_and_format_characters_before_trimming() {
        let rejected = [
            "a\u{0}b",
            "\tAna",
            "Ana\n",
            "\u{85}Ana",
            "Ana\u{7f}",
            "Ana\u{2028}",
            "Ana\u{2029}",
            "\u{ad}Ana",
            "\u{600}Ana",
            "\u{61c}Ana",
            "\u{6dd}Ana",
            "\u{70f}Ana",
            "\u{890}Ana",
            "\u{8e2}Ana",
            "\u{180e}Ana",
            "\u{200b}Ana",
            "\u{200e}Ana",
            "\u{202e}Ana",
            "\u{2060}Ana",
            "\u{2066}Ana",
            "\u{206f}Ana",
            "\u{feff}Ana",
            "\u{fffb}Ana",
            "\u{110bd}Ana",
            "\u{110cd}Ana",
            "\u{1343f}Ana",
            "\u{1bca3}Ana",
            "\u{1d17a}Ana",
            "\u{e0001}Ana",
            "\u{e007f}Ana",
        ];
        for value in rejected {
            assert_eq!(
                DisplayName::new(value),
                Err(DomainError::InvalidDisplayName(
                    "control or format characters"
                )),
                "{value:?}"
            );
        }
        for value in ["Ana\u{2065}", "\u{e000}Ana", "Ana \u{1f600}"] {
            assert!(DisplayName::new(value).is_ok(), "{value:?}");
        }
    }

    #[test]
    fn stored_display_names_must_already_be_canonical() {
        assert_eq!(DisplayName::from_stored("Ana").unwrap().as_str(), "Ana");
        assert_eq!(
            DisplayName::from_stored(" x"),
            Err(DomainError::InvalidDisplayName("not canonical"))
        );
        assert_eq!(
            DisplayName::from_stored("a\u{202e}"),
            Err(DomainError::InvalidDisplayName(
                "control or format characters"
            ))
        );
        assert!(DisplayName::from_stored("").is_err());
    }
}
