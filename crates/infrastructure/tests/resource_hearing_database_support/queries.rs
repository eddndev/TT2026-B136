use super::*;
use domain::resource_hearings::{
    ResourceHearingId, ResourceHearingOperationId, ResourceHearingRevision,
};

#[test]
fn pages_keep_exact_initial_creations_after_unlink_and_archive() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    let mut expected = Vec::new();
    for _ in 0..3 {
        let mut command = command.clone();
        command.hearing_id = ResourceHearingId::new();
        command.operation_id = ResourceHearingOperationId::new();
        command.association_id = ResourceActivityId::new();
        expected.push(submit(&db, command));
    }
    expected.sort_by_key(|v| v.origin.hearing_id.as_uuid());
    resource_activity_support::persist(
        &resource_activity_support::service(&db, db.owner, Role::Owner),
        db.case,
        captures.resource.id,
        resource_activity_support::unlink(&expected[0].association, captures.head.revision),
    );
    captures.archive(&db);
    use application::cases::{CaseAdministrativeStatus, CaseRepository, CaseRevisionExpectation};
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let store = store(&db);
    let first = store
        .list(
            db.owner,
            db.case,
            captures.resource.id,
            ResourceHearingReadQuery::new(2, None).unwrap(),
        )
        .unwrap();
    assert_eq!(first.case_id, db.case);
    assert_eq!(first.resource_id, captures.resource.id);
    assert_eq!(first.items, expected[..2]);
    assert!(first.has_more);
    assert_eq!(first.next_after_id, Some(expected[1].origin.hearing_id));
    let last = store
        .list(
            db.owner,
            db.case,
            captures.resource.id,
            ResourceHearingReadQuery::new(2, first.next_after_id).unwrap(),
        )
        .unwrap();
    assert_eq!(last.items, expected[2..]);
    assert!(!last.has_more);
    assert_eq!(last.next_after_id, None);
    let detail = store
        .get(
            db.owner,
            db.case,
            captures.resource.id,
            expected[0].origin.hearing_id,
            Some(ResourceHearingRevision::initial()),
        )
        .unwrap();
    assert_eq!(detail, expected[0]);
    assert_eq!(detail.association.revision.get(), 1);
}

#[test]
fn queries_authorize_before_scope_lookup_and_respect_revocation() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    let expected = submit(&db, command);
    let para = db.user("paralegal", true);
    let client = db.user("client", true);
    let outsider = db.user("litigator", false);
    let store = store(&db);
    let query = ResourceHearingReadQuery::new(10, None).unwrap();
    assert_eq!(
        store
            .get(
                para,
                db.case,
                captures.resource.id,
                expected.origin.hearing_id,
                None
            )
            .unwrap(),
        expected
    );
    assert!(matches!(
        store.list(client, db.case, captures.resource.id, query),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        store.get(
            outsider,
            db.case,
            ResourceId::new(),
            expected.origin.hearing_id,
            None
        ),
        Err(ApplicationError::CaseNotFound)
    ));
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &para.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        store.list(para, db.case, captures.resource.id, query),
        Err(ApplicationError::CaseNotFound)
    ));
}

#[test]
fn missing_parents_or_revisions_are_not_successful_empty_pages() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    let store = store(&db);
    let empty = store
        .list(
            db.owner,
            db.case,
            captures.resource.id,
            ResourceHearingReadQuery::new(10, None).unwrap(),
        )
        .unwrap();
    assert!(empty.items.is_empty());
    assert!(!empty.has_more);
    assert_eq!(empty.next_after_id, None);
    assert!(store
        .list(
            db.owner,
            db.case,
            ResourceId::new(),
            ResourceHearingReadQuery::new(10, None).unwrap()
        )
        .is_err());
    let expected = submit(&db, command);
    assert!(matches!(
        store.get(
            db.owner,
            db.case,
            captures.resource.id,
            expected.origin.hearing_id,
            Some(ResourceHearingRevision::new(2).unwrap())
        ),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::NotFound
        ))
    ));
    let (other, _) = setup(&mut db);
    assert!(matches!(
        store.get(
            db.owner,
            db.case,
            other.resource.id,
            expected.origin.hearing_id,
            None
        ),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::NotFound
        ))
    ));
}

