use super::*;
use application::cases::{CaseAdministrativeStatus, CaseRevisionExpectation};
use application::typed_participants::ParticipantRevisionSnapshot;
use domain::hearings::HearingParticipantRef;
use uuid::Uuid;

#[test]
fn thirty_two_imposed_rows_and_exact_typed_supervision_survive_reopening() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let ParticipantRevisionSnapshot::Typed(supervisor) = &seed.supervisor.revision else {
        panic!()
    };
    let mut inputs = values(&seed.subject);
    inputs = MeasureValues::new(MeasureValuesInput {
        subject: inputs.subject(),
        kind: inputs.kind(),
        conditions: inputs.conditions().clone(),
        validity: inputs.validity().clone(),
        supervision: MeasureSupervision::Known {
            participant: HearingParticipantRef::new(supervisor.id, supervisor.revision),
            statement: note("Declared exact supervisor"),
        },
    });
    seed.command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(
        (0..32)
            .map(|_| {
                MeasureEffect::Impose(MeasureProposal {
                    id: MeasureId::new(),
                    values: inputs.clone(),
                })
            })
            .collect(),
    ))
    .unwrap();
    let original = persist(&db, seed.actor.clone(), seed.command.clone());
    assert_eq!(original.group.measures.len(), 32);
    for row in &original.group.measures {
        assert_eq!(
            row.result.sources.supervisor.as_ref(),
            Some(&seed.supervisor)
        );
        assert_eq!(row.result.sources.subject, seed.subject);
    }
    assert_eq!(
        reads(&db, seed.actor)
            .get("session", db.case, seed.command.decision_id)
            .unwrap(),
        original
    );
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_measure_revisions", &[])
            .unwrap()
            .get::<_, i64>(0),
        32
    );
}

#[test]
fn no_change_has_a_real_decision_origin_and_audit_without_measure_rows() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let command = no_change(&seed.command);
    let original = persist(&db, seed.actor.clone(), command.clone());
    assert!(original.group.measures.is_empty());
    assert!(original.group.review.results.is_empty());
    assert!(original.group.substitutions.is_empty());
    assert!(original.measure_history.groups.is_empty());
    assert_eq!(original.origin.decision_id, command.decision_id);
    assert_eq!(original.origin.group_digest, original.group.capture_digest);
    for table in ["case_measure_operations", "case_measure_decisions"] {
        assert_eq!(
            db.admin
                .query_one(&format!("SELECT count(*) FROM {table}"), &[])
                .unwrap()
                .get::<_, i64>(0),
            1
        );
    }
    for table in ["case_measures", "case_measure_revisions"] {
        assert_eq!(
            db.admin
                .query_one(&format!("SELECT count(*) FROM {table}"), &[])
                .unwrap()
                .get::<_, i64>(0),
            0
        );
    }
    assert_eq!(
        db.admin
            .query_one(
                "SELECT count(*) FROM audit_events WHERE action='measure_decision.recorded'",
                &[]
            )
            .unwrap()
            .get::<_, i64>(0),
        1
    );
    assert_eq!(
        reads(&db, seed.actor.clone())
            .get_operation("session", db.case, command.operation_id)
            .unwrap(),
        original
    );
    let replay = service_with_format(
        &db,
        seed.actor,
        FormatCheck(Some(Box::new(|| panic!("replay must skip admission")))),
    );
    assert_eq!(
        replay
            .submit(
                "session",
                db.case,
                command,
                confirmation(&original.group.review)
            )
            .unwrap(),
        original
    );
}

#[test]
fn reopened_reads_and_original_replay_preserve_the_complete_group() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let original = persist(&db, seed.actor.clone(), seed.command.clone());
    let reopened = reads(&db, seed.actor.clone());
    assert_eq!(
        reopened
            .get("session", db.case, seed.command.decision_id)
            .unwrap(),
        original
    );
    assert_eq!(
        reopened
            .get_operation("session", db.case, seed.command.operation_id)
            .unwrap(),
        original
    );
    let replay = service_with_format(
        &db,
        seed.actor,
        FormatCheck(Some(Box::new(|| panic!("replay must skip admission")))),
    );
    assert_eq!(
        replay
            .prepare("session", db.case, seed.command.clone())
            .unwrap(),
        original.group.review
    );
    assert_eq!(
        replay
            .submit(
                "session",
                db.case,
                seed.command,
                confirmation(&original.group.review)
            )
            .unwrap(),
        original
    );
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_measure_decisions", &[])
            .unwrap()
            .get::<_, i64>(0),
        1
    );
    assert_eq!(
        db.admin
            .query_one(
                "SELECT count(*) FROM audit_events WHERE action='measure_decision.recorded'",
                &[]
            )
            .unwrap()
            .get::<_, i64>(0),
        1
    );
}

#[test]
fn pagination_orders_immutable_decisions_and_includes_zero_row_groups() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let mut expected = Vec::new();
    for id in [30, 10, 20] {
        let mut command = no_change(&seed.command);
        command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(id));
        expected.push(persist(&db, seed.actor.clone(), command));
    }
    expected.sort_by_key(|r| r.origin.decision_id.as_uuid());
    let query = reads(&db, seed.actor);
    let first = query
        .list(
            "session",
            db.case,
            MeasureDecisionReadQuery::new(2, None).unwrap(),
        )
        .unwrap();
    assert_eq!(first.items, expected[..2]);
    assert!(first.has_more);
    assert_eq!(first.next_after_id, Some(expected[1].origin.decision_id));
    let last = query
        .list(
            "session",
            db.case,
            MeasureDecisionReadQuery::new(2, first.next_after_id).unwrap(),
        )
        .unwrap();
    assert_eq!(last.items, expected[2..]);
    assert!(!last.has_more);
    assert_eq!(last.next_after_id, None);
    let empty = query
        .list(
            "session",
            db.case,
            MeasureDecisionReadQuery::new(2, Some(expected[2].origin.decision_id)).unwrap(),
        )
        .unwrap();
    assert!(empty.items.is_empty());
    assert!(!empty.has_more);
}

#[test]
fn closed_case_replay_keeps_original_role_email_and_nanoseconds_after_profile_change() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let user = db.user("litigator", true);
    let original_actor = principal(&mut db, user);
    let original = persist(&db, original_actor.clone(), seed.command.clone());
    db.admin.execute("UPDATE users SET email='changed@example.test',role='owner',revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1", &[&user.as_uuid()]).unwrap();
    let current = principal(&mut db, user);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::Revision(seed.command.context.administration_revision),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let replay = service_with_format(
        &db,
        current.clone(),
        FormatCheck(Some(Box::new(|| {
            panic!("closed replay must skip admission")
        }))),
    );
    assert_eq!(
        replay
            .submit(
                "session",
                db.case,
                seed.command.clone(),
                confirmation(&original.group.review)
            )
            .unwrap(),
        original
    );
    assert_eq!(original.group.decision.actor, original_actor);
    assert_eq!(original.group.recorded_at.nanosecond(), 123_456_789);
    assert_eq!(
        reads(&db, current.clone())
            .get("session", db.case, seed.command.decision_id)
            .unwrap(),
        original
    );
    let before = snapshot(&mut db);
    assert!(service(&db, current)
        .prepare("session", db.case, no_change(&seed.command))
        .is_err());
    assert_eq!(snapshot(&mut db), before);
}
