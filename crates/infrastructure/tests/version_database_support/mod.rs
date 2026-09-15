use postgres::{Client, NoTls};
use uuid::Uuid;

pub struct Database {
    pub url: String,
    pub client: Client,
    pub control: Client,
    pub schema: String,
}

impl Database {
    pub fn old_schema() -> Option<Self> {
        let base = std::env::var("DOCUMENT_TEST_DATABASE_URL").ok()?;
        let schema = format!("version_migration_{}", Uuid::new_v4().simple());
        let mut control = Client::connect(&base, NoTls).unwrap();
        control
            .batch_execute(&format!("CREATE SCHEMA {schema}"))
            .unwrap();
        let mut parsed = reqwest::Url::parse(&base).unwrap();
        parsed
            .query_pairs_mut()
            .append_pair("options", &format!("-csearch_path={schema}"));
        let url = parsed.to_string();
        let mut client = Client::connect(&url, NoTls).unwrap();
        client
            .batch_execute(include_str!("../../../../migrations/0001_identity.sql"))
            .unwrap();
        client
            .batch_execute(include_str!("../../../../migrations/0002_cases.sql"))
            .unwrap();
        client
            .batch_execute(include_str!(
                "../../../../migrations/0003_case_documents_audit.sql"
            ))
            .unwrap();
        Some(Self {
            url,
            client,
            control,
            schema,
        })
    }

    pub fn seed_case(&mut self) -> Uuid {
        let user = Uuid::new_v4();
        let case = Uuid::new_v4();
        self.client.execute("INSERT INTO users(id,email,password_hash,role,protected_totp_secret,recovery_codes) VALUES($1,$2,'fixture','owner','\\x00','{}')", &[&user, &format!("{user}@example.test")]).unwrap();
        let revised: bool = self.client.query_one("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_attribute WHERE attrelid='cases'::regclass AND attname='required_initial_revision' AND NOT attisdropped)",&[]).unwrap().get(0);
        let insert = if revised {
            "INSERT INTO cases(id,title,reference,created_by,required_initial_revision) VALUES($1,'Versions','V',$2,NULL)"
        } else {
            "INSERT INTO cases(id,title,reference,created_by) VALUES($1,'Versions','V',$2)"
        };
        self.client.execute(insert, &[&case, &user]).unwrap();
        case
    }

    pub fn snapshot(&mut self) -> serde_json::Value {
        self.client.query_one("SELECT jsonb_build_object('documents',(SELECT jsonb_agg(to_jsonb(d) ORDER BY id,version) FROM documents d),'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let _ = self
            .control
            .batch_execute(&format!("DROP SCHEMA {} CASCADE", self.schema));
    }
}
