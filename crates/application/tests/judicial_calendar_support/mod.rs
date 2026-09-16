#![allow(dead_code)]
#[allow(unused_imports)]
pub use crate::case_support::{identity, instant, CountingClock, MockIdentity};
use application::{judicial_calendars::*, ApplicationError};
use domain::{
    clock::OffsetDateTime,
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
    DomainError,
};
use mockall::mock;
use std::{io::Read, sync::Arc};
mock! {
    pub Store {}
    impl JudicialCalendarStore for Store {
        fn list(&self,actor:UserId,query:JudicialCalendarQuery,at:OffsetDateTime)->Result<JudicialCalendarPage,ApplicationError>;
        fn get(&self,actor:UserId,id:JudicialCalendarId,revision:Option<JudicialCalendarRevision>,at:OffsetDateTime)->Result<JudicialCalendarDetail,ApplicationError>;
        fn history(&self,actor:UserId,id:JudicialCalendarId,query:JudicialCalendarHistoryQuery,at:OffsetDateTime)->Result<JudicialCalendarHistoryPage,ApplicationError>;
        fn prepare(&self,actor:UserId,command:&JudicialCalendarCommand)->Result<JudicialCalendarPreparation,ApplicationError>;
        fn commit(&self,actor:UserId,prepared:PreparedJudicialCalendarChange)->Result<JudicialCalendarDetail,ApplicationError>;
    }
}
/// A deterministic port double; independent wire vectors test the real encoding.
pub struct Hasher;
impl DocumentHasher for Hasher {
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
        let mut bytes = [0u8; 32];
        for (i, b) in data.iter().enumerate() {
            let slot = i % 32;
            bytes[slot] = bytes[slot].wrapping_add(*b).wrapping_add(i as u8);
        }
        Sha256Digest::from_array(bytes)
    }
    fn hash_stream(&self, r: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        let mut v = vec![];
        r.read_to_end(&mut v).map_err(|e| DomainError::StreamRead {
            message: e.to_string(),
        })?;
        Ok(self.hash_bytes(&v))
    }
}
pub fn values(title: &str) -> JudicialCalendarValues {
    let scope = JudicialCalendarScope::new(JudicialCalendarScopeInput {
        title,
        jurisdiction: JudicialCalendarJurisdiction::Federal,
        entity_codes: &["09"],
        authority: "Authority",
        organ: "Court",
        territory: "Territory",
        use_description: "Declared purpose",
    })
    .unwrap();
    let pattern = (1..=7)
        .map(|d| {
            JudicialCalendarWeekdayRule::new(
                d,
                JudicialCalendarRule::new(
                    JudicialCalendarClassification::Unresolved,
                    vec![],
                    "Review source",
                )
                .unwrap(),
            )
            .unwrap()
        })
        .collect();
    JudicialCalendarValues::new(
        scope,
        JudicialCalendarCoverage::new("2026-01-01".parse().unwrap(), "2026-12-31".parse().unwrap())
            .unwrap(),
        vec![],
        pattern,
        vec![],
    )
    .unwrap()
}
pub fn command() -> JudicialCalendarCommand {
    JudicialCalendarCommand {
        operation_id: JudicialCalendarOperationId::new(),
        calendar_id: JudicialCalendarId::new(),
        change: JudicialCalendarChange::Publish {
            values: values("Calendar"),
        },
    }
}
pub fn empty(command: &JudicialCalendarCommand) -> JudicialCalendarPreparation {
    JudicialCalendarPreparation {
        calendar_id: command.calendar_id,
        base: None,
        initial_scope: None,
    }
}
pub fn preparation(base: &JudicialCalendarDetail) -> JudicialCalendarPreparation {
    JudicialCalendarPreparation {
        calendar_id: base.id,
        base: Some(base.clone()),
        initial_scope: Some(base.values.scope().clone()),
    }
}
pub fn replacement(base: &JudicialCalendarDetail) -> JudicialCalendarCommand {
    JudicialCalendarCommand {
        calendar_id: base.id,
        operation_id: JudicialCalendarOperationId::new(),
        change: JudicialCalendarChange::Replace {
            expected_revision: base.revision,
            values: base.values.clone(),
            reason: JudicialCalendarReason::new("Rechecked sources").unwrap(),
        },
    }
}
pub fn retirement(base: &JudicialCalendarDetail) -> JudicialCalendarCommand {
    JudicialCalendarCommand {
        calendar_id: base.id,
        operation_id: JudicialCalendarOperationId::new(),
        change: JudicialCalendarChange::Retire {
            expected_revision: base.revision,
            reason: JudicialCalendarReason::new("Retired reference").unwrap(),
        },
    }
}
pub fn detail(
    actor: UserId,
    command: &JudicialCalendarCommand,
    values: JudicialCalendarValues,
) -> JudicialCalendarDetail {
    let values_digest = judicial_calendar_values_digest(&Hasher, &values);
    JudicialCalendarDetail {
        id: command.calendar_id,
        revision: command.result_revision().unwrap(),
        status: command.result_status(),
        values,
        values_digest,
        reason: command.reason().cloned(),
        receipt: JudicialCalendarReceipt {
            operation_id: command.operation_id,
            action: command.action(),
            expected_revision: command.expected_revision(),
            submission_digest: judicial_calendar_submission_digest(
                &Hasher,
                actor,
                command,
                values_digest,
            ),
        },
        recorded_at: instant(),
        recorded_by: JudicialCalendarActorSnapshot {
            id: actor,
            email: "owner@example.com".into(),
        },
    }
}
pub fn service(
    store: MockStore,
    identity: MockIdentity,
) -> (JudicialCalendarService, Arc<CountingClock>) {
    let clock = Arc::new(CountingClock::default());
    (
        JudicialCalendarService::new(
            Arc::new(store),
            Arc::new(identity),
            Arc::new(Hasher),
            clock.clone(),
        ),
        clock,
    )
}
