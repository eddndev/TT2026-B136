use super::*;
use application::documents::{CaseDocumentStore, DocumentRecord};
use domain::crypto::{DocumentId, DocumentVersion, Sha256Digest};
use infrastructure::PostgresCaseDocumentStore;

fn activity_command(at: OffsetDateTime) -> CaseReportCommand {
    let mut command = command(at);
    command.filters.kind = CaseReportKind::LitigatorActivity;
    command
}
fn upload(db: &Fixture, actor: UserId, at: OffsetDateTime) {
    let document = DocumentRecord::pending(
        DocumentId::new(),
        DocumentVersion::initial(),
        "activity-evidence.txt".into(),
        Sha256Digest::from_array([3; 32]),
        vec![8; 80],
    )
    .unwrap();
    PostgresCaseDocumentStore::open(&db.runtime_url)
        .unwrap()
        .insert(actor, db.case, document, at)
        .unwrap();
}

#[test]
fn activity_counts_upload_author_after_reassignment_and_uses_event_period() {
    let Some(mut db) = fixture() else { return };
    let at = db.at + Duration::days(3);
    let created = db.at - Duration::days(1);
    db.case = seed_case(&mut db, "Earlier case", created);
    let author = db.user("litigator", true);
    let replacement = db.user("litigator", true);
    upload(&db, author, at - Duration::nanoseconds(1));
    upload(&db, author, at);
    upload(&db, author, at + Duration::hours(1));
    upload(&db, author, at + Duration::days(2));
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &author.as_uuid()],
        )
        .unwrap();
    let actor = principal(&mut db, replacement);
    let now = at + Duration::days(3);
    let store = store(&db, clock(now));
    let snapshot = capture_request(&store, &actor, activity_command(at), now);
    assert_eq!(snapshot.cases.len(), 1);
    assert!(snapshot.cases[0].created_at < snapshot.filters.period_from);
    let activity = snapshot.activity.unwrap();
    assert!(activity.documents_complete);
    assert_eq!(activity.rows.len(), 1);
    assert_eq!(activity.rows[0].litigator_id, author);
    assert_eq!(activity.rows[0].documents_uploaded, 2);
    assert_eq!(activity.rows[0].procedural_activities, 0);
    assert_eq!(activity.rows[0].deadlines_attended, 0);
    assert!(activity.actors.iter().any(|who| who.user_id == author));
}

#[test]
fn activity_report_replay_preserves_type_capture_and_later_revocation() {
    let Some(mut db) = fixture() else { return };
    let at = db.at + Duration::days(1);
    let created = db.at - Duration::days(1);
    db.case = seed_case(&mut db, "Earlier case", created);
    let author = db.user("litigator", true);
    upload(&db, author, at);
    let actor = principal(&mut db, author);
    let store = store(&db, clock(at));
    let command = activity_command(at);
    let report = request(&store, &actor, command.clone(), at).unwrap();
    assert_eq!(
        request(&store, &actor, command.clone(), at).unwrap(),
        report
    );
    let mut changed = command;
    changed.filters.kind = CaseReportKind::CaseState;
    assert!(matches!(
        request(&store, &actor, changed, at),
        Err(ApplicationError::CaseReport(
            CaseReportError::OperationConflict
        ))
    ));
    let lease = store.claim_next(at).unwrap().unwrap().lease;
    let snapshot = store.capture(&lease, at).unwrap();
    assert_eq!(store.capture(&lease, at).unwrap(), snapshot);
    assert_eq!(
        snapshot.activity.as_ref().unwrap().rows[0].documents_uploaded,
        1
    );
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &author.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        store.capture(&lease, at),
        Err(ApplicationError::CaseReport(CaseReportError::AccessRevoked))
    ));
}

#[test]
fn activity_author_picker_survives_reassignment_without_exposing_foreign_authors() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    db.case = seed_case(&mut db, "Visible contribution", at);
    let author = db.user("litigator", true);
    let reader = db.user("litigator", true);
    upload(&db, author, at);
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &author.as_uuid()],
        )
        .unwrap();
    db.case = seed_case(&mut db, "Foreign contribution", at);
    let foreign = db.user("litigator", true);
    upload(&db, foreign, at);
    let actor = principal(&mut db, reader);
    let store = store(&db, clock(at));
    let mut query = CaseReportLitigatorQuery {
        kind: CaseReportKind::CaseState,
        limit: 100,
        after_id: None,
    };
    let legacy = store.litigators(&actor, query, at).unwrap();
    assert!(!legacy.litigators.iter().any(|who| who.user_id == author));
    query.kind = CaseReportKind::LitigatorActivity;
    let current = store.litigators(&actor, query, at).unwrap();
    assert!(current.litigators.iter().any(|who| who.user_id == author));
    assert!(!current.litigators.iter().any(|who| who.user_id == foreign));
    let mut command = activity_command(at);
    command.filters.litigator = Some(author);
    let snapshot = capture_request(&store, &actor, command, at);
    assert_eq!(snapshot.activity.unwrap().rows[0].documents_uploaded, 1);
}
