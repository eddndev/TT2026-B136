use super::password_reset_backend_support::*;
use domain::audit::AuditLog;
pub use infrastructure::identity::{
    invalidate_restored_password_resets, PasswordResetRestoreHead, PasswordResetRestoreRequest,
    PasswordResetRestoreResult,
};
use infrastructure::PostgresAuditLog;
use std::path::Path;
use std::process::Command;

pub const RESTORE_ACTION: &str = "identity.password_reset_restore_invalidated";

pub fn restore_request(db: &mut Fixture, operation_id: uuid::Uuid) -> PasswordResetRestoreRequest {
    let events = PostgresAuditLog::open(&db.runtime_url)
        .unwrap()
        .load_all()
        .unwrap();
    PasswordResetRestoreRequest {
        operation_id,
        expected_database: db
            .admin
            .query_one("SELECT current_database()", &[])
            .unwrap()
            .get(0),
        expected_schema: db.schema.clone(),
        expected_head: events.last().map(|last| PasswordResetRestoreHead {
            sequence: last.event.sequence,
            chain: last.chain,
        }),
    }
}

pub fn repeat_request(request: &PasswordResetRestoreRequest) -> PasswordResetRestoreRequest {
    PasswordResetRestoreRequest {
        operation_id: request.operation_id,
        expected_database: request.expected_database.clone(),
        expected_schema: request.expected_schema.clone(),
        expected_head: request
            .expected_head
            .as_ref()
            .map(|head| PasswordResetRestoreHead {
                sequence: head.sequence,
                chain: head.chain,
            }),
    }
}

pub fn assert_restore_receipt(
    db: &mut Fixture,
    id: uuid::Uuid,
    result: &PasswordResetRestoreResult,
) {
    let events = PostgresAuditLog::open(&db.runtime_url)
        .unwrap()
        .load_all()
        .unwrap();
    let receipt = &events[usize::try_from(result.audit_sequence).unwrap()];
    assert_eq!(receipt.event.actor, "database-restore");
    assert_eq!(receipt.event.action, RESTORE_ACTION);
    assert_eq!(
        receipt.event.resource,
        format!(
            "reset-capabilities:restore:{id}:cancelled:{}",
            result.invalidated,
        )
    );
    assert_eq!(receipt.chain, result.audit_head);
    let count: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action=$1",
            &[&RESTORE_ACTION],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
    assert_chain(db);
}

pub fn assert_private_unchanged(db: &mut Fixture, before: &serde_json::Value) {
    assert!(
        snapshot(db) == *before,
        "private database state changed unexpectedly"
    );
}

pub struct Inventory {
    pub candidates: Vec<(u8, ResetCandidate)>,
    pub live: UserId,
}

pub fn populate(db: &mut Fixture, repository: &PostgresPasswordResetRepository) -> Inventory {
    let mut candidates = Vec::new();
    let mut live = None;
    for value in 1..=6 {
        let user = account(db, "litigator", true);
        let issued = issue(repository, user, value);
        let candidate = repository.inspect(digest(value)).unwrap().unwrap();
        match value {
            1 => live = Some(user),
            2 => expire(db, issued.id),
            3 => {
                db.admin
                    .execute(
                        "UPDATE users SET role='paralegal',revision=revision+1,
                    auth_generation=auth_generation+1 WHERE id=$1",
                        &[&user.as_uuid()],
                    )
                    .unwrap();
            }
            4 => {
                db.admin
                    .execute(
                        "UPDATE users SET active=false,revision=revision+1,
                    auth_generation=auth_generation+1 WHERE id=$1",
                        &[&user.as_uuid()],
                    )
                    .unwrap();
            }
            5 => repository
                .cancel_undelivered(issued.id, digest(value))
                .unwrap(),
            6 => assert_eq!(
                repository
                    .complete(complete(
                        candidate.clone(),
                        value,
                        "confirmed-before-backup"
                    ))
                    .unwrap(),
                ResetCompletion::Changed
            ),
            _ => unreachable!(),
        }
        candidates.push((value, candidate));
    }
    Inventory {
        candidates,
        live: live.unwrap(),
    }
}

pub fn dump_restore(db: &mut Fixture) {
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("reset-capabilities.dump");
    private_dump_file(&dump);
    let output = database_command("pg_dump", &db.admin_url)
        .arg("--schema")
        .arg(&db.schema)
        .arg("--format=custom")
        .arg("--file")
        .arg(&dump)
        .output()
        .expect("pg_dump must be available");
    assert!(output.status.success(), "private fixture pg_dump failed");
    db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    let output = database_command("pg_restore", &db.admin_url)
        .arg("--exit-on-error")
        .arg(&dump)
        .output()
        .expect("pg_restore must be available");
    assert!(output.status.success(), "private fixture pg_restore failed");
}

fn private_dump_file(path: &Path) {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).unwrap();
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

pub fn operation_url(db: &Fixture, application: &str, prefix: Option<&str>) -> String {
    let mut url = reqwest::Url::parse(&db.admin_url).unwrap();
    let parameters: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| key != "options" && key != "application_name")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    let path = prefix.map_or_else(
        || db.schema.clone(),
        |first| format!("{first},{}", db.schema),
    );
    url.set_query(None);
    url.query_pairs_mut()
        .extend_pairs(parameters)
        .append_pair(
            "options",
            &format!("-csearch_path={path} -cstatement_timeout=2500"),
        )
        .append_pair("application_name", application);
    // PostgreSQL URI options need percent-encoded spaces, not form-encoded '+'.
    let query = url.query().unwrap().replace('+', "%20");
    url.set_query(Some(&query));
    url.to_string()
}
