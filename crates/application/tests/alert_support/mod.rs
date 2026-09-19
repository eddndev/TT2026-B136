#![allow(dead_code)]
use application::{alerts::*, identity::Principal, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    identity::{Role, UserId},
};
use std::sync::{Arc, Mutex};
use time::{macros::datetime, OffsetDateTime};
use uuid::Uuid;

pub type Events = Arc<Mutex<Vec<&'static str>>>;
pub fn at() -> OffsetDateTime {
    datetime!(2026-09-01 12:00:00.123456789 UTC)
}
pub fn actor(role: Role) -> Principal {
    Principal {
        id: UserId::new(),
        email: "operator@example.test".into(),
        role,
    }
}
pub fn id(n: u128) -> AlertId {
    AlertId::from_uuid(Uuid::from_u128(n))
}
pub fn operation(n: u128) -> AlertOperationId {
    AlertOperationId::from_uuid(Uuid::from_u128(n))
}
pub fn query() -> AlertQuery {
    AlertQuery::new(20, AlertReadFilter::All, AlertStateFilter::Active, None).unwrap()
}
pub fn record(recipient: UserId, n: u128) -> AlertRecord {
    AlertRecord {
        id: id(n),
        recipient_id: recipient,
        occurrence_id: AlertOccurrenceId::from_uuid(Uuid::from_u128(10)),
        subject: AlertSubject::Deadline {
            case_id: CaseId::from_uuid(Uuid::from_u128(11)),
            id: application::deadlines::DeadlineId::from_uuid(Uuid::from_u128(12)),
        },
        subject_title: "Captured deadline".into(),
        case_title: "Captured case".into(),
        case_reference: "CASE-11".into(),
        kind: AlertKind::OverdueUnattended { due_at: at() },
        origin: AlertOrigin {
            revision: 1,
            evidence_digest: Sha256Digest::from_array([1; 32]),
        },
        trigger_at: at(),
        created_at: at(),
        read_at: None,
        state: AlertState::Active,
        email: AlertEmailStatus::Disabled,
    }
}
pub fn page(recipient: UserId) -> AlertPage {
    AlertPage {
        checked_at: at(),
        alerts: vec![record(recipient, 1)],
        has_more: false,
        next_cursor: None,
    }
}
pub fn preference_command() -> AlertPreferenceCommand {
    AlertPreferenceCommand {
        operation_id: operation(20),
        expected_revision: 0,
        values: AlertPreferenceValues::default(),
    }
}
pub fn saved(actor: UserId, command: &AlertPreferenceCommand) -> AlertPreferences {
    AlertPreferences {
        user_id: actor,
        revision: command.expected_revision + 1,
        values: command.values.clone(),
        updated_at: Some(at()),
        receipt: Some(AlertPreferenceReceipt {
            operation_id: command.operation_id,
            expected_revision: command.expected_revision,
        }),
        email_transport: AlertEmailTransport::Ready,
    }
}

pub enum Reply {
    Preferences(Result<AlertPreferences, ApplicationError>),
    Page(Result<AlertPage, ApplicationError>),
    Detail(Result<AlertDetail, ApplicationError>),
    Read(Result<AlertReadReceipt, ApplicationError>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    Preferences(UserId),
    Save(UserId, AlertPreferenceCommand),
    List(UserId, AlertQuery),
    Get(UserId, AlertId),
    Read(UserId, AlertReadCommand),
}
pub struct Store {
    pub events: Events,
    pub calls: Mutex<Vec<Call>>,
    pub replies: Mutex<Vec<Reply>>,
}
impl Store {
    fn take(&self, call: Call) -> Reply {
        self.events.lock().unwrap().push("store");
        self.calls.lock().unwrap().push(call);
        self.replies.lock().unwrap().remove(0)
    }
}
impl AlertStore for Store {
    fn preferences(&self, actor: UserId) -> Result<AlertPreferences, ApplicationError> {
        match self.take(Call::Preferences(actor)) {
            Reply::Preferences(value) => value,
            _ => panic!("preferences reply"),
        }
    }
    fn save_preferences(
        &self,
        actor: UserId,
        command: AlertPreferenceCommand,
    ) -> Result<AlertPreferences, ApplicationError> {
        match self.take(Call::Save(actor, command)) {
            Reply::Preferences(value) => value,
            _ => panic!("preferences reply"),
        }
    }
    fn list(&self, actor: UserId, query: AlertQuery) -> Result<AlertPage, ApplicationError> {
        match self.take(Call::List(actor, query)) {
            Reply::Page(value) => value,
            _ => panic!("page reply"),
        }
    }
    fn get(&self, actor: UserId, id: AlertId) -> Result<AlertDetail, ApplicationError> {
        match self.take(Call::Get(actor, id)) {
            Reply::Detail(value) => value,
            _ => panic!("detail reply"),
        }
    }
    fn mark_read(
        &self,
        actor: UserId,
        command: AlertReadCommand,
    ) -> Result<AlertReadReceipt, ApplicationError> {
        match self.take(Call::Read(actor, command)) {
            Reply::Read(value) => value,
            _ => panic!("read reply"),
        }
    }
}
