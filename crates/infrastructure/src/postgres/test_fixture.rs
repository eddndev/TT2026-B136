//! Disposable migrated schema for in-crate catalog validation tests.
//!
//! A fixture creates a uniquely named schema and runtime role in the database
//! named by `CASE_TEST_DATABASE_URL`, applies every migration and runtime grant
//! through `initialize_database`, and drops both when it is dropped. Tests
//! return early when the variable is absent, like the integration fixtures.
//!
//! `mutated` alters the catalog inside a transaction on the administrative
//! connection and rolls it back afterwards, so one fixture serves many catalog
//! states. Inside that transaction the connection acts as the runtime role, and
//! `Mutated::run` isolates each check in a savepoint: a check that fails with a
//! database error does not abort the transaction for the next check.

use postgres::{Client, NoTls};
use uuid::Uuid;

const RUNTIME_PASSWORD: &str = "runtime-test-only";

pub(crate) struct CatalogFixture {
    admin: Client,
    control: Client,
    pub(crate) runtime_url: String,
    pub(crate) schema: String,
    pub(crate) role: String,
}

impl CatalogFixture {
    pub(crate) fn new() -> Option<Self> {
        let base = std::env::var("CASE_TEST_DATABASE_URL").ok()?;
        let mut control = Client::connect(&base, NoTls).unwrap();
        let name = format!("catalog_check_{}", Uuid::new_v4().simple());
        control
            .batch_execute(&format!(
                "CREATE SCHEMA {name};
                CREATE ROLE {name} LOGIN NOSUPERUSER NOCREATEROLE PASSWORD '{RUNTIME_PASSWORD}'"
            ))
            .unwrap();
        let mut url = reqwest::Url::parse(&base).unwrap();
        url.query_pairs_mut()
            .append_pair("options", &format!("-csearch_path={name}"));
        let admin_url = url.to_string();
        super::initialize_database(&admin_url, &name).unwrap();
        let admin = Client::connect(&admin_url, NoTls).unwrap();
        url.set_username(&name).unwrap();
        url.set_password(Some(RUNTIME_PASSWORD)).unwrap();
        Some(Self {
            admin,
            control,
            runtime_url: url.to_string(),
            schema: name.clone(),
            role: name,
        })
    }

    /// Name of a role that a mutation may create; it disappears with the
    /// rollback of that mutation.
    pub(crate) fn helper(&self) -> String {
        format!("{}_helper", self.role)
    }

    /// Replaces `{runtime}`, `{helper}` and `{schema}` in a mutation template.
    pub(crate) fn render(&self, template: &str) -> String {
        template
            .replace("{runtime}", &self.role)
            .replace("{helper}", &self.helper())
            .replace("{schema}", &self.schema)
    }

    /// Applies a rendered mutation as the administrator in a transaction that
    /// is rolled back when the returned guard is dropped, then switches the
    /// connection to the runtime role for the checks.
    pub(crate) fn mutated(&mut self, template: &str) -> Result<Mutated<'_>, postgres::Error> {
        let mutation = self.render(template);
        self.admin.batch_execute("BEGIN")?;
        let state = Mutated {
            client: &mut self.admin,
        };
        state.client.batch_execute(&mutation)?;
        state
            .client
            .batch_execute(&format!("SET LOCAL ROLE {}", self.role))?;
        Ok(state)
    }
}

impl Drop for CatalogFixture {
    fn drop(&mut self) {
        let _ = self.control.batch_execute(&format!(
            "DROP SCHEMA {} CASCADE; DROP ROLE {}",
            self.schema, self.role
        ));
    }
}

/// A mutated catalog visible only to the administrative connection.
pub(crate) struct Mutated<'a> {
    client: &'a mut Client,
}

impl Mutated<'_> {
    /// Runs one check in a savepoint and discards whatever it changed.
    pub(crate) fn run<T>(&mut self, check: impl FnOnce(&mut Client) -> T) -> T {
        self.client
            .batch_execute("SAVEPOINT catalog_check")
            .unwrap();
        let result = check(self.client);
        self.client
            .batch_execute("ROLLBACK TO SAVEPOINT catalog_check")
            .unwrap();
        result
    }
}

impl Drop for Mutated<'_> {
    fn drop(&mut self) {
        let _ = self.client.batch_execute("ROLLBACK");
    }
}
