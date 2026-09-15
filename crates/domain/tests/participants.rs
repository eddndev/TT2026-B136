use std::str::FromStr;

use domain::identity::{Permission, Role};
use domain::participants::{
    DirectoryStatus, ParticipantId, ParticipantRevision, ParticipantValues,
};
use domain::DomainError;

fn values(
    name: &str,
    role: &str,
    organization: Option<&str>,
    legal: Option<&str>,
) -> ParticipantValues {
    ParticipantValues::new(name, role, organization, legal, DirectoryStatus::Active).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn canonical_vectors_preserve_utf8_lengths_optionality_and_status() {
    for (name, role, organization, legal, status, expected) in [
        ("Ana", "Witness", None, None, DirectoryStatus::Active,
            "504152543100000003416e61000000075769746e657373000000"),
        ("Jos\u{e9}", "Defensor", Some("Despacho"), Some("Registrado"), DirectoryStatus::Archived,
            "5041525431000000054a6f73c3a900000008446566656e736f720100000008446573706163686f010000000a5265676973747261646f01"),
        ("A\u{10000}", "Rol", Some("a,b"), None, DirectoryStatus::Active,
            "50415254310000000541f090808000000003526f6c0100000003612c620000"),
    ] {
        let actual = ParticipantValues::new(name, role, organization, legal, status).unwrap();
        assert_eq!(hex(&actual.canonical_bytes()), expected);
    }
}

#[test]
fn normalization_trims_edges_without_changing_interior_or_unicode_forms() {
    let value = values(
        "  A\u{301}na,  B  ",
        "\u{2003} Defensa \u{a0}",
        Some("  A,B  "),
        Some("\u{2003} "),
    );
    assert_eq!(value.display_name(), "A\u{301}na,  B");
    assert_eq!(value.procedural_role(), "Defensa");
    assert_eq!(value.organization(), Some("A,B"));
    assert_eq!(value.legal_status(), None);
    assert_eq!(value.directory_status(), DirectoryStatus::Active);
    assert_ne!(
        values("\u{c1}", "R", None, None),
        values("A\u{301}", "R", None, None)
    );
    assert_ne!(
        values("Ana", "R", None, None),
        values("ana", "R", None, None)
    );
    for code in [
        32, 160, 5760, 8192, 8193, 8194, 8195, 8196, 8197, 8198, 8199, 8200, 8201, 8202, 8232,
        8233, 8239, 8287, 12288,
    ] {
        let space = char::from_u32(code).unwrap();
        assert_eq!(
            values(&format!("{space}Ana{space}"), "R", None, None).display_name(),
            "Ana"
        );
    }
}

#[test]
fn controls_are_rejected_on_each_original_input_before_trimming() {
    for code in (0..=31).chain(127..=159) {
        let control = char::from_u32(code).unwrap();
        let invalid = format!("{control}x{control}");
        for (field, result) in [
            (
                "display_name",
                ParticipantValues::new(&invalid, "R", None, None, DirectoryStatus::Active),
            ),
            (
                "procedural_role",
                ParticipantValues::new("A", &invalid, None, None, DirectoryStatus::Active),
            ),
            (
                "organization",
                ParticipantValues::new("A", "R", Some(&invalid), None, DirectoryStatus::Active),
            ),
            (
                "legal_status",
                ParticipantValues::new("A", "R", None, Some(&invalid), DirectoryStatus::Active),
            ),
        ] {
            let error = result.unwrap_err();
            assert!(
                matches!(error, DomainError::InvalidParticipantValues { field: actual, .. } if actual == field)
            );
            assert!(!error.to_string().contains(&invalid));
        }
    }
}

#[test]
fn required_fields_and_scalar_maxima_bound_the_complete_encoding() {
    for (name, role) in [("", "R"), (" \u{2003}", "R"), ("A", ""), ("A", "  ")] {
        assert!(ParticipantValues::new(name, role, None, None, DirectoryStatus::Active).is_err());
    }
    let name = "\u{1f600}".repeat(200);
    let role = "\u{1f600}".repeat(80);
    let legal = "\u{1f600}".repeat(160);
    let maximum = values(&name, &role, Some(&name), Some(&legal));
    assert_eq!(maximum.canonical_bytes().len(), 2584);
    assert!(ParticipantValues::new(
        &(name.clone() + "x"),
        "R",
        None,
        None,
        DirectoryStatus::Active
    )
    .is_err());
    assert!(
        ParticipantValues::new("A", &(role + "x"), None, None, DirectoryStatus::Active).is_err()
    );
    assert!(
        ParticipantValues::new("A", "R", Some(&(name + "x")), None, DirectoryStatus::Active)
            .is_err()
    );
    assert!(ParticipantValues::new(
        "A",
        "R",
        None,
        Some(&(legal + "x")),
        DirectoryStatus::Active
    )
    .is_err());
    assert_eq!(
        values("A", "R", Some(" "), Some("")),
        values("A", "R", None, None)
    );
}

#[test]
fn status_changes_preserve_text_and_canonical_fields_are_unambiguous() {
    let active = values("Ana", "Defensa", Some("Despacho"), Some("Manual"));
    let archived = active.with_directory_status(DirectoryStatus::Archived);
    assert_eq!(archived.display_name(), active.display_name());
    assert_eq!(archived.procedural_role(), active.procedural_role());
    assert_eq!(archived.organization(), active.organization());
    assert_eq!(archived.legal_status(), active.legal_status());
    assert_eq!(archived.directory_status(), DirectoryStatus::Archived);
    assert_eq!(
        archived.with_directory_status(DirectoryStatus::Active),
        active
    );
    let mut expected = active.canonical_bytes();
    *expected.last_mut().unwrap() = 1;
    assert_eq!(archived.canonical_bytes(), expected);
    let samples = [
        values("ab", "c", None, None),
        values("a", "bc", None, None),
        values("a", "b", Some("c"), None),
        values("a", "b", None, Some("c")),
    ];
    for (index, left) in samples.iter().enumerate() {
        for right in &samples[index + 1..] {
            assert_ne!(left.canonical_bytes(), right.canonical_bytes());
        }
    }
}

#[test]
fn directory_status_has_only_two_exact_organizational_values() {
    for (text, status) in [
        ("active", DirectoryStatus::Active),
        ("archived", DirectoryStatus::Archived),
    ] {
        assert_eq!(DirectoryStatus::from_str(text).unwrap(), status);
        assert_eq!(status.as_str(), text);
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, format!("\"{text}\""));
        assert_eq!(
            serde_json::from_str::<DirectoryStatus>(&serialized).unwrap(),
            status
        );
    }
    for text in ["", "ACTIVE", " active", "archived ", "pending"] {
        assert!(matches!(
            DirectoryStatus::from_str(text),
            Err(DomainError::InvalidDirectoryStatus(_))
        ));
        assert!(serde_json::from_str::<DirectoryStatus>(&format!("\"{text}\"")).is_err());
    }
}

