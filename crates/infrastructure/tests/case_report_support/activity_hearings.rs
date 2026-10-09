use crate::{
    case_report_support::*, hearing_database_support as hearings,
    hearing_result_database_support as results,
};
use application::hearing_results::*;
use domain::{hearings::HearingId, identity::Role};
use time::Duration;

#[test]
fn activity_counts_original_hearing_results_by_author_and_operation_period() {
    let Some(mut db) = fixture() else { return };
    db.at = time::OffsetDateTime::now_utc() + Duration::hours(1);
    let hearing = appointment(&mut db);
    let author = db.user("litigator", true);
    let other = db.user("litigator", true);
    let from = db.at + Duration::days(1);
    let before = from + Duration::days(2);

    db.at = from - Duration::nanoseconds(1);
    persist(&db, author, results::record(hearing));
    db.at = from;
    let original_command = with_agreements(hearing);
    let original = persist(&db, author, original_command.clone());
    assert_eq!(original.snapshot.recorded_at, from);
    assert_eq!(original.snapshot.recorded_by.id, author);
    assert_eq!(original.snapshot.values.agreements().len(), 3);

    db.at = from + Duration::hours(1);
    let workflow = results::service(&db, author, Role::Litigator);
    assert!(matches!(
        workflow.submit(
            "session",
            db.case,
            original_command,
            original.snapshot.receipt.submission_digest,
        ),
        Err(application::ApplicationError::HearingResult(
            HearingResultError::OperationConflict
        ))
    ));
    assert_eq!(
        workflow
            .get(
                "session",
                db.case,
                hearing,
                original.snapshot.id,
                Some(original.snapshot.revision),
            )
            .unwrap(),
        original
    );
    persist(&db, other, results::record(hearing));

    db.at = from + Duration::hours(2);
    let corrected = persist(
        &db,
        other,
        HearingResultCommand {
            operation_id: HearingResultOperationId::new(),
            hearing_id: hearing,
            result_id: original.snapshot.id,
            change: HearingResultChange::Correct {
                expected_revision: original.snapshot.revision,
                values: results::values("Corrected declared session"),
                reason: HearingResultText::new("Correct the administrative account").unwrap(),
            },
        },
    );
    assert_eq!(corrected.snapshot.recorded_by.id, other);
    db.at = from + Duration::hours(3);
    persist(
        &db,
        other,
        HearingResultCommand {
            operation_id: HearingResultOperationId::new(),
            hearing_id: hearing,
            result_id: original.snapshot.id,
            change: HearingResultChange::Withdraw {
                expected_revision: corrected.snapshot.revision,
                reason: HearingResultText::new("Withdraw the administrative account").unwrap(),
            },
        },
    );
    results::service(&db, author, Role::Litigator)
        .prepare("session", db.case, results::record(hearing))
        .unwrap();

    db.at = before;
    let excluded = persist(&db, author, results::record(hearing));
    assert_eq!(excluded.snapshot.recorded_at, before);
    assert_eq!(count(&mut db, "case_hearing_results"), 4);
    assert_eq!(count(&mut db, "case_hearing_result_revisions"), 6);
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
fn activity_hearing_author_picker_survives_reassignment_and_isolates_cases() {
    let Some(mut db) = fixture() else { return };
    db.at = time::OffsetDateTime::now_utc() + Duration::hours(1);
    let hearing = appointment(&mut db);
    let visible_case = db.case;
    let author = db.user("litigator", true);
    let reader = db.user("litigator", true);
    persist(&db, author, results::record(hearing));
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &author.as_uuid()],
        )
        .unwrap();
    let foreign_hearing = appointment(&mut db);
    let foreign = db.user("litigator", true);
    persist(&db, foreign, results::record(foreign_hearing));

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

fn appointment(db: &mut Fixture) -> HearingId {
    hearings::complete(db);
    hearings::persist(
        &hearings::service(db, db.owner, Role::Owner),
        db.case,
        hearings::schedule(),
    )
    .snapshot
    .id
}

fn persist(
    db: &Fixture,
    author: domain::identity::UserId,
    command: HearingResultCommand,
) -> HearingResultDetail {
    results::persist(
        &results::service(db, author, Role::Litigator),
        db.case,
        command,
    )
}

fn with_agreements(hearing: HearingId) -> HearingResultCommand {
    let mut command = results::record(hearing);
    let HearingResultChange::Record { values, .. } = &mut command.change else {
        unreachable!()
    };
    *values = HearingResultValues::new(HearingResultValuesInput {
        occurrence: values.occurrence(),
        extent: values.extent(),
        event_time: values.event_time(),
        summary: values.summary().clone(),
        attendees: values.attendees().to_vec(),
        agreements: (0..3)
            .map(|index| {
                HearingResultAgreement::new(
                    HearingResultAgreementId::new(),
                    HearingResultText::new(&format!("Declared agreement {index}")).unwrap(),
                )
            })
            .collect(),
        provenance: values.provenance().clone(),
    })
    .unwrap();
    command
}
