use crate::{
    activity_administrative_fixture as administrative, case_report_support::*,
    measure_fixture as measures,
};
use domain::{
    judicial_calendars::CivilDate,
    precautionary_measures::{MeasureDecisionValues, MeasureDecisionValuesInput, MeasureTime},
    procedural_time::DeclaredProceduralTime,
};
use time::Duration;

#[test]
fn activity_counts_judicial_decisions_not_submeasures_or_administrative_corrections() {
    let Some(mut db) = fixture() else { return };
    db.at = time::OffsetDateTime::now_utc() + Duration::hours(1);
    let mut seed = measures::setup(&mut db);
    let author = db.user("litigator", true);
    let other = db.user("litigator", true);
    let author_principal = principal(&mut db, author);
    let other_principal = principal(&mut db, other);
    let from = db.at + Duration::days(1);
    let before = from + Duration::days(2);
    let old = &seed.command.values;
    seed.command.values = MeasureDecisionValues::new(MeasureDecisionValuesInput {
        authority: old.authority().clone(),
        declared_at: MeasureTime::new(
            DeclaredProceduralTime::date(
                CivilDate::from_date((db.at - Duration::days(1)).date()).unwrap(),
                None,
            )
            .unwrap(),
            None,
        )
        .unwrap(),
        justification: old.justification().clone(),
        support: old.support(),
        locator: old.locator().clone(),
    });

    db.at = from - Duration::nanoseconds(1);
    measures::persist(
        &db,
        author_principal.clone(),
        measures::no_change(&seed.command),
    );
    db.at = from;
    seed.command.outcome = measures::impositions(&seed.subject, 3);
    let original = measures::persist(&db, author_principal.clone(), seed.command.clone());
    assert_eq!(original.group.recorded_at, from);
    assert_eq!(original.group.decision.actor.id, author);
    assert_eq!(original.group.measures.len(), 3);

    db.at = from + Duration::hours(1);
    assert_eq!(
        measures::service(&db, author_principal.clone())
            .submit(
                "session",
                db.case,
                seed.command.clone(),
                measures::confirmation(&original.group.review),
            )
            .unwrap(),
        original
    );
    let empty = measures::persist(
        &db,
        other_principal.clone(),
        measures::no_change(&seed.command),
    );
    assert!(empty.group.measures.is_empty());

    db.at = from + Duration::hours(2);
    let selected = &original.group.measures[0];
    administrative::persist(
        &db,
        other_principal,
        administrative::correction(
            administrative::reference(selected),
            seed.command.context,
            &selected.result.values,
            "Corrected terms without a new judicial decision",
        ),
    );
    measures::service(&db, author_principal.clone())
        .prepare("session", db.case, measures::no_change(&seed.command))
        .unwrap();

    db.at = before;
    let excluded = measures::persist(&db, author_principal, measures::no_change(&seed.command));
    assert_eq!(excluded.group.recorded_at, before);
    assert_eq!(count(&mut db, "case_measure_decisions"), 4);
    assert_eq!(count(&mut db, "case_measures"), 3);
    assert_eq!(count(&mut db, "case_measure_administrations"), 1);
    let actor = owner(&mut db);
    let mut command = command(from);
    command.filters.kind = CaseReportKind::LitigatorActivity;
    let snapshot = capture_request(&store(&db, clock(db.at)), &actor, command, db.at);
    let activity = snapshot.activity.unwrap();
    assert_eq!(activity.rows.len(), 2);
    for id in [author, other] {
        let row = activity
            .rows
            .iter()
            .find(|row| row.litigator_id == id)
            .unwrap();
        assert_eq!(row.procedural_activities, 1);
        assert_eq!(row.documents_uploaded, 0);
        assert_eq!(row.deadlines_attended, 0);
    }
}

#[test]
fn activity_decision_author_picker_survives_reassignment_and_isolates_cases() {
    let Some(mut db) = fixture() else { return };
    db.at = time::OffsetDateTime::now_utc() + Duration::hours(1);
    let seed = measures::setup(&mut db);
    let visible_case = db.case;
    let author = db.user("litigator", true);
    let reader = db.user("litigator", true);
    let author_principal = principal(&mut db, author);
    measures::persist(&db, author_principal, measures::no_change(&seed.command));
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &author.as_uuid()],
        )
        .unwrap();
    let foreign_seed = measures::setup(&mut db);
    let foreign = db.user("litigator", true);
    let foreign_principal = principal(&mut db, foreign);
    measures::persist(
        &db,
        foreign_principal,
        measures::no_change(&foreign_seed.command),
    );

    let actor = principal(&mut db, reader);
    let store = store(&db, clock(db.at));
    let mut query = CaseReportLitigatorQuery {
        kind: CaseReportKind::CaseState,
        limit: 100,
        after_id: None,
    };
    let legacy = store.litigators(&actor, query, db.at).unwrap();
    assert!(!legacy.litigators.iter().any(|who| who.user_id == author));
    query.kind = CaseReportKind::LitigatorActivity;
    let current = store.litigators(&actor, query, db.at).unwrap();
    assert!(current.litigators.iter().any(|who| who.user_id == author));
    assert!(!current.litigators.iter().any(|who| who.user_id == foreign));
    let mut command = command(db.at);
    command.filters.kind = CaseReportKind::LitigatorActivity;
    command.filters.litigator = Some(author);
    let snapshot = capture_request(&store, &actor, command.clone(), db.at);
    assert_eq!(snapshot.cases.len(), 1);
    let activity = snapshot.activity.unwrap();
    assert_eq!(activity.rows.len(), 1);
    assert_eq!(activity.rows[0].case_id, visible_case);
    assert_eq!(activity.rows[0].litigator_id, author);
    assert_eq!(activity.rows[0].procedural_activities, 1);
    command.operation_id = CaseReportOperationId::new();
    command.filters.litigator = Some(foreign);
    assert!(matches!(
        request(&store, &actor, command, db.at),
        Err(application::ApplicationError::InvalidInput(_))
    ));
}
