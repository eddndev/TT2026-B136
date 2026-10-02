use super::password_reset_backend_support::*;
use postgres::{Client, NoTls};
use std::time::{Duration, Instant};

struct EmptySchema {
    admin: Client,
    name: String,
}

impl EmptySchema {
    fn before(db: &Fixture) -> Self {
        let name = format!("reset_empty_{}", uuid::Uuid::new_v4().simple());
        let mut admin = Client::connect(&db.admin_url, NoTls).unwrap();
        admin
            .batch_execute(&format!(
                "CREATE SCHEMA {name}; GRANT USAGE ON SCHEMA {name} TO {}",
                db.role,
            ))
            .unwrap();
        Self { admin, name }
    }

    fn runtime_url(&self, db: &Fixture) -> String {
        let mut url = reqwest::Url::parse(&db.runtime_url).unwrap();
        let parameters: Vec<(String, String)> = url
            .query_pairs()
            .filter(|(name, _)| name != "options")
            .map(|(name, value)| (name.into_owned(), value.into_owned()))
            .collect();
        url.set_query(None);
        url.query_pairs_mut().extend_pairs(parameters).append_pair(
            "options",
            &format!("-csearch_path={},{}", self.name, db.schema),
        );
        url.to_string()
    }
}

impl Drop for EmptySchema {
    fn drop(&mut self) {
        let _ = self
            .admin
            .batch_execute(&format!("DROP SCHEMA {} CASCADE", self.name));
    }
}

#[test]
fn reset_adapter_uses_the_validated_relation_schema_with_a_readonly_search_path_prefix() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let prefix = EmptySchema::before(&db);
    let url = prefix.runtime_url(&db);
    let mut runtime = Client::connect(&url, NoTls).unwrap();
    let row = runtime
        .query_one(
            "SELECT current_schema(), n.nspname::text,
         has_schema_privilege(current_user,$1,'USAGE'),
         has_schema_privilege(current_user,$1,'CREATE')
         FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
         WHERE c.oid='password_reset_capabilities'::regclass",
            &[&prefix.name],
        )
        .unwrap();
    assert_eq!(row.get::<_, String>(0), prefix.name);
    assert_eq!(row.get::<_, String>(1), db.schema);
    assert!(row.get::<_, bool>(2));
    assert!(!row.get::<_, bool>(3));

    // A validation rejection is distinct from the suspected wrong invocation schema.
    let repository = PostgresPasswordResetRepository::open(&url)
        .expect("trusted readonly prefix must retain validated runtime access");
    let issued = issue(&repository, target, 1);
    let candidate = repository.inspect(digest(1)).unwrap().unwrap();
    assert_eq!(candidate.id, issued.id);
    assert_eq!(
        repository
            .complete(complete(candidate, 1, "changed-hash"))
            .unwrap(),
        ResetCompletion::Changed
    );
    assert_eq!(reset_events(&mut db), 1);
    assert_chain(&db);
}

fn observe_until(mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if condition() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn expiry_is_rechecked_after_waiting_for_the_user_row_lock() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    let issued = issue(&repository, target, 1);
    let candidate = repository.inspect(digest(1)).unwrap().unwrap();
    let mut holder_connection = Client::connect(&db.admin_url, NoTls).unwrap();
    let holder_pid: i32 = holder_connection
        .query_one("SELECT pg_backend_pid()", &[])
        .unwrap()
        .get(0);
    db.admin
        .execute(
            "UPDATE password_reset_capabilities
         SET expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1",
            &[&issued.id.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    let mut holder = holder_connection.transaction().unwrap();
    // SELECT FOR UPDATE holds only the user row; no member-write trigger or audit lock.
    holder
        .query_one(
            "SELECT id FROM users WHERE id=$1 FOR UPDATE",
            &[&target.as_uuid()],
        )
        .unwrap();
    let role = db.role.clone();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let result = repository.complete(complete(candidate, 1, "row-lock-expired-hash"));
        let _ = sender.send(result);
    });
    let reached_row_lock = observe_until(|| {
        db.admin
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity a
         WHERE a.usename=$1 AND a.wait_event_type='Lock' AND a.wait_event='transactionid'
         AND a.query LIKE '%password_reset_consume%'
         AND $2=ANY(pg_blocking_pids(a.pid)))",
                &[&role, &holder_pid],
            )
            .unwrap()
            .get(0)
    });
    let reached_expiry = reached_row_lock
        && observe_until(|| {
            db.admin.query_one(
        "SELECT clock_timestamp()>=expires_at FROM password_reset_capabilities WHERE id=$1",
        &[&issued.id.as_uuid()],
    ).unwrap().get(0)
        });
    holder.commit().unwrap();
    let result = receiver.recv_timeout(Duration::from_secs(10));
    worker.join().unwrap();
    assert!(
        reached_row_lock,
        "consumer did not wait on the held user row"
    );
    assert!(
        reached_expiry,
        "stored server expiry was not crossed while waiting"
    );
    assert_eq!(result.unwrap().unwrap(), ResetCompletion::Rejected);
    assert_eq!(snapshot(&mut db), before);
}