#[test]
fn participant_identity_and_revision_preserve_their_invariants() {
    let id = ParticipantId::new();
    assert_ne!(id, ParticipantId::default());
    assert_eq!(ParticipantId::from_uuid(id.as_uuid()), id);
    assert_eq!(id.to_string(), id.as_uuid().to_string());
    assert_eq!(
        serde_json::from_str::<ParticipantId>(&serde_json::to_string(&id).unwrap()).unwrap(),
        id
    );
    assert!(format!("{id:?}").contains("ParticipantId"));
    assert_eq!(ParticipantRevision::initial().get(), 1);
    assert_eq!(
        ParticipantRevision::try_from(0),
        Err(DomainError::InvalidParticipantRevision)
    );
    assert_eq!(
        ParticipantRevision::new(0),
        Err(DomainError::InvalidParticipantRevision)
    );
    let second = ParticipantRevision::initial().next().unwrap();
    assert_eq!(second, ParticipantRevision::new(2).unwrap());
    assert!(second > ParticipantRevision::initial());
    assert_eq!(serde_json::to_string(&second).unwrap(), "2");
    assert_eq!(ParticipantRevision::new(u32::MAX).unwrap().next(), None);
}

#[test]
fn participant_permissions_are_independent_from_procedural_role_text() {
    for (role, read, manage) in [
        (Role::Owner, true, true),
        (Role::Litigator, true, true),
        (Role::Paralegal, true, false),
        (Role::Client, false, false),
    ] {
        assert_eq!(role.allows(Permission::ReadParticipant), read);
        assert_eq!(role.allows(Permission::ManageParticipant), manage);
    }
}
