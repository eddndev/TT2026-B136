mod materials;
pub use materials::materials;

use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};

use domain::clock::OffsetDateTime;
use postgres::{Client, NoTls};
use uuid::Uuid;

pub const MIGRATION: &str =
    include_str!("../../../../migrations/0009_participant_credential_trust.sql");

pub struct Clock(AtomicI64);
impl Clock {
    pub fn new(at: i64) -> Arc<Self> {
        Arc::new(Self(AtomicI64::new(at)))
    }
    pub fn set(&self, at: i64) {
        self.0.store(at, Ordering::SeqCst);
    }
}
impl domain::clock::Clock for Clock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.0.load(Ordering::SeqCst)).unwrap()
    }
}

pub struct Database {
    pub control: Client,
    pub client: Client,
    pub url: String,
    pub schema: String,
    pub roles: Vec<String>,
}
impl Database {
    pub fn new() -> Option<Self> {
        let base = std::env::var("CASE_TEST_DATABASE_URL").ok()?;
        let mut control = Client::connect(&base, NoTls).unwrap();
        let schema = format!("credential_trust_{}", Uuid::new_v4().simple());
        control
            .batch_execute(&format!("CREATE SCHEMA {schema}"))
            .unwrap();
        let mut url = reqwest::Url::parse(&base).unwrap();
        url.query_pairs_mut()
            .append_pair("options", &format!("-csearch_path={schema}"));
        let url = url.to_string();
        let mut client = Client::connect(&url, NoTls).unwrap();
        for migration in [
            include_str!("../../../../migrations/0001_identity.sql"),
            include_str!("../../../../migrations/0002_cases.sql"),
            include_str!("../../../../migrations/0003_case_documents_audit.sql"),
            MIGRATION,
        ] {
            client.batch_execute(migration).unwrap();
        }
        Some(Self {
            control,
            client,
            url,
            schema,
            roles: vec![],
        })
    }
    pub fn session_user(&mut self) -> String {
        self.client
            .query_one("SELECT SESSION_USER", &[])
            .unwrap()
            .get(0)
    }
    pub fn reapply(&mut self) {
        self.client.batch_execute(MIGRATION).unwrap();
    }
    pub fn counts(&mut self) -> (i64, i64, i64) {
        let row = self.client.query_one("SELECT (SELECT count(*) FROM participant_credential_authority),
            (SELECT count(*) FROM participant_credential_trust_revisions),(SELECT count(*) FROM audit_events)", &[]).unwrap();
        (row.get(0), row.get(1), row.get(2))
    }
    pub fn snapshot(&mut self) -> serde_json::Value {
        self.client.query_one("SELECT jsonb_build_object(
            'authority',(SELECT jsonb_agg(to_jsonb(r) ORDER BY deployment_id) FROM participant_credential_authority r),
            'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY revision) FROM participant_credential_trust_revisions r),
            'audit',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM audit_events r))", &[]).unwrap().get(0)
    }
    pub fn role(&mut self, grants: &str) -> String {
        let role = format!("trust_runtime_{}", Uuid::new_v4().simple());
        self.control
            .batch_execute(&format!(
                "CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEROLE PASSWORD 'runtime-test-only'"
            ))
            .unwrap();
        self.client
            .batch_execute(
                &format!("GRANT USAGE ON SCHEMA {} TO {role}; {grants}", self.schema)
                    .replace("{role}", &role),
            )
            .unwrap();
        self.roles.push(role.clone());
        let mut url = reqwest::Url::parse(&self.url).unwrap();
        url.set_username(&role).unwrap();
        url.set_password(Some("runtime-test-only")).unwrap();
        url.to_string()
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        self.control
            .batch_execute(&format!("DROP SCHEMA {} CASCADE", self.schema))
            .unwrap();
        for role in &self.roles {
            self.control
                .batch_execute(&format!("DROP ROLE {role}"))
                .unwrap();
        }
    }
}
