use application::ApplicationError;
use domain::cases::CaseId;
use domain::identity::UserId;
use infrastructure::initialize_database;
use postgres::{Client, NoTls};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct Fixture {
    pub admin: Client,
    pub control: Client,
    pub admin_url: String,
    pub runtime_url: String,
    pub schema: String,
    pub role: String,
    pub owner: UserId,
    pub case: CaseId,
    pub at: OffsetDateTime,
}

impl Fixture {
    pub fn new() -> Option<Self> {
        let base = std::env::var("CASE_TEST_DATABASE_URL").ok()?;
        let mut control = Client::connect(&base, NoTls).unwrap();
        let schema = format!("participant_test_{}", Uuid::new_v4().simple());
        let role = format!("participant_runtime_{}", Uuid::new_v4().simple());
        control.batch_execute(&format!("CREATE SCHEMA {schema}; CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEROLE PASSWORD 'runtime-test-only'")).unwrap();
        let mut url = reqwest::Url::parse(&base).unwrap();
        url.query_pairs_mut()
            .append_pair("options", &format!("-csearch_path={schema}"));
        let admin_url = url.to_string();
        initialize_database(&admin_url, &role).unwrap();
        let mut admin = Client::connect(&admin_url, NoTls).unwrap();
        let owner = UserId::from_uuid(Uuid::new_v4());
        let case = CaseId::new();
        admin.execute("INSERT INTO users(id,email,password_hash,role,protected_totp_secret,recovery_codes) VALUES($1,'owner@example.test','fixture','owner','\\x00','{}')", &[&owner.as_uuid()]).unwrap();
        admin
            .execute(
                "INSERT INTO cases(id,title,reference,created_by,required_initial_revision) VALUES($1,'Participants','P',$2,NULL)",
                &[&case.as_uuid(), &owner.as_uuid()],
            )
            .unwrap();
        url.set_username(&role).unwrap();
        url.set_password(Some("runtime-test-only")).unwrap();
        Some(Self {
            admin,
            control,
            admin_url,
            runtime_url: url.to_string(),
            schema,
            role,
            owner,
            case,
            at: OffsetDateTime::from_unix_timestamp_nanos(1_735_689_600_123_456_789).unwrap(),
        })
    }

    pub fn store(&self) -> infrastructure::PostgresParticipantStore {
        infrastructure::PostgresParticipantStore::open(
            &self.runtime_url,
            std::sync::Arc::new(infrastructure::RingSha256Hasher::new()),
        )
        .unwrap()
    }

    pub fn user(&mut self, role: &str, assigned: bool) -> UserId {
        let id = UserId::from_uuid(Uuid::new_v4());
        self.admin.execute("INSERT INTO users(id,email,password_hash,role,protected_totp_secret,recovery_codes) VALUES($1,$2,'fixture',$3,'\\x00','{}')", &[&id.as_uuid(), &format!("{id}@example.test"), &role]).unwrap();
        if assigned {
            self.admin
                .execute(
                    "INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2)",
                    &[&self.case.as_uuid(), &id.as_uuid()],
                )
                .unwrap();
        }
        id
    }

    pub fn snapshot(&mut self) -> serde_json::Value {
        self.admin.query_one("SELECT jsonb_build_object(
            'roots',(SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM case_participants p),
            'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY participant_id,revision) FROM case_participant_revisions r),
            'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
            'members',(SELECT jsonb_agg(to_jsonb(m) ORDER BY case_id,user_id) FROM case_memberships m),
            'documents',(SELECT jsonb_agg(to_jsonb(d) ORDER BY id,version) FROM documents d),
            'metadata',(SELECT jsonb_agg(to_jsonb(m) ORDER BY document_id,metadata_revision) FROM document_metadata_revisions m),
            'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.control.batch_execute(&format!(
            "DROP SCHEMA {} CASCADE; DROP ROLE {}",
            self.schema, self.role
        ));
    }
}

pub fn configuration_error<T>(result: Result<T, ApplicationError>) -> bool {
    matches!(result, Err(ApplicationError::InvalidConfiguration(_)))
}

pub fn values(name: &str) -> application::participants::ParticipantValues {
    application::participants::ParticipantValues::new(
        name,
        "Witness",
        Some("Office"),
        None,
        application::participants::DirectoryStatus::Active,
    )
    .unwrap()
}
