use super::*;

#[test]
fn schedule_replace_cancel_reopens_with_exact_revisions_original_operations_and_prefixes() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let first = persist(&db, actor.clone(), command.clone());
    let second = persist(&db, actor.clone(), replacement(&first));
    let third = persist(&db, actor.clone(), cancellation(&second));
    assert_eq!(third.capture.review.status, HearingStatus::Cancelled);
    assert_eq!(
        third.capture.review.resolved_values,
        second.capture.review.resolved_values
    );
    assert_eq!(
        third.capture.review.scheduling_context,
        second.capture.review.scheduling_context
    );
    assert_eq!(third.history.origin, first.history.origin);
    assert_eq!(
        third.history.captures,
        vec![
            first.capture.clone(),
            second.capture.clone(),
            third.capture.clone()
        ]
    );
    assert_eq!(third.capture.review.result_revision.get(), 3);

    let reopened = reads(&db, actor.clone());
    let id = command.hearing_id;
    assert_eq!(reopened.get("session", db.case, id, None).unwrap(), third);
    for operation in [&first, &second, &third] {
        assert_eq!(
            reopened
                .get(
                    "session",
                    db.case,
                    id,
                    Some(operation.capture.review.result_revision)
                )
                .unwrap(),
            *operation
        );
        assert_eq!(
            reopened
                .get_operation(
                    "session",
                    db.case,
                    operation.capture.review.command.operation_id
                )
                .unwrap(),
            *operation
        );
    }
    let replay = service_with_format(
        &db,
        actor,
        FormatCheck(Some(Box::new(|| panic!("replay must skip admission")))),
    );
    assert_eq!(
        replay
            .submit(
                "session",
                db.case,
                command,
                confirmation(&first.capture.review)
            )
            .unwrap(),
        first
    );
}

#[test]
fn list_pages_current_heads_including_cancelled_hearings_and_excludes_prior_revisions() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, mut command) = setup(&mut db);
    command.hearing_id = PrecautionaryHearingId::from_uuid(uuid::Uuid::from_u128(10));
    let first = persist(&db, actor.clone(), command.clone());
    let cancelled = persist(&db, actor.clone(), cancellation(&first));
    command.hearing_id = PrecautionaryHearingId::from_uuid(uuid::Uuid::from_u128(20));
    command.operation_id = PrecautionaryHearingOperationId::new();
    let second = persist(&db, actor.clone(), command);
    let read = reads(&db, actor);
    let page = read
        .list(
            "session",
            db.case,
            PrecautionaryHearingReadQuery::new(1, None).unwrap(),
        )
        .unwrap();
    assert_eq!(page.items, vec![cancelled.clone()]);
    assert!(page.has_more);
    assert_eq!(
        page.next_after_id,
        Some(cancelled.capture.review.command.hearing_id)
    );
    let last = read
        .list(
            "session",
            db.case,
            PrecautionaryHearingReadQuery::new(1, page.next_after_id).unwrap(),
        )
        .unwrap();
    assert_eq!(last.items, vec![second]);
    assert!(!last.has_more);
    assert_eq!(last.next_after_id, None);
}

#[test]
fn reopened_replay_preserves_original_litigator_profile_even_after_case_closure() {
    use application::cases::{CaseAdministrativeStatus, CaseRevisionExpectation};
    let Some(mut db) = Fixture::new() else { return };
    let (_, command) = setup(&mut db);
    let user = db.user("litigator", true);
    let original = principal(&mut db, user);
    let first = persist(&db, original.clone(), command.clone());
    db.admin
        .execute(
            "UPDATE users SET email='new-profile@example.test',role='owner',revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
            &[&user.as_uuid()],
        )
        .unwrap();
    let current = principal(&mut db, user);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::Revision(
                first
                    .capture
                    .review
                    .observed_context
                    .material()
                    .administration
                    .revision,
            ),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let replay = service_with_format(
        &db,
        current.clone(),
        FormatCheck(Some(Box::new(|| {
            panic!("closed-case replay must skip admission")
        }))),
    );
    let actual = replay
        .submit(
            "session",
            db.case,
            command.clone(),
            confirmation(&first.capture.review),
        )
        .unwrap();
    assert_eq!(actual, first);
    assert_eq!(actual.capture.review.actor, original);
    assert_eq!(
        reads(&db, current)
            .get_operation("session", db.case, command.operation_id)
            .unwrap(),
        first
    );
}
