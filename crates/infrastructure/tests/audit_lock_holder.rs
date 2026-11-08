//! Holds the database-wide audit lock the way a sibling audited read does.

use postgres::{Client, NoTls};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const AUDIT_LOCK: i64 = 0x4155444954;

/// Longest sibling hold measured on the debug CI server: an Owner dashboard
/// read over 15 active cases kept the lock for about 2.3 s, so a concurrent
/// inbox or dashboard read must wait longer than the former 1 s budget.
pub const SIBLING_HOLD: Duration = Duration::from_millis(2500);

/// Runs `read` while another session holds the audit lock for `SIBLING_HOLD`
/// and returns its result with the time it took.
pub fn behind_sibling_hold<T>(url: &str, read: impl FnOnce() -> T) -> (T, Duration) {
    let mut holder = Client::connect(url, NoTls).unwrap();
    let (held, acquired) = mpsc::sync_channel(1);
    thread::scope(|scope| {
        let hold = scope.spawn(move || {
            let mut tx = holder.transaction().unwrap();
            tx.query_one("SELECT pg_advisory_xact_lock($1)", &[&AUDIT_LOCK])
                .unwrap();
            held.send(()).unwrap();
            thread::sleep(SIBLING_HOLD);
            tx.commit().unwrap();
        });
        acquired
            .recv_timeout(Duration::from_secs(10))
            .expect("the sibling must acquire the audit lock");
        let started = Instant::now();
        let result = read();
        let elapsed = started.elapsed();
        hold.join().unwrap();
        (result, elapsed)
    })
}
