use crate::{alert_support::*, case_support::MockIdentity};
use application::{alerts::*, deadlines::DeadlineId, hearings::HearingId, ApplicationError};
use domain::{
    alerts::AlertLeadHours,
    cases::CaseId,
    identity::{Role, UserId},
    procedural_resources::ResourceId,
    resource_hearings::ResourceHearingId,
};
use std::sync::{Arc, Mutex};
use time::Duration;
use uuid::Uuid;

fn subject() -> AlertSubject {
    AlertSubject::ResourceHearing {
        case_id: CaseId::from_uuid(Uuid::from_u128(11)),
        resource_id: ResourceId::from_uuid(Uuid::from_u128(12)),
        id: ResourceHearingId::from_uuid(Uuid::from_u128(13)),
    }
}

fn upcoming(recipient: UserId) -> AlertRecord {
    AlertRecord {
        subject: subject(),
        subject_title: "Captured appeal hearing".into(),
        kind: AlertKind::Upcoming {
            lead_hours: AlertLeadHours::new(48).unwrap(),
            activity_at: at() + Duration::hours(48),
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
fn resource_hearing_subject_keeps_parent_and_distinct_typed_identity() {
    let own = subject();
    let case_id = CaseId::from_uuid(Uuid::from_u128(11));
    let common_id = Uuid::from_u128(13);
    assert_eq!(own.case_id(), case_id);
    assert_ne!(
        own,
        AlertSubject::Hearing {
            case_id,
            id: HearingId::from_uuid(common_id),
        }
    );
    assert_ne!(
        own,
        AlertSubject::Deadline {
            case_id,
            id: DeadlineId::from_uuid(common_id),
        }
    );
    assert_ne!(
        own,
        AlertSubject::ResourceHearing {
            case_id,
            resource_id: ResourceId::from_uuid(Uuid::from_u128(14)),
            id: ResourceHearingId::from_uuid(common_id),
        }
    );
}

#[test]
fn resource_hearing_notification_accepts_only_initial_upcoming_capture() {
    let actor = actor(Role::Owner);
    let valid = upcoming(actor.id);
    valid.validate(actor.id, at()).unwrap();
    for revision in [0, 2, u32::MAX] {
        let mut invalid = valid.clone();
        invalid.origin.revision = revision;
        stored(invalid.validate(actor.id, at()));
    }
    for kind in [
        AlertKind::OverdueUnattended { due_at: at() },
        AlertKind::ReviewRequired,
        AlertKind::DueChangedSoon {
            previous_due_at: at() + Duration::hours(72),
            current_due_at: at() + Duration::hours(48),
        },
    ] {
        let mut invalid = valid.clone();
        invalid.kind = kind;
        stored(invalid.validate(actor.id, at()));
    }
    let mut ordinary = valid;
    ordinary.subject = AlertSubject::Hearing {
        case_id: ordinary.subject.case_id(),
        id: HearingId::from_uuid(Uuid::from_u128(13)),
    };
    ordinary.origin.revision = 2;
    ordinary.validate(actor.id, at()).unwrap();
}

#[test]
fn resource_hearing_notification_preserves_exact_activity_and_trigger() {
    let actor = actor(Role::Litigator);
    let valid = upcoming(actor.id);
    let activity_at = at() + Duration::hours(48);
    assert_eq!(valid.trigger_at.nanosecond(), 123_456_789);
    assert_eq!(
        valid.kind,
        AlertKind::Upcoming {
            lead_hours: AlertLeadHours::new(48).unwrap(),
            activity_at,
        }
    );
    valid.validate(actor.id, at()).unwrap();
    let mut last_instant = valid.clone();
    last_instant.created_at = activity_at - Duration::nanoseconds(1);
    last_instant.validate(actor.id, activity_at).unwrap();
    for created_at in [at() - Duration::nanoseconds(1), activity_at] {
        let mut invalid = valid.clone();
        invalid.created_at = created_at;
        stored(invalid.validate(actor.id, activity_at));
    }
    let mut invalid = valid;
    invalid.trigger_at -= Duration::nanoseconds(1);
    stored(invalid.validate(actor.id, at()));
}

#[test]
fn resource_hearing_read_keeps_subject_origin_and_receipt_binding() {
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
    let events = Arc::new(Mutex::new(Vec::new()));
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
        assert_eq!(receipt.alert.state, AlertState::Active);
        assert_eq!(receipt.alert.email, AlertEmailStatus::Disabled);
    }
    assert_eq!(
        *store.calls.lock().unwrap(),
        vec![Call::Read(actor.id, command), Call::Read(actor.id, command)]
    );
    assert_eq!(
        *events.lock().unwrap(),
        vec!["auth", "store", "auth", "auth", "store", "auth"]
    );
    let mut wrong_operation = expected.clone();
    wrong_operation.operation_id = operation(22);
    let mut wrong_alert = expected.clone();
    wrong_alert.alert.id = id(2);
    let mut unread = expected;
    unread.alert.read_at = None;
    for receipt in [wrong_operation, wrong_alert, unread] {
        let rejected_store = Arc::new(Store {
            events: Arc::new(Mutex::new(vec![])),
            calls: Mutex::new(vec![]),
            replies: Mutex::new(vec![Reply::Read(Ok(receipt))]),
        });
        let mut identity = MockIdentity::new();
        let current = actor.clone();
        identity
            .expect_authenticate()
            .withf(|token| token == "session")
            .times(1)
            .returning(move |_| Ok(current.clone()));
        let service = AlertService::new(rejected_store, Arc::new(identity));
        stored(service.mark_read("session", command));
    }
}
