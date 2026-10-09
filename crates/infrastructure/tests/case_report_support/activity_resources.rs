use crate::{case_report_support::*, procedural_resource_support as resources};
use application::{case_reports::*, procedural_resources::*};
use domain::{
    identity::{Role, UserId},
    judicial_calendars::CivilDate,
    procedural_time::DeclaredProceduralTime,
};
use time::Duration;

#[test]
fn activity_counts_original_resource_acts_by_author_and_operation_period() {
    let Some(mut db) = fixture() else { return };
    let from = db.at + Duration::days(1);
    let before = from + Duration::days(2);
    let created = db.at - Duration::days(20);
    db.case = seed_case(&mut db, "Resource activity", created);
    let author = db.user("litigator", true);
    let other = db.user("litigator", true);
    let declaration =
        DeclaredProceduralTime::date(CivilDate::from_date(created.date()).unwrap(), None).unwrap();
    let registration = resources::registration(&db);
    let mut head = persist(&db, author, registration.clone());

    db.at = from - Duration::nanoseconds(1);
    head = persist(
        &db,
        author,
        act(&head, ResourceActKind::Interposition, declaration),
    );
    assert_eq!(head.recorded_at, db.at);

    db.at = from;
    let mut another_registration = registration;
    another_registration.operation_id = ResourceOperationId::new();
    another_registration.resource_id = ResourceId::new();
    let registered = persist(&db, author, another_registration);
    assert_eq!(registered.receipt.action, ResourceAction::Register);

    let mut originals = Vec::new();
    for (index, kind) in [
        ResourceActKind::Interposition,
        ResourceActKind::Admission,
        ResourceActKind::Inadmissibility,
        ResourceActKind::Withdrawal,
        ResourceActKind::Resolution,
    ]
    .into_iter()
    .enumerate()
    {
        db.at = from + Duration::hours(i64::try_from(index).unwrap());
        let command = act(&head, kind, declaration);
        head = persist(&db, author, command.clone());
        assert_eq!(head.recorded_at, db.at);
        assert_eq!(head.recorded_by.id, author);
        assert_eq!(head.act.as_ref().unwrap().values.kind(), kind);
        originals.push((command, head.clone()));
    }
    db.at = from + Duration::hours(5);
    head = persist(
        &db,
        other,
        act(&head, ResourceActKind::Admission, declaration),
    );

    db.at = from + Duration::hours(6);
    let original = originals[0].1.act.as_ref().unwrap();
    head = persist(
        &db,
        other,
        ResourceCommand {
            operation_id: ResourceOperationId::new(),
            resource_id: head.id,
            change: ResourceChange::CorrectAct {
                expected_revision: head.revision,
                act_id: original.id,
                expected_act_revision: original.revision,
                values: original.values.clone(),
                reason: resources::text("Correct the original declared act"),
            },
        },
    );
    assert_eq!(head.receipt.action, ResourceAction::CorrectAct);
    assert_eq!(head.recorded_by.id, other);
    assert_eq!(head.act.as_ref().unwrap().revision.get(), 2);

    db.at = from + Duration::hours(7);
    let service = resources::service(&db, author, Role::Litigator);
    service
        .prepare(
            "session",
            db.case,
            act(&head, ResourceActKind::Resolution, declaration),
        )
        .unwrap();
    let (replayed_command, saved) = &originals[0];
    assert_eq!(
        service
            .submit(
                "session",
                db.case,
                replayed_command.clone(),
                saved.receipt.submission_digest
            )
            .unwrap(),
        *saved
    );

    db.at = before;
    let excluded = persist(
        &db,
        author,
        act(&head, ResourceActKind::Resolution, declaration),
    );
    assert_eq!(excluded.recorded_at, before);
    let owner = owner(&mut db);
    let mut query = command(from);
    query.filters.kind = CaseReportKind::LitigatorActivity;
    let snapshot = capture_request(&store(&db, clock(db.at)), &owner, query, db.at);
    let activity = snapshot.activity.unwrap();
    assert_eq!(activity.rows.len(), 2);
    for (id, count) in [(author, 5), (other, 1)] {
        let row = activity
            .rows
            .iter()
            .find(|row| row.litigator_id == id)
            .unwrap();
        assert_eq!(row.procedural_activities, count);
        assert_eq!(row.documents_uploaded, 0);
        assert_eq!(row.deadlines_attended, 0);
    }
}

fn persist(db: &Fixture, author: UserId, command: ResourceCommand) -> ResourceDetail {
    resources::persist(
        &resources::service(db, author, Role::Litigator),
        db.case,
        command,
    )
}

fn act(
    base: &ResourceDetail,
    kind: ResourceActKind,
    occurred_at: DeclaredProceduralTime,
) -> ResourceCommand {
    let mut command = resources::act(base);
    let ResourceChange::RecordAct { values, .. } = &mut command.change else {
        unreachable!()
    };
    *values = ResourceActValues::new(ResourceActValuesInput {
        kind,
        mode: values.mode().clone(),
        occurred_at,
        authority: values.authority().clone(),
        statement: values.statement().clone(),
        evidence: values.evidence().to_vec(),
    })
    .unwrap();
    command
}
