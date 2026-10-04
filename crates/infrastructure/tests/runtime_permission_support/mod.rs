use postgres::{Client, NoTls};

pub struct Fixture {
    pub url: String,
    pub schema: String,
    control: Client,
    schemas: Vec<String>,
    roles: Vec<String>,
}

pub struct Resources {
    schemas: Vec<String>,
    roles: Vec<String>,
}

impl Fixture {
    pub fn new() -> Option<Self> {
        let base = crate::document_store_support::database_url()?;
        let control = Client::connect(&base, NoTls).unwrap();
        let schema = format!("runtime_permission_{}", uuid::Uuid::new_v4().simple());
        let mut url = reqwest::Url::parse(&without_options(&base)).unwrap();
        url.query_pairs_mut()
            .append_pair("options", &format!("-csearch_path={schema}"));
        let mut fixture = Self {
            url: url.to_string(),
            schema: schema.clone(),
            control,
            schemas: vec![schema.clone()],
            roles: Vec::new(),
        };
        fixture
            .control
            .batch_execute(&format!("CREATE SCHEMA {schema}"))
            .unwrap();
        Some(fixture)
    }

    pub fn create_role(&mut self, prefix: &str, login: bool) -> String {
        let role = format!("{prefix}_{}", uuid::Uuid::new_v4().simple());
        let access = if login {
            "LOGIN PASSWORD 'runtime-test-only'"
        } else {
            "NOLOGIN"
        };
        self.roles.push(role.clone());
        self.control
            .batch_execute(&format!(
                "CREATE ROLE {role} {access} NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS"
            ))
            .unwrap();
        role
    }

    pub fn create_schema(&mut self, prefix: &str, owner: &str) -> String {
        assert!(self.roles.iter().any(|role| role == owner));
        let schema = format!("{prefix}_{}", uuid::Uuid::new_v4().simple());
        self.schemas.push(schema.clone());
        self.control
            .batch_execute(&format!("CREATE SCHEMA {schema} AUTHORIZATION {owner}"))
            .unwrap();
        schema
    }

    pub fn resources(&self) -> Resources {
        Resources {
            schemas: self.schemas.clone(),
            roles: self.roles.clone(),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for schema in self.schemas.iter().rev() {
            if let Err(error) = self
                .control
                .batch_execute(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
            {
                eprintln!("failed to remove owned permission schema {schema}: {error}");
            }
        }
        for role in self.roles.iter().rev() {
            if let Err(error) = self
                .control
                .batch_execute(&format!("DROP OWNED BY {role}; DROP ROLE IF EXISTS {role}"))
            {
                eprintln!("failed to remove owned permission role {role}: {error}");
            }
        }
    }
}

impl Resources {
    pub fn assert_absent(&self, client: &mut Client) {
        for schema in &self.schemas {
            assert!(
                !client
                    .query_one(
                        "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_namespace WHERE nspname=$1)",
                        &[schema],
                    )
                    .unwrap()
                    .get::<_, bool>(0),
                "owned schema survived cleanup: {schema}"
            );
        }
        for role in &self.roles {
            assert!(
                !client
                    .query_one(
                        "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_roles WHERE rolname=$1)",
                        &[role],
                    )
                    .unwrap()
                    .get::<_, bool>(0),
                "owned role survived cleanup: {role}"
            );
        }
    }
}

pub fn without_options(value: &str) -> String {
    let mut url = reqwest::Url::parse(value).unwrap();
    let parameters: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(name, _)| name != "options")
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();
    url.query_pairs_mut().clear().extend_pairs(parameters);
    url.to_string()
}
