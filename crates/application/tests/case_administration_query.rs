use std::io::Read;

use application::cases::{
    case_administration_digest, CaseActorSnapshot, CaseAdministrationAction,
    CaseAdministrationHistoryQuery, CaseAdministrationQuery, CaseAdministrationSnapshot,
    CaseAdministrationValues, CaseProfileFilter, CaseRevision, CaseRevisionExpectation,
    CaseStatusFilter, CurrentCaseAdministration,
};
use application::ApplicationError;
use domain::cases::{CaseId, CaseMetadata};
use domain::clock::OffsetDateTime;
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::identity::{Permission, UserId};
use domain::DomainError;

#[test]
fn queries_preserve_literals_exact_identifiers_and_independent_filters() {
    let id = CaseId::new();
    for status in [
        CaseStatusFilter::Active,
        CaseStatusFilter::Closed,
        CaseStatusFilter::All,
    ] {
        for profile in [
            CaseProfileFilter::Complete,
            CaseProfileFilter::Pending,
            CaseProfileFilter::All,
        ] {
            let q = CaseAdministrationQuery::new(
                100,
                Some(id),
                status,
                profile,
                Some(" \u{c1}na%_, B "),
                Some(" Nuc-1 "),
                Some(" cj_1% "),
            )
            .unwrap();
            assert_eq!(q.limit(), 100);
            assert_eq!(q.after_id(), Some(id));
            assert_eq!(q.status(), status);
            assert_eq!(q.profile(), profile);
            assert_eq!(q.title(), Some("\u{c1}na%_, B"));
            assert_eq!(q.nuc(), Some("Nuc-1"));
            assert_eq!(q.judicial_case_number(), Some("cj_1%"));
        }
    }
    assert_eq!(CaseStatusFilter::default(), CaseStatusFilter::Active);
    assert_eq!(CaseProfileFilter::default(), CaseProfileFilter::All);
    let q = CaseAdministrationQuery::new(
        1,
        None,
        CaseStatusFilter::default(),
        CaseProfileFilter::default(),
        Some(" \u{2003}"),
        Some(""),
        None,
    )
    .unwrap();
    assert_eq!(q.after_id(), None);
    assert_eq!(
        (q.title(), q.nuc(), q.judicial_case_number()),
        (None, None, None)
    );
}

#[test]
fn queries_reject_original_controls_and_out_of_range_limits_without_secret_echo() {
    for limit in [0, 101, u32::MAX] {
        assert!(CaseAdministrationQuery::new(
            limit,
            None,
            CaseStatusFilter::All,
            CaseProfileFilter::All,
            None,
            None,
            None
        )
        .is_err());
    }
    for control in ['\0', '\n', '\r', '\t', '\u{85}'] {
        let secret = format!("{control}Secret");
        for fields in [
            [Some(secret.as_str()), None, None],
            [None, Some(secret.as_str()), None],
            [None, None, Some(secret.as_str())],
        ] {
            let error = CaseAdministrationQuery::new(
                1,
                None,
                CaseStatusFilter::All,
                CaseProfileFilter::All,
                fields[0],
                fields[1],
                fields[2],
            )
            .unwrap_err();
            assert!(matches!(error, ApplicationError::InvalidInput(_)));
            assert!(!error.to_string().contains("Secret"));
        }
    }
    let title = "\u{10000}".repeat(200);
    let id = "\u{10000}".repeat(100);
    assert!(CaseAdministrationQuery::new(
        1,
        None,
        CaseStatusFilter::All,
        CaseProfileFilter::All,
        Some(&title),
        Some(&id),
        Some(&id)
    )
    .is_ok());
    let title = title + "x";
    let id = id + "x";
    for fields in [
        [Some(title.as_str()), None, None],
        [None, Some(id.as_str()), None],
        [None, None, Some(id.as_str())],
    ] {
        assert!(CaseAdministrationQuery::new(
            1,
            None,
            CaseStatusFilter::All,
            CaseProfileFilter::All,
            fields[0],
            fields[1],
            fields[2]
        )
        .is_err());
    }
}

