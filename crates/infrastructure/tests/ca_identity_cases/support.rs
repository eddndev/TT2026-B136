use application::identity::UserRecord;
use application::ApplicationError;
use domain::crypto::{RecoveryCodeSet, RECOVERY_CODE_COUNT};
use domain::identity::{Role, UserId};
use infrastructure::{
    with_validated_ca_identity_postgres, PostgresAuditLog, PostgresUserRepository,
};
use postgres::{Client, NoTls};
use reqwest::Url;

pub struct Fixture {
    pub database: String,
    pub role: String,
    pub backup_role: String,
    pub runtime: String,
    pub backup: String,
    pub operator: UserRecord,
    admin: Option<Client>,
    control: Client,
    roles: Vec<String>,
}

impl Fixture {
    pub fn new() -> Self {
        let raw = std::env::var("IDENTITY_TEST_DATABASE_URL")
            .expect("CA identity tests require disposable IDENTITY_TEST_DATABASE_URL");
        let mut base =
            Url::parse(&raw).expect("IDENTITY_TEST_DATABASE_URL must be a PostgreSQL URL");
        assert!(matches!(base.scheme(), "postgres" | "postgresql"));
        assert!(matches!(
            base.host_str(),
            Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
        ));
        base.set_query(None);
        let mut control = Client::connect(base.as_str(), NoTls).unwrap();
        let database = format!("ca_identity_{}", uuid::Uuid::new_v4().simple());
        let role = format!("ca_runtime_{}", uuid::Uuid::new_v4().simple());
        let backup_role = format!("ca_backup_{}", uuid::Uuid::new_v4().simple());
        let password = uuid::Uuid::new_v4().simple().to_string();
        control
            .batch_execute(&format!("CREATE DATABASE {database}"))
            .unwrap();
        base.set_path(&format!("/{database}"));
        let mut admin = Client::connect(base.as_str(), NoTls).unwrap();
        admin
            .batch_execute("SET statement_timeout='5s'; SET lock_timeout='2s'")
            .unwrap();
        let mut fixture = Self {
            database,
            role: role.clone(),
            backup_role: backup_role.clone(),
            runtime: String::new(),
            backup: String::new(),
            operator: operator(),
            admin: Some(admin),
            control,
            roles: Vec::new(),
        };
        for name in [&role, &backup_role] {
            fixture.roles.push(name.clone());
            fixture
                .control
                .batch_execute(&format!(
                    "CREATE ROLE {name} LOGIN NOINHERIT NOSUPERUSER NOCREATEDB
                 NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{password}'"
                ))
                .unwrap();
        }
        fixture
            .admin()
            .batch_execute(include_str!("../../src/ca_identity/schema.sql"))
            .unwrap();
        fixture
            .admin()
            .batch_execute(
                "REVOKE ALL ON SCHEMA public FROM PUBLIC;
             CREATE TABLE public.office_sentinel (email TEXT PRIMARY KEY, marker TEXT NOT NULL);
             INSERT INTO public.office_sentinel VALUES ('operator@example.test', 'office-only');",
            )
            .unwrap();
        fixture
            .admin()
            .batch_execute(&format!(
                "GRANT USAGE ON SCHEMA qadra_ca TO {role}, {backup_role};
             GRANT SELECT ON qadra_ca.users, qadra_ca.audit_events TO {role}, {backup_role};
             GRANT INSERT ON qadra_ca.audit_events TO {role};
             GRANT UPDATE(recovery_codes, revision, updated_at) ON qadra_ca.users TO {role};
             GRANT USAGE ON SCHEMA public TO {backup_role};
             GRANT SELECT ON public.office_sentinel TO {backup_role};"
            ))
            .unwrap();
        fixture.runtime = credential_url(&base, &role, &password);
        fixture.backup = credential_url(&base, &backup_role, &password);
        fixture.seed_operator();
        fixture
    }

    pub fn admin(&mut self) -> &mut Client {
        self.admin.as_mut().unwrap()
    }

    pub fn seed_operator(&mut self) {
        let user = self.operator.clone();
        let codes = serde_json::to_value(&user.recovery_codes).unwrap();
        self.admin()
            .execute(
                "INSERT INTO qadra_ca.users
             (id,email,password_hash,role,active,protected_totp_secret,recovery_codes)
             VALUES ($1,$2,$3,'client',true,$4,$5)",
                &[
                    &user.id.as_uuid(),
                    &user.email,
                    &user.password_hash,
                    &user.protected_totp_secret,
                    &codes,
                ],
            )
            .unwrap();
    }

    pub fn open(&self) -> Result<(PostgresUserRepository, PostgresAuditLog), ApplicationError> {
        with_validated_ca_identity_postgres(&self.runtime, &self.database, &self.role, |source| {
            Ok((
                PostgresUserRepository::open(source)?,
                PostgresAuditLog::open(source)?,
            ))
        })
    }

    pub fn assert_rejected(&self) {
        assert!(
            self.open().is_err(),
            "unsafe CA runtime target was admitted"
        );
    }

    pub fn connect_runtime(&self) -> Client {
        Client::connect(&self.runtime, NoTls).unwrap()
    }

    pub fn new_role(&mut self) -> String {
        let name = format!("ca_extra_{}", uuid::Uuid::new_v4().simple());
        self.roles.push(name.clone());
        self.control
            .batch_execute(&format!("CREATE ROLE {name} NOLOGIN"))
            .unwrap();
        name
    }

    pub fn assert_office_unchanged(&mut self) {
        let marker: String = self
            .admin()
            .query_one(
                "SELECT marker FROM public.office_sentinel WHERE email='operator@example.test'",
                &[],
            )
            .unwrap()
            .get(0);
        assert_eq!(marker, "office-only");
        assert!(!self
            .admin()
            .query_one("SELECT to_regclass('public.users') IS NOT NULL", &[],)
            .unwrap()
            .get::<_, bool>(0));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.admin.take();
        let dropped = self.control.batch_execute(&format!(
            "DROP DATABASE IF EXISTS {} WITH (FORCE)",
            self.database
        ));
        if dropped.is_err() {
            eprintln!("owned CA test database cleanup failed");
        }
        for role in self.roles.iter().rev() {
            if self
                .control
                .batch_execute(&format!("DROP ROLE IF EXISTS {role}"))
                .is_err()
            {
                eprintln!("owned CA test role cleanup failed");
            }
        }
    }
}

pub fn search_path(url: &str, path: &str) -> String {
    let mut url = Url::parse(url).unwrap();
    url.set_query(None);
    url.query_pairs_mut()
        .append_pair("options", &format!("-csearch_path={path}"));
    url.to_string()
}

fn credential_url(base: &Url, role: &str, password: &str) -> String {
    let mut url = base.clone();
    url.set_username(role).unwrap();
    url.set_password(Some(password)).unwrap();
    search_path(url.as_str(), "qadra_ca,pg_temp")
}

fn operator() -> UserRecord {
    UserRecord {
        id: UserId::new(),
        email: "operator@example.test".into(),
        password_hash: "$argon2id$synthetic-credential".into(),
        role: Role::Client,
        active: true,
        protected_totp_secret: vec![1; 32],
        recovery_codes: RecoveryCodeSet::from_hashes(
            (0..RECOVERY_CODE_COUNT)
                .map(|index| format!("synthetic-{index}"))
                .collect(),
        )
        .unwrap(),
        revision: 0,
        auth_generation: 0,
    }
}
