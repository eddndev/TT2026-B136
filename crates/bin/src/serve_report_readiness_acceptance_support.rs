use crate::{
    case_administration_support::Fixture as Database,
    password_reset_composition_support::router_with_identity_reports,
    password_reset_runtime_support::Fixture as RedisKeys,
    serve_identity_composition,
    serve_invitation_acceptance_support::code,
    serve_password_reset_http_acceptance_support::{login, mfa, track_token},
};
use application::{
    case_reports::*,
    cases::CaseRepository,
    identity::{EnrollmentResult, IdentityWorkflow, SessionPolicy},
    ApplicationError,
};
use axum::{http::Method, Router};
use domain::{
    cases::{CaseId, CaseMetadata},
    clock::Clock,
    crypto::DocumentHasher,
};
use infrastructure::{
    case_report_isolation::IsolatedCaseReportRenderer, CaseReportEmailConfiguration,
    CaseReportEnvelopeProtector, EnvelopeKeyManager, PostgresCaseReportStore, RingAesGcmCipher,
    RingSha256Hasher, SystemClock,
};
use serde_json::{json, Value};
use std::{
    fs::File,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};
use tokio::runtime::Runtime;
use uuid::Uuid;
use web::{HttpLimits, HttpWorkBudget};
use zeroize::Zeroizing;

#[path = "serve_report_readiness_http.rs"]
mod http;
pub use http::{download, id, stable, Reply};

pub const PATH: &str = "/api/v1/case-reports";
const PASSWORD: &str = "native report readiness acceptance password";

#[derive(Clone)]
pub struct Delivered {
    pub id: Uuid,
    pub message: CaseReportEmailMessage,
    pub at: OffsetDateTime,
}

#[derive(Default)]
pub struct Capture(Mutex<Vec<Delivered>>);

impl Capture {
    pub fn messages(&self) -> Vec<Delivered> {
        self.0.lock().unwrap().clone()
    }
}

impl CaseReportEmailDelivery for Capture {
    fn deliver(
        &self,
        id: Uuid,
        message: &CaseReportEmailMessage,
    ) -> Result<CaseReportEmailOutcome, ApplicationError> {
        self.0.lock().unwrap().push(Delivered {
            id,
            message: message.clone(),
            at: SystemClock.now(),
        });
        Ok(CaseReportEmailOutcome::Accepted)
    }
}

pub struct Acceptance {
    pub runtime: Runtime,
    pub router: Router,
    identity: Arc<dyn IdentityWorkflow>,
    pub owner: EnrollmentResult,
    pub worker: CaseReportWorker,
    pub email: CaseReportEmailWorker,
    pub capture: Arc<Capture>,
    renderer: Arc<IsolatedCaseReportRenderer>,
    pub renderer_digest: String,
    keys: RedisKeys,
    pub db: Database,
}

impl Acceptance {
    pub fn new() -> Self {
        let executable = PathBuf::from(
            std::env::var_os("TT_REPORT_TIMING_EXECUTABLE")
                .expect("manual report acceptance requires TT_REPORT_TIMING_EXECUTABLE"),
        );
        let renderer = Arc::new(IsolatedCaseReportRenderer::new(executable.clone()).unwrap());
        let renderer_digest = RingSha256Hasher
            .hash_stream(&mut File::open(executable).unwrap())
            .unwrap()
            .to_hex();
        let keys = RedisKeys::new();
        let mut db =
            Database::old().expect("manual report acceptance requires CASE_TEST_DATABASE_URL");
        assert_eq!(
            db.admin
                .execute("DELETE FROM cases WHERE id=$1", &[&db.case.as_uuid()])
                .unwrap(),
            1
        );
        assert_eq!(
            db.admin
                .execute("DELETE FROM users WHERE id=$1", &[&db.owner.as_uuid()])
                .unwrap(),
            1
        );
        db.migrate();
        let identity = serve_identity_composition::open(
            &db.runtime_url,
            &keys.url,
            Zeroizing::new(vec![42; 32]),
            SessionPolicy::new(300, None).unwrap(),
            None,
        )
        .unwrap_or_else(|_| panic!("open real report identity"))
        .identity;
        let owner = identity
            .bootstrap_owner(
                &format!("report-ready-{}@example.test", Uuid::new_v4().simple()),
                PASSWORD,
            )
            .unwrap_or_else(|_| panic!("bootstrap real report owner"));
        let cases = db.store();
        for index in 0..3 {
            cases
                .create_basic(
                    owner.principal.id,
                    CaseId::new(),
                    CaseMetadata::new(
                        &format!("Readiness case {index}"),
                        &format!("READY-{index}"),
                    )
                    .unwrap(),
                    SystemClock.now(),
                )
                .unwrap();
        }
        let store = store(&db.runtime_url);
        let router = router(identity.clone(), store.clone());
        let capture = Arc::new(Capture::default());
        let email = email(store.clone(), capture.clone());
        let worker = CaseReportWorker::new(
            store,
            renderer.clone(),
            Arc::new(RingSha256Hasher),
            Arc::new(SystemClock),
        );
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        Self {
            runtime,
            router,
            identity,
            owner,
            worker,
            email,
            capture,
            renderer,
            renderer_digest,
            keys,
            db,
        }
    }

