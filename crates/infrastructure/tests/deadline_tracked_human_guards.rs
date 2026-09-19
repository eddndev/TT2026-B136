mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
mod deadline_tracked_guard_support;
mod procedural_fact_backend_support;

use application::{deadline_reevaluation::ObservationRole, deadlines::*};
use deadline_backend_support::*;
use deadline_tracked_backend_support::*;
use deadline_tracked_guard_support::*;
use domain::identity::Role;
use infrastructure::RingSha256Hasher;
use procedural_fact_backend_support as facts;

#[test]
fn sql_register_accepts_fixed_historical_profile_with_its_current_observed_head() {
    let Some(mut db) = Fixture::new() else { return };
    let profile = profile(&db);
    let source = source(&db);
    let head = advance_profile(&db, &profile);
    let repository = store(&db);
    let prepared = tracked_prepared(
        &db,
        repository.as_ref(),
        &command(&db, &profile, &source),
        Some(fixed_profile()),
    );
    let expected = prepared_detail(&db, &prepared);
    assert_eq!(expected.definition.profile.revision, profile.revision);
    assert_eq!(expected.calculation.profile, profile);
    let observation = expected
        .tracking
        .as_ref()
        .unwrap()
        .observations
        .entries
        .iter()
        .find(|entry| entry.role == ObservationRole::Profile)
        .unwrap();
    assert_eq!(observation.revision, head.revision.get());
    let before = snapshot(&mut db);
    insert(&db, &expected)
        .expect("current observed profile permits an explicitly fixed historical selection");
    assert_eq!(
        stored_evidence(&mut db, &expected),
        canonical_evidence(&expected)
    );
    let after = snapshot(&mut db);
    assert_eq!(after["audit"], before["audit"]);
    assert_eq!(after["events"], before["events"]);
    assert_readback(&db, repository.as_ref(), &[expected]);
}

#[test]
fn sql_correct_accepts_fixed_profile_and_preserves_attention_and_prior_captures() {
    let Some(mut db) = Fixture::new() else { return };
    let profile = profile(&db);
    let source = source(&db);
    let repository = store(&db);
    let prepared = tracked_prepared(
        &db,
        repository.as_ref(),
        &command(&db, &profile, &source),
        Some(fixed_profile()),
    );
    let first = prepared_detail(&db, &prepared);
    insert(&db, &first).unwrap();
    let prepared = tracked_prepared(&db, repository.as_ref(), &attention(&first), None);
    let attended = prepared_detail(&db, &prepared);
    insert(&db, &attended).unwrap();
    let prior = revision_row(&mut db, &attended);
    let head = advance_profile(&db, &profile);
    let prepared = tracked_prepared(
        &db,
        repository.as_ref(),
        &correct(&attended),
        Some(fixed_profile()),
    );
    let corrected = prepared_detail(&db, &prepared);
    insert(&db, &corrected)
        .expect("correction may keep a published Fixed profile while observing its head");
    assert_eq!(corrected.definition.profile.revision, profile.revision);
    assert_eq!(corrected.calculation.profile, profile);
    assert_eq!(corrected.attention, attended.attention);
    assert_eq!(
        corrected.tracking.as_ref().unwrap().observations.entries[0].revision,
        head.revision.get()
    );
    assert_eq!(revision_row(&mut db, &attended), prior);
    deadline_successor_matches(&RingSha256Hasher, &attended, &corrected).unwrap();
    assert_readback(&db, repository.as_ref(), &[first, attended, corrected]);
}

#[test]
fn sql_register_and_correct_reject_obsolete_observations_without_partial_rows_or_audit() {
    for correction in [false, true] {
        for profile_change in [false, true] {
            let Some(mut db) = Fixture::new() else { return };
            let profile = profile(&db);
            let source = source(&db);
            let repository = store(&db);
            let initial = command(&db, &profile, &source);
            let change = if correction {
                let prepared =
                    tracked_prepared(&db, repository.as_ref(), &initial, Some(fixed_profile()));
                let first = prepared_detail(&db, &prepared);
                insert(&db, &first).unwrap();
                correct(&first)
            } else {
                initial
            };
            let prepared =
                tracked_prepared(&db, repository.as_ref(), &change, Some(fixed_profile()));
            let stale = prepared_detail(&db, &prepared);
            if profile_change {
                advance_profile(&db, &profile);
            } else {
                facts::persist(
                    &facts::service(&db, db.owner, Role::Owner),
                    db.case,
                    facts::correct(&source),
                );
            }
            let before = snapshot(&mut db);
            let error = insert(&db, &stale)
                .expect_err("stale observed heads must not become a new human qualification");
            assert_eq!(
                error.code(),
                Some(&postgres::error::SqlState::CHECK_VIOLATION)
            );
            assert_eq!(snapshot(&mut db), before);
        }
    }
}

#[test]
fn sql_attention_and_retirement_preserve_v2_tracking_after_a_profile_head_advance() {
    let Some(mut db) = Fixture::new() else { return };
    let profile = profile(&db);
    let source = source(&db);
    let repository = store(&db);
    let prepared = tracked_prepared(
        &db,
        repository.as_ref(),
        &command(&db, &profile, &source),
        Some(fixed_profile()),
    );
    let first = prepared_detail(&db, &prepared);
    insert(&db, &first).unwrap();
    let original = revision_row(&mut db, &first);
    advance_profile(&db, &profile);
    let mut prior = first.clone();
    for retiring in [false, true] {
        let command = if retiring {
            retire(&prior)
        } else {
            attention(&prior)
        };
        let prepared = tracked_prepared(&db, repository.as_ref(), &command, None);
        let next = prepared_detail(&db, &prepared);
        insert(&db, &next).expect("preserving human actions retain historical tracking");
        assert_eq!(next.tracking, first.tracking);
        assert_eq!(next.calculation, first.calculation);
        assert_eq!(next.responsible, first.responsible);
        deadline_successor_matches(&RingSha256Hasher, &prior, &next).unwrap();
        assert_eq!(revision_row(&mut db, &first), original);
        assert_eq!(stored_evidence(&mut db, &next), canonical_evidence(&next));
        prior = next;
    }
    assert_eq!(prior.status, DeadlineStatus::Retired);
}

#[test]
fn sql_human_author_requires_current_case_membership_even_with_a_valid_v2_receipt() {
    let Some(mut db) = Fixture::new() else { return };
    let actor = db.user("litigator", true);
    let email = format!("{actor}@example.test");
    let repository = store(&db);
    let prepared = prepare_as(
        &db,
        repository.as_ref(),
        &setup(&db),
        actor,
        &email,
        Some(fixed_profile()),
    );
    let valid = prepared_detail(&db, &prepared);
    deadline_receipt_matches(&RingSha256Hasher, &valid).unwrap();
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &actor.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    let error = insert(&db, &valid).expect_err("revoked membership must reject direct V2 SQL");
    assert_eq!(
        error.code(),
        Some(&postgres::error::SqlState::INSUFFICIENT_PRIVILEGE)
    );
    assert_eq!(snapshot(&mut db), before);
}
