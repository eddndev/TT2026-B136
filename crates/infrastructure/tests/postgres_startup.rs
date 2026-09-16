use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use application::identity::UserRepository;
use infrastructure::{PostgresCaseRepository, PostgresUserRepository};
use postgres::{Client, NoTls};
use uuid::Uuid;

fn scoped_url(url: &str, schema: &str, lock_timeout: Option<&str>) -> String {
    let mut options = format!("-csearch_path={schema} -capplication_name={schema}");
    if let Some(timeout) = lock_timeout {
        options.push_str(&format!(" -clock_timeout={timeout}"));
    }
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        let separator = if url.contains('?') { '&' } else { '?' };
        let encoded = options.replace(' ', "%20").replace('=', "%3D");
        format!("{url}{separator}options={encoded}")
    } else {
        format!("{url} options='{options}'")
    }
}

fn wait_until_both_adapters_are_blocked(client: &mut Client, application: &str) -> bool {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        let waiting: i64 = client
            .query_one(
                "SELECT count(*) FROM pg_stat_activity WHERE application_name=$1
             AND wait_event_type='Lock' AND wait_event='advisory'",
                &[&application],
            )
            .unwrap()
            .get(0);
        if waiting == 2 {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

fn initialize_together(url: &str) -> Vec<Result<(), application::ApplicationError>> {
    let barrier = Arc::new(Barrier::new(2));
    let user_url = url.to_owned();
    let user_barrier = barrier.clone();
    let users = thread::spawn(move || {
        user_barrier.wait();
        PostgresUserRepository::connect(&user_url).map(|_| ())
    });
    let case_url = url.to_owned();
    let cases = thread::spawn(move || {
        barrier.wait();
        PostgresCaseRepository::connect(
            &case_url,
            std::sync::Arc::new(infrastructure::RingSha256Hasher),
        )
        .map(|_| ())
    });
    vec![users.join().unwrap(), cases.join().unwrap()]
}

#[test]
fn user_and_case_adapters_share_the_migration_lock_and_initialize_a_fresh_schema() {
    let Ok(url) = env::var("CASE_TEST_DATABASE_URL") else {
        return;
    };
    let schema = format!("startup_{}", Uuid::new_v4().simple());
    let scoped = scoped_url(&url, &schema, None);
    let blocked_url = scoped_url(&url, &schema, Some("500ms"));
    let mut control = Client::connect(&url, NoTls).unwrap();
    control
        .batch_execute(&format!("CREATE SCHEMA {schema}"))
        .unwrap();
    let mut transaction = control.transaction().unwrap();
    transaction
        .query_one("SELECT pg_advisory_xact_lock($1)", &[&0x4341534553_i64])
        .unwrap();

    // lock_timeout makes both connection attempts complete while this lock
    // remains held, so success proves an adapter bypassed schema coordination.
    let blocked = initialize_together(&blocked_url);
    let successful_url = scoped.clone();
    let initialization = thread::spawn(move || initialize_together(&successful_url));
    let mut observer = Client::connect(&url, NoTls).unwrap();
    let waiting = wait_until_both_adapters_are_blocked(&mut observer, &schema);
    // Successful initialization must tolerate a migration lock held beyond the
    // short timeout used only to prove that both adapters honor coordination.
    thread::sleep(Duration::from_millis(750));
    transaction.commit().unwrap();
    let initialized = initialization.join().unwrap();
    let users_empty =
        PostgresUserRepository::connect(&scoped).and_then(|repository| repository.has_users());
    let cases_empty: i64 = Client::connect(&scoped, NoTls)
        .unwrap()
        .query_one("SELECT COUNT(*) FROM cases", &[])
        .unwrap()
        .get(0);
    let tables: i64 = control
        .query_one(
            "SELECT COUNT(*) FROM information_schema.tables
             WHERE table_schema = $1 AND table_name IN ('users', 'cases', 'case_memberships')",
            &[&schema],
        )
        .unwrap()
        .get(0);
    control
        .batch_execute(&format!("DROP SCHEMA {schema} CASCADE"))
        .unwrap();

    assert!(
        waiting,
        "both initialization clients must wait for the migration lock"
    );
    assert!(blocked.iter().all(Result::is_err), "{blocked:?}");
    assert!(initialized.iter().all(Result::is_ok), "{initialized:?}");
    assert!(!users_empty.unwrap());
    assert_eq!(cases_empty, 0);
    assert_eq!(tables, 3);
}
