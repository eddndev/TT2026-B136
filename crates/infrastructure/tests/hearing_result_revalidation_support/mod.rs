#![allow(dead_code)]
use super::case_stage_database_support::{processor, FixedClock, FormatCheck, TestIdentity};
use super::hearing_result_database_support::{store, Fixture};
use application::{
    documents::StageSupportReadLimits, hearing_results::*, identity::Principal, ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    hearings::HearingId,
    identity::{Role, UserId},
};
use postgres::{Client, NoTls};
use std::sync::{Arc, Mutex};

struct InterleavedStore {
    inner: Arc<dyn HearingResultStore>,
    after_prepare: Box<dyn Fn() + Send + Sync>,
}
impl HearingResultStore for InterleavedStore {
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        command: &HearingResultCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<HearingResultPreparation, ApplicationError> {
        let prepared = self.inner.prepare(actor, case, command, limits)?;
        (self.after_prepare)();
        Ok(prepared)
    }
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedHearingResultChange,
    ) -> Result<HearingResultDetail, ApplicationError> {
        self.inner.commit(actor, case, prepared)
    }
    fn list(
        &self,
        actor: UserId,
        case: CaseId,
        hearing: HearingId,
        query: HearingResultQuery,
        at: OffsetDateTime,
    ) -> Result<HearingResultPage, ApplicationError> {
        self.inner.list(actor, case, hearing, query, at)
    }
    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        hearing: HearingId,
        id: HearingResultId,
        revision: Option<HearingResultRevision>,
        at: OffsetDateTime,
    ) -> Result<HearingResultDetail, ApplicationError> {
        self.inner.get(actor, case, hearing, id, revision, at)
    }
    fn history(
        &self,
        actor: UserId,
        case: CaseId,
        hearing: HearingId,
        id: HearingResultId,
        query: HearingResultHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<HearingResultHistoryPage, ApplicationError> {
        self.inner.history(actor, case, hearing, id, query, at)
    }
}

pub fn interleaved(
    inner: Arc<dyn HearingResultStore>,
    change: impl Fn() + Send + Sync + 'static,
) -> Arc<dyn HearingResultStore> {
    Arc::new(InterleavedStore {
        inner,
        after_prepare: Box::new(change),
    })
}
pub fn hooked(
    db: &Fixture,
    actor: UserId,
    role: Role,
    change: impl Fn() + Send + Sync + 'static,
) -> HearingResultService {
    HearingResultService::new(
        interleaved(store(db), change),
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
    client.query_one("SELECT jsonb_build_object(
        'roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM case_hearing_results r),
        'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY result_id,revision) FROM case_hearing_result_revisions r),
        'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
}
pub type Captured = Arc<Mutex<Option<serde_json::Value>>>;
pub fn watched(
    db: &Fixture,
    actor: UserId,
    role: Role,
    change: impl Fn(&mut Client) + Send + Sync + 'static,
) -> (HearingResultService, Captured) {
    let captured = Arc::new(Mutex::new(None));
    let copy = captured.clone();
    let url = db.admin_url.clone();
    let service = hooked(db, actor, role, move || {
        let mut client = Client::connect(&url, NoTls).unwrap();
        change(&mut client);
        *copy.lock().unwrap() = Some(snapshot(&mut client));
    });
    (service, captured)
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
pub fn appointment(db: &mut Fixture) -> application::hearings::HearingDetail {
    super::hearing_database_support::complete(db);
    super::hearing_database_support::persist(
        &super::hearing_database_support::service(db, db.owner, Role::Owner),
        db.case,
        super::hearing_database_support::schedule(),
    )
}
pub fn set_values(command: &mut HearingResultCommand, value: HearingResultValues) {
    match &mut command.change {
        HearingResultChange::Record { values, .. }
        | HearingResultChange::Correct { values, .. } => *values = value,
        HearingResultChange::Withdraw { .. } => panic!("withdrawal derives historical values"),
    }
}
pub fn input(values: &HearingResultValues) -> HearingResultValuesInput {
    HearingResultValuesInput {
        occurrence: values.occurrence(),
        extent: values.extent(),
        event_time: values.event_time(),
        summary: values.summary().clone(),
        attendees: values.attendees().to_vec(),
        agreements: values.agreements().to_vec(),
        provenance: values.provenance().clone(),
    }
}
