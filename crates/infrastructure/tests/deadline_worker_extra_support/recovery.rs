use crate::deadline_backend_support as dl;
use domain::clock::{Clock, OffsetDateTime};
use infrastructure::{PostgresDeadlineWorkerStore, RingSha256Hasher};
use postgres::{Client, NoTls};
use std::{
    process::Command,
    sync::{Arc, Mutex},
};

pub struct ControlledClock(Mutex<OffsetDateTime>);
impl ControlledClock {
    pub fn set(&self, at: OffsetDateTime) {
        *self.0.lock().unwrap() = at;
    }
}
impl Clock for ControlledClock {
    fn now(&self) -> OffsetDateTime {
        *self.0.lock().unwrap()
    }
}
pub fn controlled(db: &dl::Fixture) -> (PostgresDeadlineWorkerStore, Arc<ControlledClock>) {
    let clock = Arc::new(ControlledClock(Mutex::new(db.at)));
    let worker = PostgresDeadlineWorkerStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        clock.clone(),
    )
    .unwrap();
    (worker, clock)
}

pub fn restore(db: &mut dl::Fixture) {
    let constraints = worker_checks(db);
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("deadline-worker.dump");
    run(Command::new("pg_dump")
        .args([
            "--dbname",
            &db.admin_url,
            "--schema",
            &db.schema,
            "--format=custom",
            "--file",
        ])
        .arg(&dump));
    let mut restore_url = reqwest::Url::parse(&db.admin_url).unwrap();
    let remaining = restore_url
        .query_pairs()
        .filter(|(name, _)| name != "options")
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    restore_url
        .query_pairs_mut()
        .clear()
        .extend_pairs(remaining)
        .append_pair("options", "-csearch_path=");
    let mut probe = Client::connect(restore_url.as_str(), NoTls).unwrap();
    let search_path: String = probe.query_one("SHOW search_path", &[]).unwrap().get(0);
    assert_eq!(search_path, "");
    drop(probe);
    db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    run(Command::new("pg_restore")
        .args(["--exit-on-error", "--dbname", restore_url.as_str()])
        .arg(&dump));
    let restored = worker_checks(db);
    assert_eq!(restored.len(), constraints.len());
    for (before, after) in constraints.iter().zip(&restored) {
        assert_eq!(
            after, before,
            "restored CHECK must retain its exact expression"
        );
    }
}
fn worker_checks(db: &mut dl::Fixture) -> Vec<(String, String)> {
    db.admin
        .query(
            "SELECT conname::text,pg_catalog.pg_get_expr(conbin,conrelid)
        FROM pg_catalog.pg_constraint WHERE contype='c'
            AND conrelid IN ('deadline_reevaluation_results'::regclass,
                'deadline_reevaluation_attempts'::regclass) ORDER BY conname",
            &[],
        )
        .unwrap()
        .into_iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect()
}
fn run(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