#[test]
fn read_audit_failure_prevents_returning_data_without_mutating_creations() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    let expected = submit(&db, command);
    let store = store(&db);
    db.admin
        .batch_execute(
            "CREATE FUNCTION reject_resource_hearing_read() RETURNS trigger
        LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected read audit failure'; END; $$;
        CREATE TRIGGER reject_resource_hearing_read BEFORE INSERT ON audit_events
        FOR EACH ROW WHEN (NEW.action IN ('resource_hearing.list','resource_hearing.read'))
        EXECUTE FUNCTION reject_resource_hearing_read()",
        )
        .unwrap();
    let before = atomic_rows(&mut db);
    assert!(store
        .list(
            db.owner,
            db.case,
            captures.resource.id,
            ResourceHearingReadQuery::new(10, None).unwrap()
        )
        .is_err());
    assert!(store
        .get(
            db.owner,
            db.case,
            captures.resource.id,
            expected.origin.hearing_id,
            None
        )
        .is_err());
    assert_eq!(atomic_rows(&mut db), before);
    db.admin
        .batch_execute(
            "DROP TRIGGER reject_resource_hearing_read ON audit_events;
        DROP FUNCTION reject_resource_hearing_read()",
        )
        .unwrap();
    assert_eq!(
        store
            .get(
                db.owner,
                db.case,
                captures.resource.id,
                expected.origin.hearing_id,
                None
            )
            .unwrap(),
        expected
    );
}

#[test]
fn query_rejects_lost_origin_instead_of_presenting_a_successful_capture() {
    let Some(mut db) = Fixture::new() else { return };
    let (captures, command) = setup(&mut db);
    let expected = submit(&db, command);
    let store = store(&db);
    db.admin
        .batch_execute(
            "CREATE TEMP TABLE saved_resource_hearing_origin AS
        SELECT * FROM audit_events WHERE action='resource_hearing.registered';
        SET session_replication_role=replica;
        DELETE FROM audit_events WHERE action='resource_hearing.registered';
        SET session_replication_role=origin",
        )
        .unwrap();
    for missing in ["origin", "initial association"] {
        assert!(
            matches!(
                store.get(
                    db.owner,
                    db.case,
                    captures.resource.id,
                    expected.origin.hearing_id,
                    None
                ),
                Err(ApplicationError::ResourceActivity(
                    ResourceActivityError::StoredInconsistent(_)
                ))
            ),
            "missing {missing} must be a stored-integrity failure on get"
        );
        assert!(
            matches!(
                store.list(
                    db.owner,
                    db.case,
                    captures.resource.id,
                    ResourceHearingReadQuery::new(10, None).unwrap()
                ),
                Err(ApplicationError::ResourceActivity(
                    ResourceActivityError::StoredInconsistent(_)
                ))
            ),
            "missing {missing} must be a stored-integrity failure on list"
        );
        if missing == "origin" {
            // Restore the exact audit row before isolating the second corruption.
            db.admin
                .batch_execute(
                    "SET session_replication_role=replica;
                INSERT INTO audit_events (sequence,timestamp,actor,action,resource,chain)
                SELECT sequence,timestamp,actor,action,resource,chain
                FROM saved_resource_hearing_origin;
                DROP TABLE saved_resource_hearing_origin",
                )
                .unwrap();
            db.admin
                .execute(
                    "DELETE FROM case_resource_activity_association_revisions
                WHERE association_id=$1 AND revision=1",
                    &[&expected.origin.association_id.as_uuid()],
                )
                .unwrap();
            db.admin
                .batch_execute("SET session_replication_role=origin")
                .unwrap();
        }
    }
}
