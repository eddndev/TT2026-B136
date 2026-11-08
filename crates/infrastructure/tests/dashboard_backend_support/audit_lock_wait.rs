use super::*;
use crate::audit_lock_holder::{behind_sibling_hold, SIBLING_HOLD};

// The application shell reads the alert inbox while the dashboard loads; the
// loser of the audit lock must wait for the winner instead of failing busy.
#[test]
fn owner_read_waits_for_a_sibling_audited_read_beyond_one_second() {
    let Some(mut db) = fixture() else { return };
    let reader = store(&db, db.at);
    let (read, waited) = behind_sibling_hold(&db.admin_url, || reader.read(db.owner));
    let snapshot = read.unwrap();
    assert!(waited >= SIBLING_HOLD / 2, "{waited:?}");
    assert_eq!(snapshot.scope, DashboardScope::Office);
    assert_eq!(reader.read(db.owner).unwrap(), snapshot);
    assert_eq!(audits(&mut db), 2);
}
