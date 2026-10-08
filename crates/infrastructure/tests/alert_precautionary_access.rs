use super::*;
use application::ApplicationError;

#[test]
fn precautionary_alerts_require_active_staff_membership_and_revoke_saved_access() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    simple(&mut db, due);
    let litigator = db.user("litigator", true);
    let paralegal = db.user("paralegal", true);
    let global_owner = db.user("owner", false);
    let outside = db.user("litigator", false);
    let client = db.user("client", true);
    let inactive = db.user("paralegal", true);
    db.admin.execute("UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1", &[&inactive.as_uuid()]).unwrap();
    let alerts = store(&db, Arc::new(MutableClock::new(due - Duration::hours(12))));
    drive(&alerts);
    for user in [db.owner, litigator, paralegal] {
        assert_eq!(own_alerts(&alerts, user).len(), 1);
    }
    for user in [global_owner, outside] {
        assert!(own_alerts(&alerts, user).is_empty());
    }
    assert!(matches!(
        alerts.list(client, query(20)),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(alerts.list(inactive, query(20)).is_err());
    let id = own_alerts(&alerts, litigator)[0].id;
    db.store()
        .remove_member(db.case, litigator, db.owner, db.at)
        .unwrap();
    assert!(own_alerts(&alerts, litigator).is_empty());
    assert!(matches!(
        alerts.get(litigator, id),
        Err(ApplicationError::Alert(AlertError::NotFound))
    ));
    assert!(matches!(
        alerts.mark_read(
            litigator,
            AlertReadCommand {
                operation_id: operation(),
                alert_id: id
            }
        ),
        Err(ApplicationError::Alert(AlertError::NotFound))
    ));
}
