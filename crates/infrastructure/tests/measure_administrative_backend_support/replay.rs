use super::*;
use application::cases::{CaseAdministrativeStatus, CaseRepository, CaseRevisionExpectation};

#[test]
fn closed_case_replay_keeps_original_principal_and_skips_current_eligibility() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, _, command) = setup(&mut db);
    let user = db.user("litigator", true);
    let original_actor = crate::measure_fixture::principal(&mut db, user);
    let original = persist(&db, original_actor.clone(), command);
    let later = persist(
        &db,
        original_actor.clone(),
        mark(corrected_reference(&original.capture), seed.command.context),
    );
    db.admin
        .execute(
            "UPDATE users SET email='changed-administrator@example.test',role='owner',
         revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
            &[&user.as_uuid()],
        )
        .unwrap();
    let current_actor = crate::measure_fixture::principal(&mut db, user);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::Revision(seed.command.context.administration_revision),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();

    reopened(&db, &current_actor, &original);
    reopened(&db, &current_actor, &later);

    assert_eq!(original.capture.review.actor, original_actor);
    assert_eq!(original.capture.recorded_at.nanosecond(), 123_456_789);
    assert_eq!(
        db.admin
            .query_one(
                "SELECT count(*) FROM audit_events WHERE action='measure_administrative.recorded'",
                &[]
            )
            .unwrap()
            .get::<_, i64>(0),
        2
    );
}
