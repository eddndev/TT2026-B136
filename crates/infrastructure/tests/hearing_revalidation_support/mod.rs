#![allow(dead_code)]

use super::case_stage_database_support::{processor, FixedClock, FormatCheck, TestIdentity};
use super::hearing_database_support::{store, Fixture};
use application::{
    documents::StageSupportReadLimits, hearings::*, identity::Principal, ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    identity::{Role, UserId},
};
use postgres::{Client, NoTls};
use std::sync::{Arc, Mutex};

struct InterleavedStore {
    inner: Arc<dyn HearingStore>,
    after_prepare: Box<dyn Fn() + Send + Sync>,
}
impl HearingStore for InterleavedStore {
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        command: &HearingCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<HearingPreparation, ApplicationError> {
        let prepared = self.inner.prepare(actor, case, command, limits)?;
        (self.after_prepare)();
        Ok(prepared)
    }
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedHearingChange,
    ) -> Result<HearingDetail, ApplicationError> {
        self.inner.commit(actor, case, prepared)
    }
    fn context(
        &self,
        _: UserId,
        _: CaseId,
        _: OffsetDateTime,
    ) -> Result<HearingCaseContext, ApplicationError> {
        unreachable!()
    }
    fn list(
        &self,
        _: UserId,
        _: CaseId,
        _: HearingQuery,
        _: OffsetDateTime,
    ) -> Result<HearingPage, ApplicationError> {
        unreachable!()
    }
    fn get(
        &self,
        _: UserId,
        _: CaseId,
        _: HearingId,
        _: Option<HearingRevision>,
        _: OffsetDateTime,
    ) -> Result<HearingDetail, ApplicationError> {
        unreachable!()
    }
    fn history(
        &self,
        _: UserId,
        _: CaseId,
        _: HearingId,
        _: HearingHistoryQuery,
        _: OffsetDateTime,
    ) -> Result<HearingHistoryPage, ApplicationError> {
        unreachable!()
    }
    fn agenda(
        &self,
        _: UserId,
        _: HearingAgendaQuery,
        _: OffsetDateTime,
    ) -> Result<HearingAgendaPage, ApplicationError> {
        unreachable!()
    }
}

pub fn hooked(
    db: &Fixture,
    actor: UserId,
    role: Role,
    change: impl Fn() + Send + Sync + 'static,
) -> HearingService {
    HearingService::new(
        Arc::new(InterleavedStore {
            inner: store(db),
            after_prepare: Box::new(change),
        }),
        Arc::new(TestIdentity(Principal {
            id: actor,
            email: "session@example.test".into(),
            role,
        })),
        processor(),
        Arc::new(FormatCheck(None)),
        Arc::new(infrastructure::RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

pub fn snapshot(client: &mut Client) -> serde_json::Value {
    client.query_one("SELECT jsonb_build_object('roots',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM case_hearings h),'revisions',(SELECT jsonb_agg(to_jsonb(h) ORDER BY hearing_id,revision) FROM case_hearing_revisions h),'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
}

pub type Captured = Arc<Mutex<Option<serde_json::Value>>>;
pub fn watched(
    db: &Fixture,
    actor: UserId,
    role: Role,
    change: impl Fn(&mut Client) + Send + Sync + 'static,
) -> (HearingService, Captured) {
    let captured = Arc::new(Mutex::new(None));
    let copy = captured.clone();
    let url = db.admin_url.clone();
    let workflow = hooked(db, actor, role, move || {
        let mut client = Client::connect(&url, NoTls).unwrap();
        change(&mut client);
        *copy.lock().unwrap() = Some(snapshot(&mut client));
    });
    (workflow, captured)
}

pub fn unchanged(db: &mut Fixture, captured: Captured) {
    assert_eq!(
        snapshot(&mut db.admin),
        captured
            .lock()
            .unwrap()
            .clone()
            .expect("preparation hook was not reached")
    );
}

pub fn replacement(base: &HearingDetail) -> HearingCommand {
    HearingCommand {
        operation_id: HearingOperationId::new(),
        hearing_id: base.snapshot.id,
        change: HearingChange::Replace {
            expected_revision: base.snapshot.revision,
            context: HearingContextExpectation {
                case_revision: base.snapshot.scheduling_context.administration_revision,
                stage_revision: base.snapshot.scheduling_context.stage_revision,
            },
            values: base.snapshot.values.clone(),
            reason: HearingNote::new("Communicated replacement").unwrap(),
        },
    }
}

pub fn interleaved(
    inner: Arc<dyn HearingStore>,
    change: impl Fn() + Send + Sync + 'static,
) -> Arc<dyn HearingStore> {
    Arc::new(InterleavedStore {
        inner,
        after_prepare: Box::new(change),
    })
}
