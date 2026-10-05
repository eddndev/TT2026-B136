use crate::{alert_support::*, case_support::MockIdentity};
use application::{alerts::*, deadlines::DeadlineId, hearings::HearingId, ApplicationError};
use domain::{
    alerts::AlertLeadHours,
    cases::CaseId,
    crypto::Sha256Digest,
    identity::{Role, UserId},
    precautionary_hearings::PrecautionaryHearingId,
    procedural_resources::ResourceId,
    resource_hearings::ResourceHearingId,
};
use std::sync::{Arc, Mutex};
use time::Duration;
use uuid::Uuid;

fn subject() -> AlertSubject {
    AlertSubject::PrecautionaryHearing {
        case_id: CaseId::from_uuid(Uuid::from_u128(11)),
        id: PrecautionaryHearingId::from_uuid(Uuid::from_u128(13)),
    }
}

fn upcoming(recipient: UserId) -> AlertRecord {
    AlertRecord {
        subject: subject(),
        subject_title: "Captured precautionary review appointment".into(),
        kind: AlertKind::Upcoming {
            lead_hours: AlertLeadHours::new(48).unwrap(),
            activity_at: at() + Duration::hours(48),
        },
        origin: AlertOrigin {
            revision: 2,
            evidence_digest: Sha256Digest::from_array([3; 32]),
        },
        ..record(recipient, 1)
    }
}

fn stored<T: std::fmt::Debug>(result: Result<T, ApplicationError>) {
    assert!(matches!(
        result,
        Err(ApplicationError::Alert(AlertError::Stored(_)))
    ));
}

#[test]
fn precautionary_subject_preserves_case_and_typed_identity_across_four_families() {
    let own = subject();
    let case_id = CaseId::from_uuid(Uuid::from_u128(11));
    let common_id = Uuid::from_u128(13);
    assert_eq!(own.case_id(), case_id);
    for other in [
        AlertSubject::Hearing {
            case_id,
            id: HearingId::from_uuid(common_id),
        },
        AlertSubject::Deadline {
            case_id,
            id: DeadlineId::from_uuid(common_id),
        },
        AlertSubject::ResourceHearing {
            case_id,
            resource_id: ResourceId::from_uuid(Uuid::from_u128(12)),
            id: ResourceHearingId::from_uuid(common_id),
        },
        AlertSubject::PrecautionaryHearing {
            case_id: CaseId::from_uuid(Uuid::from_u128(12)),
            id: PrecautionaryHearingId::from_uuid(common_id),
        },
        AlertSubject::PrecautionaryHearing {
            case_id,
            id: PrecautionaryHearingId::from_uuid(Uuid::from_u128(14)),
        },
    ] {
        assert_ne!(own, other);
    }
}

#[test]
fn precautionary_alert_accepts_positive_historical_revisions_and_only_upcoming_kind() {
    let actor = actor(Role::Owner);
    let valid = upcoming(actor.id);
    for revision in [1, 2, 3, u32::MAX] {
        let mut row = valid.clone();
        row.origin.revision = revision;
        row.validate(actor.id, at()).unwrap();
        assert_eq!(row.subject, subject());
        assert_eq!(row.origin.evidence_digest, valid.origin.evidence_digest);
    }
    let mut zero = valid.clone();
    zero.origin.revision = 0;
    stored(zero.validate(actor.id, at()));
    for kind in [
        AlertKind::OverdueUnattended { due_at: at() },
        AlertKind::ReviewRequired,
        AlertKind::DueChangedSoon {
            previous_due_at: at() + Duration::hours(72),
            current_due_at: at() + Duration::hours(48),
        },
    ] {
        let mut row = valid.clone();
        row.kind = kind;
        stored(row.validate(actor.id, at()));
    }
    stored(valid.validate(UserId::new(), at()));
}

#[test]
fn precautionary_alert_preserves_exact_trigger_and_activity_window() {
    let actor = actor(Role::Litigator);
    let valid = upcoming(actor.id);
    valid.validate(actor.id, at()).unwrap();
    assert_eq!(valid.trigger_at.nanosecond(), 123_456_789);
    let activity_at = at() + Duration::hours(48);
    let mut last = valid.clone();
    last.created_at = activity_at - Duration::nanoseconds(1);
    last.validate(actor.id, activity_at).unwrap();
    for created_at in [at() - Duration::nanoseconds(1), activity_at] {
        let mut row = valid.clone();
        row.created_at = created_at;
        stored(row.validate(actor.id, activity_at));
    }
    let mut wrong_trigger = valid;
    wrong_trigger.trigger_at -= Duration::nanoseconds(1);
    stored(wrong_trigger.validate(actor.id, at()));
}

#[test]
fn repeated_precautionary_read_preserves_historical_origin_and_receipt_identity() {
    let actor = actor(Role::Paralegal);
    let mut alert = upcoming(actor.id);
    alert.read_at = Some(at());
    let expected = AlertReadReceipt {
        operation_id: operation(21),
        checked_at: at(),
        alert,
    };
    let command = AlertReadCommand {
        operation_id: operation(21),
        alert_id: id(1),
    };
    let events = Arc::new(Mutex::new(vec![]));
    let store = Arc::new(Store {
        events: events.clone(),
        calls: Mutex::new(vec![]),
        replies: Mutex::new(vec![
            Reply::Read(Ok(expected.clone())),
            Reply::Read(Ok(expected.clone())),
        ]),
    });
    let mut identity = MockIdentity::new();
    let current = actor.clone();
    let log = events.clone();
    identity
        .expect_authenticate()
        .withf(|token| token == "session")
        .times(4)
        .returning(move |_| {
            log.lock().unwrap().push("auth");
            Ok(current.clone())
        });
    let service = AlertService::new(store.clone(), Arc::new(identity));
    for _ in 0..2 {
        let receipt = service.mark_read("session", command).unwrap();
        assert_eq!(receipt, expected);
        assert_eq!(receipt.alert.subject, subject());
        assert_eq!(receipt.alert.origin, upcoming(actor.id).origin);
    }
    assert_eq!(
        *store.calls.lock().unwrap(),
        vec![Call::Read(actor.id, command); 2]
    );
    assert_eq!(
        *events.lock().unwrap(),
        vec!["auth", "store", "auth", "auth", "store", "auth"]
    );
}
