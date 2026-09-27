use crate::resource_deadline_database_support::*;
use application::resource_deadlines::ResourceDeadlineWorkflow;
use domain::identity::Role;
use std::{
    sync::{Arc, Barrier},
    thread,
};
#[test]
fn concurrent_identical_submissions_return_one_pair_of_receipts() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    let first = service(&db, db.owner, Role::Owner);
    let second = service(&db, db.owner, Role::Owner);
    let digest = first
        .prepare("session", db.case, captures.resource.id, command.clone())
        .unwrap()
        .submission_digest;
    let barrier = Arc::new(Barrier::new(2));
    let a = barrier.clone();
    let case = db.case;
    let resource = captures.resource.id;
    let copy = command.clone();
    let worker = thread::spawn(move || {
        a.wait();
        first.submit("session", case, resource, copy, digest)
    });
    barrier.wait();
    let b = second
        .submit("session", case, resource, command, digest)
        .unwrap();
    assert_eq!(worker.join().unwrap().unwrap(), b);
    let counts=db.admin.query_one("SELECT (SELECT count(*) FROM case_deadline_revisions),(SELECT count(*) FROM case_resource_activity_association_revisions),(SELECT count(*) FROM audit_events WHERE action='deadline.registered'),(SELECT count(*) FROM audit_events WHERE action='resource_activity.link')",&[]).unwrap();
    for index in 0..4 {
        assert_eq!(counts.get::<_, i64>(index), 1);
    }
}