    pub fn login(&mut self, first: bool) -> Zeroizing<String> {
        let email = &self.owner.principal.email;
        track_token(&mut self.keys, "password-failures", email);
        let challenge = login(&self.runtime, &self.router, &mut self.keys, email, PASSWORD);
        let (kind, proof) = if first {
            let proof = code(&self.owner.totp_secret_base32);
            let mut claim = Zeroizing::new(self.owner.principal.id.as_uuid().as_bytes().to_vec());
            claim.extend_from_slice(proof.as_bytes());
            self.keys.track(format!(
                "identity:totp-used:{}",
                RingSha256Hasher.hash_bytes(&claim).to_hex()
            ));
            ("totp", proof)
        } else {
            ("recovery", self.owner.recovery_codes[0].clone())
        };
        mfa(
            &self.runtime,
            &self.router,
            &mut self.keys,
            kind,
            &challenge,
            &proof,
        )
    }

    pub fn send(&self, method: Method, path: &str, token: &str, body: Option<Value>) -> Reply {
        http::send(&self.runtime, &self.router, method, path, token, body)
    }

    pub fn get(&self, path: &str, token: &str) -> Reply {
        self.send(Method::GET, path, token, None)
    }

    pub fn request(&self, token: &str, command: &Value) -> Reply {
        self.send(Method::POST, PATH, token, Some(command.clone()))
    }

    pub fn reopen(&mut self) {
        let store = store(&self.db.runtime_url);
        self.router = router(self.identity.clone(), store.clone());
        self.email = email(store.clone(), self.capture.clone());
        self.worker = CaseReportWorker::new(
            store,
            self.renderer.clone(),
            Arc::new(RingSha256Hasher),
            Arc::new(SystemClock),
        );
    }
}

fn store(url: &str) -> Arc<PostgresCaseReportStore> {
    let protector = CaseReportEnvelopeProtector::new(
        Arc::new(RingAesGcmCipher::new()),
        Arc::new(EnvelopeKeyManager::new()),
        vec![52; 32],
    )
    .unwrap();
    Arc::new(
        PostgresCaseReportStore::open(
            url,
            Arc::new(RingSha256Hasher),
            Arc::new(SystemClock),
            Arc::new(protector),
        )
        .unwrap(),
    )
}

fn router(identity: Arc<dyn IdentityWorkflow>, store: Arc<PostgresCaseReportStore>) -> Router {
    let reports = Arc::new(CaseReportService::new(
        store,
        identity.clone(),
        Arc::new(RingSha256Hasher),
        Arc::new(SystemClock),
    ));
    let limits = HttpLimits::default();
    router_with_identity_reports(
        identity,
        reports,
        limits,
        HttpWorkBudget::new(limits.max_blocking_operations),
    )
}

pub fn template() -> CaseReportEmailTemplate {
    CaseReportEmailConfiguration::new(
        "reports@example.test".into(),
        "https://qadra.example.test/".into(),
    )
    .unwrap()
    .template()
}

fn email(store: Arc<PostgresCaseReportStore>, capture: Arc<Capture>) -> CaseReportEmailWorker {
    CaseReportEmailWorker::new(store, capture, template(), Arc::new(SystemClock))
}

pub fn command() -> Value {
    let now = SystemClock.now();
    json!({
        "operation_id":Uuid::new_v4(),
        "filters":{
            "created_from":(now-Duration::days(1)).format(&Rfc3339).unwrap(),
            "created_before":(now+Duration::days(1)).format(&Rfc3339).unwrap(),
            "status":"all", "assigned_litigator":null,
        },
    })
}

pub fn instant(value: &Value) -> OffsetDateTime {
    OffsetDateTime::parse(value.as_str().unwrap(), &Rfc3339).unwrap()
}
