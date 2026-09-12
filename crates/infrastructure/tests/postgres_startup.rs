use std::env;
use std::sync::{Arc, Barrier};
use std::thread;

use application::cases::{CaseAccess, CaseRepository};
use application::identity::UserRepository;
use infrastructure::{PostgresCaseRepository, PostgresUserRepository};
use postgres::{Client, NoTls};
use uuid::Uuid;

fn scoped_url(url: &str, schema: &str) -> String {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        let separator = if url.contains('?') { '&' } else { '?' };
        format!("{url}{separator}options=-csearch_path%3D{schema}%20-clock_timeout%3D500ms")
    } else {
        format!("{url} options='-c search_path={schema} -c lock_timeout=500ms'")
    }
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
        PostgresCaseRepository::connect(&case_url).map(|_| ())
    });
    vec![users.join().unwrap(), cases.join().unwrap()]
}

#[test]
fn user_and_case_adapters_share_the_migration_lock_and_initialize_a_fresh_schema() {
    let Ok(url) = env::var("CASE_TEST_DATABASE_URL") else {
        return;
    };
    let schema = format!("startup_{}", Uuid::new_v4().simple());
    let scoped = scoped_url(&url, &schema);
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
    let blocked = initialize_together(&scoped);
    transaction.commit().unwrap();
    let initialized = initialize_together(&scoped);
    let users_empty =
        PostgresUserRepository::connect(&scoped).and_then(|repository| repository.has_users());
    let cases_empty = PostgresCaseRepository::connect(&scoped)
        .and_then(|repository| repository.list(CaseAccess::All, 10, 0));
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

    assert!(blocked.iter().all(Result::is_err), "{blocked:?}");
    assert!(initialized.iter().all(Result::is_ok), "{initialized:?}");
    assert!(!users_empty.unwrap());
    assert!(cases_empty.unwrap().is_empty());
    assert_eq!(tables, 3);
}
