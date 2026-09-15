#[allow(dead_code)]
mod version_database_support;

use infrastructure::initialize_database;
use postgres::{Client, NoTls};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use uuid::Uuid;
use version_database_support::Database;

#[test]
fn an_insert_waits_for_the_previous_commit_before_reading_the_contiguous_head() {
    for commit_predecessor in [true, false] {
        let Some(mut db) = Database::old_schema() else {
            return;
        };
        let case = db.seed_case();
        let id = Uuid::new_v4();
        db.client
            .execute(
                "INSERT INTO documents VALUES($1,$2,1,'first.txt',$3,$4,NULL)",
                &[&id, &case, &vec![3_u8; 32], &vec![7_u8; 80]],
            )
            .unwrap();
        let role = format!("append_runtime_{}", Uuid::new_v4().simple());
        db.control
            .batch_execute(&format!(
                "CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEROLE"
            ))
            .unwrap();
        initialize_database(&db.url, &role).unwrap();
        let mut url = reqwest::Url::parse(&db.url).unwrap();
        url.set_username(&role).unwrap();
        let mut first = Client::connect(url.as_str(), NoTls).unwrap();
        let mut transaction = first.transaction().unwrap();
        transaction
            .execute(
                "INSERT INTO documents VALUES($1,$2,2,'second.txt',$3,$4,NULL)",
                &[&id, &case, &vec![3_u8; 32], &vec![7_u8; 80]],
            )
            .unwrap();
        let (ready, observed) = mpsc::channel();
        let writer = std::thread::spawn(move || {
            let mut second = Client::connect(url.as_str(), NoTls).unwrap();
            let row = second
                .query_one(
                    "SELECT pg_backend_pid(),MAX(version) FROM documents WHERE id=$1",
                    &[&id],
                )
                .unwrap();
            assert_eq!(row.get::<_, i64>(1), 1);
            ready.send(row.get::<_, i32>(0)).unwrap();
            second.execute(
                "INSERT INTO documents VALUES($1,$2,3,'third.txt',$3,$4,NULL)",
                &[&id, &case, &vec![3_u8; 32], &vec![7_u8; 80]],
            )
        });
        let pid = observed.recv_timeout(Duration::from_secs(3)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let blocked: bool = db.control.query_one("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE pid=$1 AND locktype='advisory' AND NOT granted)",&[&pid]).unwrap().get(0);
            if blocked {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "insert did not reach the sequence lock"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        if commit_predecessor {
            transaction.commit().unwrap();
        } else {
            transaction.rollback().unwrap();
        }
        let result = writer.join().unwrap();
        assert_eq!(result.is_ok(), commit_predecessor, "{result:?}");
        assert_eq!(
            db.client
                .query_one("SELECT COUNT(*) FROM documents WHERE id=$1", &[&id])
                .unwrap()
                .get::<_, i64>(0),
            if commit_predecessor { 3 } else { 1 }
        );
        drop(first);
        db.control
            .batch_execute(&format!(
                "DROP SCHEMA {} CASCADE; DROP ROLE {role}",
                db.schema
            ))
            .unwrap();
    }
}
