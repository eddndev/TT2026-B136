use std::io::Read;

use application::participants::{
    participant_digest, DirectoryStatus, ParticipantAction, ParticipantHistoryQuery, ParticipantId,
    ParticipantQuery, ParticipantRevision, ParticipantStatusFilter, ParticipantValues,
};
use application::ApplicationError;
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::identity::Permission;
use domain::DomainError;

#[test]
fn list_queries_preserve_literal_filters_and_exclusive_uuid_cursor() {
    let id = ParticipantId::new();
    for (status, state) in [
        (
            ParticipantStatusFilter::Active,
            Some(DirectoryStatus::Active),
        ),
        (
            ParticipantStatusFilter::Archived,
            Some(DirectoryStatus::Archived),
        ),
        (ParticipantStatusFilter::All, None),
    ] {
        let q = ParticipantQuery::new(
            100,
            Some(id),
            Some(" \u{c1}na%_,  B "),
            Some(" Defensa "),
            status,
        )
        .unwrap();
        assert_eq!(q.limit(), 100);
        assert_eq!(q.after_id(), Some(id));
        assert_eq!(q.name(), Some("\u{c1}na%_,  B"));
        assert_eq!(q.procedural_role(), Some("Defensa"));
        assert_eq!(q.status(), status);
        assert_eq!(q.status().directory_status(), state);
    }
    assert_eq!(
        ParticipantStatusFilter::default(),
        ParticipantStatusFilter::Active
    );
    let q = ParticipantQuery::new(
        1,
        None,
        Some(" \u{2003}"),
        Some(""),
        ParticipantStatusFilter::default(),
    )
    .unwrap();
    assert_eq!(q.after_id(), None);
    assert_eq!(q.name(), None);
    assert_eq!(q.procedural_role(), None);
}

#[test]
fn list_queries_reject_controls_and_limits_before_normalizing() {
    for limit in [0, 101, u32::MAX] {
        assert!(matches!(
            ParticipantQuery::new(limit, None, None, None, ParticipantStatusFilter::All),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
    for control in ['\0', '\n', '\t', '\u{85}'] {
        let value = format!("{control}text ");
        for (name, role) in [(Some(value.as_str()), None), (None, Some(value.as_str()))] {
            assert!(matches!(
                ParticipantQuery::new(1, None, name, role, ParticipantStatusFilter::Active),
                Err(ApplicationError::InvalidInput(_))
            ));
        }
    }
    let name = "\u{1f600}".repeat(200);
    let role = "\u{1f600}".repeat(80);
    assert!(ParticipantQuery::new(
        1,
        None,
        Some(&name),
        Some(&role),
        ParticipantStatusFilter::All
    )
    .is_ok());
    assert!(ParticipantQuery::new(
        1,
        None,
        Some(&(name + "a")),
        None,
        ParticipantStatusFilter::All
    )
    .is_err());
    assert!(ParticipantQuery::new(
        1,
        None,
        None,
        Some(&(role + "a")),
        ParticipantStatusFilter::All
    )
    .is_err());
}

#[test]
fn history_queries_require_bounded_pages_and_positive_revisions() {
    for (limit, before) in [(0, None), (101, None), (1, Some(0))] {
        assert!(matches!(
            ParticipantHistoryQuery::new(limit, before),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
    for (limit, before) in [(1, None), (100, Some(1)), (50, Some(u32::MAX))] {
        let query = ParticipantHistoryQuery::new(limit, before).unwrap();
        assert_eq!(query.limit(), limit);
        assert_eq!(
            query.before_revision().map(ParticipantRevision::get),
            before
        );
    }
}

#[test]
fn every_participant_action_has_an_explicit_permission_and_event() {
    for (action, permission, event) in [
        (
            ParticipantAction::Create,
            Permission::ManageParticipant,
            "participant.created",
        ),
        (
            ParticipantAction::Replace,
            Permission::ManageParticipant,
            "participant.updated",
        ),
        (
            ParticipantAction::ChangeStatus,
            Permission::ManageParticipant,
            "participant.directory_status_changed",
        ),
        (
            ParticipantAction::Read,
            Permission::ReadParticipant,
            "participant.read",
        ),
        (
            ParticipantAction::List,
            Permission::ReadParticipant,
            "participant.listed",
        ),
        (
            ParticipantAction::History,
            Permission::ReadParticipant,
            "participant.history_listed",
        ),
    ] {
        assert_eq!(action.permission(), permission);
        assert_eq!(action.audit_action(), event);
    }
}

struct CanonicalHasher;
impl DocumentHasher for CanonicalHasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        assert_eq!(bytes, b"PART1\0\0\0\x03Ana\0\0\0\x07Witness\0\0\0");
        Sha256Digest::from_hex("fc4e619a010411b046c79cbd60bfd6292c9948cbef2d5caea6ae2c17a88263ad")
            .unwrap()
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("bounded participant values use hash_bytes")
    }
}

#[test]
fn participant_digest_uses_the_canonical_bytes_through_the_existing_port() {
    let values =
        ParticipantValues::new("Ana", "Witness", None, None, DirectoryStatus::Active).unwrap();
    assert_eq!(
        participant_digest(&CanonicalHasher, &values).to_hex(),
        "fc4e619a010411b046c79cbd60bfd6292c9948cbef2d5caea6ae2c17a88263ad"
    );
}
