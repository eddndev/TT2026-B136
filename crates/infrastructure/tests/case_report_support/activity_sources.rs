use crate::{
    case_report_support::*, deadline_backend_support as deadlines,
    procedural_fact_backend_support as facts,
};
use application::{case_reports::*, deadlines::*, procedural_facts::*};
use domain::identity::Role;
use time::Duration;

#[test]
fn activity_uses_original_facts_and_attention_transitions_not_corrections() {
    let Some(mut db) = fixture() else { return };
    let created = db.at - Duration::days(20);
    db.case = seed_case(&mut db, "Recorded activity", created);
    let author = db.user("litigator", true);
    let facts_service = facts::service(&db, author, Role::Litigator);
    let resolution = facts::persist(&facts_service, db.case, facts::record());
    let _notification = facts::persist(
        &facts_service,
        db.case,
        facts::notify(facts::resolution_ref(&resolution)),
    );
    let _corrected = facts::persist(&facts_service, db.case, facts::correct(&resolution));
    facts_service
        .prepare("session", db.case, facts::record())
        .unwrap();
    let profile = deadlines::profile(&db);
    let source = deadlines::source(&db);
    let owner_deadlines = deadlines::service(&db, db.owner, Role::Owner);
    let deadline = deadlines::persist(
        &owner_deadlines,
        db.case,
        deadlines::human(
            deadlines::command(&db, &profile, &source),
            Some(deadlines::FOLLOW_RESOLUTION),
        ),
    );
    let service = deadlines::service(&db, author, Role::Litigator);
    let attention_command = deadlines::human(deadlines::attention(&deadline), None);
    let prepared = service
        .prepare("session", db.case, attention_command.clone())
        .unwrap();
    let first = service
        .submit(
            "session",
            db.case,
            attention_command.clone(),
            prepared.submission_digest,
        )
        .unwrap();
    assert!(service
        .submit(
            "session",
            db.case,
            attention_command,
            prepared.submission_digest
        )
        .is_err());
    let mut edit = deadlines::attention(&first);
    if let DeadlineChange::SetAttention {
        attention: DeadlineAttention::Recorded { statement, .. },
        ..
    } = &mut edit.change
    {
        *statement =
            deadlines::text("Correct the filing description without marking a new deadline");
    }
    deadlines::persist(&service, db.case, deadlines::human(edit, None));
    let owner = owner(&mut db);
    let mut command = command(db.at);
    command.filters.kind = CaseReportKind::LitigatorActivity;
    command.filters.litigator = Some(author);
    let snapshot = capture_request(&store(&db, clock(db.at)), &owner, command, db.at);
    let activity = snapshot.activity.unwrap();
    assert_eq!(activity.rows.len(), 1);
    assert_eq!(activity.rows[0].litigator_id, author);
    assert_eq!(activity.rows[0].procedural_activities, 2);
    assert_eq!(activity.rows[0].deadlines_attended, 1);
    assert_eq!(activity.rows[0].documents_uploaded, 0);
}