#[test]
fn history_cursors_are_positive_but_expected_zero_is_an_explicit_baseline() {
    for (limit, before) in [(0, None), (101, None), (1, Some(0))] {
        assert!(matches!(
            CaseAdministrationHistoryQuery::new(limit, before),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
    for (limit, before) in [(1, None), (100, Some(1)), (50, Some(u32::MAX))] {
        let q = CaseAdministrationHistoryQuery::new(limit, before).unwrap();
        assert_eq!(q.limit(), limit);
        assert_eq!(q.before_revision().map(CaseRevision::get), before);
    }
    assert_eq!(
        CaseRevisionExpectation::new(0),
        CaseRevisionExpectation::Unrevised
    );
    for value in [1, 2, u32::MAX] {
        let expected = CaseRevisionExpectation::new(value);
        assert_eq!(
            expected,
            CaseRevisionExpectation::Revision(CaseRevision::new(value).unwrap())
        );
        assert_eq!(expected.get(), value);
    }
    assert_eq!(CaseRevisionExpectation::Unrevised.get(), 0);
}

#[test]
fn baseline_projection_has_no_fabricated_digest_author_or_time() {
    let metadata = CaseMetadata::new("Baseline", "REF").unwrap();
    let current = CurrentCaseAdministration::Unrevised(metadata.clone());
    assert_eq!(current.revision(), None);
    assert_eq!(current.snapshot(), None);
    assert_eq!(current.values(), CaseAdministrationValues::basic(metadata));
}

#[test]
fn recorded_basic_revision_still_has_no_penal_profile() {
    let snapshot = CaseAdministrationSnapshot {
        case_id: CaseId::new(),
        revision: CaseRevision::FIRST,
        values: CaseAdministrationValues::basic(CaseMetadata::new("Basic", "REF").unwrap()),
        values_digest: Sha256Digest::from_bytes(&[7; 32]).unwrap(),
        changed_at: OffsetDateTime::from_unix_timestamp(123).unwrap(),
        changed_by: CaseActorSnapshot {
            id: UserId::new(),
            email: "captured@example.com".into(),
        },
    };
    let current = CurrentCaseAdministration::Recorded(Box::new(snapshot.clone()));
    assert_eq!(current.revision(), Some(CaseRevision::FIRST));
    assert_eq!(current.snapshot(), Some(&snapshot));
    assert_eq!(current.values(), snapshot.values);
    assert_eq!(current.values().profile(), None);
}

#[test]
fn administration_actions_have_specific_permissions_and_events() {
    for (action, permission, event) in [
        (
            CaseAdministrationAction::RegisterPenal,
            Permission::ManageCaseAdministration,
            "case.penal_registered",
        ),
        (
            CaseAdministrationAction::Replace,
            Permission::ManageCaseAdministration,
            "case.administration_replaced",
        ),
        (
            CaseAdministrationAction::ChangeStatus,
            Permission::ManageCaseAdministration,
            "case.administrative_status_changed",
        ),
        (
            CaseAdministrationAction::List,
            Permission::ReadCaseAdministration,
            "case.administration_listed",
        ),
        (
            CaseAdministrationAction::Read,
            Permission::ReadCaseAdministration,
            "case.administration_read",
        ),
        (
            CaseAdministrationAction::History,
            Permission::ReadCaseAdministration,
            "case.administration_history_read",
        ),
    ] {
        assert_eq!(action.permission(), permission);
        assert_eq!(action.audit_action(), event);
    }
}

struct CanonicalHasher;
impl DocumentHasher for CanonicalHasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        assert_eq!(bytes, b"CADM1\0\0\0\0\x0aExpediente\0\0\0\x07REF-001\0");
        Sha256Digest::from_hex("b6428497e7da2bb7e653e3b6da6d97d3cdb7ec4f34e0f09d6f8c00390bdaa0dd")
            .unwrap()
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("bounded administration values use hash_bytes")
    }
}

#[test]
fn values_digest_uses_cadm1_through_the_hash_port() {
    let values =
        CaseAdministrationValues::basic(CaseMetadata::new("Expediente", "REF-001").unwrap());
    assert_eq!(
        case_administration_digest(&CanonicalHasher, &values).to_hex(),
        "b6428497e7da2bb7e653e3b6da6d97d3cdb7ec4f34e0f09d6f8c00390bdaa0dd"
    );
}
