use crate::procedural_fact_backend_support::{self as backend, Fixture};
use application::{documents::StageSupportReadLimits, procedural_facts::*, ApplicationError};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    identity::{Role, UserId},
};
use postgres::{Client, NoTls};
use std::sync::{Arc, Mutex};

struct BeforeCommit {
    inner: Arc<dyn ProceduralFactStore>,
    change: Box<dyn Fn(&PreparedFactChange) + Send + Sync>,
}
impl ProceduralFactStore for BeforeCommit {
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        command: &ProceduralFactCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<FactPreparation, ApplicationError> {
        self.inner.prepare(actor, case, command, limits)
    }
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedFactChange,
    ) -> Result<FactDetail, ApplicationError> {
        (self.change)(&prepared);
        self.inner.commit(actor, case, prepared)
    }
    fn list_resolutions(
        &self,
        actor: UserId,
        case: CaseId,
        query: ResolutionQuery,
        at: OffsetDateTime,
    ) -> Result<ResolutionPage, ApplicationError> {
        self.inner.list_resolutions(actor, case, query, at)
    }
    fn list_notifications(
        &self,
        actor: UserId,
        case: CaseId,
        resolution: ResolutionId,
        query: NotificationQuery,
        at: OffsetDateTime,
    ) -> Result<NotificationPage, ApplicationError> {
        self.inner
            .list_notifications(actor, case, resolution, query, at)
    }
    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        target: FactTarget,
        revision: Option<FactRevision>,
        at: OffsetDateTime,
    ) -> Result<FactDetail, ApplicationError> {
        self.inner.get(actor, case, target, revision, at)
    }
    fn history(
        &self,
        actor: UserId,
        case: CaseId,
        target: FactTarget,
        query: FactHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<FactHistoryPage, ApplicationError> {
        self.inner.history(actor, case, target, query, at)
    }
}
pub fn before_commit(
    inner: Arc<dyn ProceduralFactStore>,
    change: impl Fn(&PreparedFactChange) + Send + Sync + 'static,
) -> Arc<dyn ProceduralFactStore> {
    Arc::new(BeforeCommit {
        inner,
        change: Box::new(change),
    })
}
pub type Captured = Arc<Mutex<Option<serde_json::Value>>>;
pub fn watched(
    db: &Fixture,
    actor: UserId,
    role: Role,
    change: impl Fn(&mut Client) + Send + Sync + 'static,
) -> (ProceduralFactService, Captured) {
    let captured = Arc::new(Mutex::new(None));
    let saved = captured.clone();
    let url = db.admin_url.clone();
    let inner = before_commit(backend::store(db), move |_| {
        let mut client = Client::connect(&url, NoTls).unwrap();
        change(&mut client);
        *saved.lock().unwrap() = Some(client.query_one("SELECT jsonb_build_object('roots',(SELECT jsonb_agg(to_jsonb(f) ORDER BY family,id) FROM case_procedural_facts f),'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY family,id,revision) FROM case_procedural_fact_revisions r),'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0));
    });
    (
        backend::service_with_store(db, inner, actor, role),
        captured,
    )
}
pub fn unchanged(db: &mut Fixture, captured: Captured) {
    assert_eq!(
        backend::snapshot(db),
        captured
            .lock()
            .unwrap()
            .clone()
            .expect("commit hook was not reached")
    );
}
pub fn wait_for_audit_lock(db: &mut Fixture) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let waiting: bool = db.admin.query_one("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE usename=$1 AND wait_event='advisory')", &[&db.role]).unwrap().get(0);
        if waiting {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
