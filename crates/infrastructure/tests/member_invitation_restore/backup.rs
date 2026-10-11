use crate::case_administration_support::Fixture;
use postgres::{Client, NoTls};
use std::process::Command;
use uuid::Uuid;

pub struct RestoredDatabase {
    pub admin_url: String,
    pub runtime_url: String,
    control: Client,
    name: String,
}

impl RestoredDatabase {
    pub fn copy(source: &Fixture) -> Self {
        let name = format!("invitation_restore_{}", Uuid::new_v4().simple());
        let mut control = Client::connect(&source.admin_url, NoTls).unwrap();
        control
            .batch_execute(&format!("CREATE DATABASE {name} TEMPLATE template0"))
            .unwrap();
        let target = Self {
            admin_url: database_url(&source.admin_url, &name),
            runtime_url: database_url(&source.runtime_url, &name),
            control,
            name,
        };
        let dump = tempfile::NamedTempFile::new().unwrap();
        let output = database_command("pg_dump", &source.admin_url)
            .args(["--schema", &source.schema, "--format=custom", "--file"])
            .arg(dump.path())
            .output()
            .expect("pg_dump must be available");
        assert!(output.status.success(), "private invitation pg_dump failed");
        let output = database_command("pg_restore", &target.admin_url)
            .args(["--single-transaction", "--exit-on-error"])
            .arg(dump.path())
            .output()
            .expect("pg_restore must be available");
        assert!(
            output.status.success(),
            "private invitation pg_restore failed"
        );
        target
    }
}

impl Drop for RestoredDatabase {
    fn drop(&mut self) {
        let _ = self
            .control
            .batch_execute(&format!("DROP DATABASE {}", self.name));
    }
}

fn database_url(source: &str, name: &str) -> String {
    let mut url = reqwest::Url::parse(source).unwrap();
    url.set_path(name);
    url.to_string()
}

fn database_command(program: &str, connection: &str) -> Command {
    let config: postgres::Config = connection.parse().unwrap();
    let mut url = reqwest::Url::parse(connection).unwrap();
    url.set_password(None).unwrap();
    let parameters: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| key != "password")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.set_query(None);
    url.query_pairs_mut().extend_pairs(parameters);
    let mut command = Command::new(program);
    command
        .arg("--dbname")
        .arg(url.as_str())
        .env_remove("PGPASSWORD");
    if let Some(password) = config.get_password() {
        command.env("PGPASSWORD", std::str::from_utf8(password).unwrap());
    }
    command
}
