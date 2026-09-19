#[path = "deadline_backend_support/access_tests.rs"]
mod access_tests;
mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod procedural_fact_backend_support;
use application::{deadlines::*, ApplicationError};
use deadline_backend_support::*;
use domain::{identity::Role, procedural_facts::FactDeclaration};
use infrastructure::RingSha256Hasher;

#[test]
fn blocked_and_calculable_registrations_preserve_explicit_inputs_and_exact_receipts() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let profile = profile(&db);
    let source = source(&db);
    for blocked in [true, false] {
        let mut command = command(&db, &profile, &source);
        if blocked {
            definition_mut(&mut command).input.selection.source =
                FactDeclaration::Unknown(text("Exact source not yet supplied"));
        }
        let policies = TrackingPolicies {
            source: if blocked {
                TrackingPolicy::Undetermined
            } else {
                TrackingPolicy::Follow
            },
            ..FOLLOW_RESOLUTION
        };
        let before = snapshot(&mut db);
        let draft = workflow
            .prepare("session", db.case, human(command.clone(), Some(policies)))
            .unwrap();
        assert_eq!(snapshot(&mut db), before);
        let result = workflow
            .submit(
                "session",
                db.case,
                human(command.clone(), Some(policies)),
                draft.submission_digest,
            )
            .unwrap();
        assert_eq!(result.definition, draft.definition);
        assert_eq!(result.calculation, draft.calculation);
        assert_eq!(result.receipt.operation_id, command.operation_id);
        assert_eq!(result.recorded_by.user_id(), Some(db.owner));
        assert_eq!(result.recorded_by.email(), Some("owner@example.test"));
        assert_eq!(result.receipt.version, draft.receipt_version);
        assert!(matches!(
            &result.receipt.version,
            DeadlineReceiptVersion::Tracked(_)
        ));
        assert_eq!(result.tracking.as_ref(), Some(&draft.tracking));
        assert_eq!(draft.tracking.policies, policies);
        assert_eq!(result.recorded_by, draft.author);
        assert_eq!(result.receipt.review_digest, draft.review_digest);
        assert_eq!(result.receipt.capture_digest, draft.capture_digest);
        deadline_receipt_matches(&RingSha256Hasher, &result).unwrap();
        assert_eq!(result.calculation.result.due_at().is_none(), blocked);
        assert_eq!(!result.calculation.result.blocks().is_empty(), blocked);
        if blocked {
            assert!(result.calculation.material.source.is_none());
            assert!(result.calculation.material.source_head.is_none());
        } else {
            let due = result.calculation.result.due_at().unwrap();
            let expected = time::Date::from_calendar_date(2026, time::Month::January, 6)
                .unwrap()
                .with_hms(23, 30, 0)
                .unwrap()
                .assume_utc();
            assert_eq!(due, expected);
            assert_eq!(due.offset(), time::UtcOffset::UTC);
            let local = due.to_offset(time::UtcOffset::from_hms(-6, 0, 0).unwrap());
            assert_eq!(local.time(), time::Time::from_hms(17, 30, 0).unwrap());
        }
        assert_eq!(
            workflow
                .get("session", db.case, result.id, Some(result.revision))
                .unwrap(),
            result
        );
    }
    let page = workflow.list("session", db.case, query(100, None)).unwrap();
    assert_eq!(page.deadlines.len(), 2);
    assert_eq!(
        page.deadlines
            .iter()
            .filter(|row| row.calculation_blocked)
            .count(),
        1
    );
    assert_eq!(
        page.deadlines
            .iter()
            .filter(|row| row.operational.due_at().is_some())
            .count(),
        1
    );
    assert!(page.deadlines.iter().all(
        |row| row.review_state == application::deadline_tracking::DeadlineReviewState::Accepted
    ));
}

#[test]
fn corrections_attention_retirement_and_paged_history_preserve_historical_calculation() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist(
        &workflow,
        db.case,
        human(setup(&db), Some(FOLLOW_RESOLUTION)),
    );
    let second = persist(
        &workflow,
        db.case,
        human(correct(&first), Some(FOLLOW_RESOLUTION)),
    );
    let third = persist(&workflow, db.case, human(attention(&second), None));
    let fourth = persist(&workflow, db.case, human(retire(&third), None));
    assert_ne!(first.definition.title, second.definition.title);
    assert_eq!(second.calculation, third.calculation);
    assert_eq!(third.calculation, fourth.calculation);
    assert_eq!(second.tracking, third.tracking);
    assert_eq!(third.tracking, fourth.tracking);
    assert_eq!(third.attention, fourth.attention);
    assert!(matches!(
        third.attention,
        DeadlineAttention::Recorded { .. }
    ));
    assert_eq!(fourth.status, DeadlineStatus::Retired);
    let page = workflow
        .history("session", db.case, first.id, history_query(2, None))
        .unwrap();
    assert!(page.has_more);
    assert_eq!(page.next_before_revision, Some(third.revision));
    let older = workflow
        .history(
            "session",
            db.case,
            first.id,
            history_query(2, Some(third.revision.get())),
        )
        .unwrap();
    assert!(!older.has_more);
    let entries: Vec<_> = page.revisions.into_iter().chain(older.revisions).collect();
    for (entry, detail) in entries.iter().zip([&fourth, &third, &second, &first]) {
        assert_eq!(
            entry,
            &DeadlineHistoryEntry::from_detail(&RingSha256Hasher, detail).unwrap()
        );
        assert_eq!(
            workflow
                .get("session", db.case, first.id, Some(detail.revision))
                .unwrap(),
            *detail
        );
    }
    let before = snapshot(&mut db);
    assert!(matches!(
        workflow.prepare(
            "session",
            db.case,
            human(correct(&fourth), Some(FOLLOW_RESOLUTION))
        ),
        Err(ApplicationError::Deadline(DeadlineError::Retired))
    ));
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn failed_audit_rolls_back_new_root_and_followup_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let first = persist_legacy(&db, db.owner, setup(&db));
    let mut register = correct(&first);
    register.deadline_id = DeadlineId::new();
    register.change = DeadlineChange::Register {
        definition: first.definition.clone(),
    };
    let changes = [
        prepared_legacy(&db, db.owner, &register),
        prepared_legacy(&db, db.owner, &correct(&first)),
    ];
    db.admin.batch_execute("CREATE FUNCTION reject_deadline_audit() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN RAISE EXCEPTION 'injected audit failure'; END; $$;
        CREATE TRIGGER reject_deadline_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_deadline_audit()").unwrap();
    let before = snapshot(&mut db);
    for prepared in changes {
        assert!(store(&db).commit(db.owner, prepared).is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}
