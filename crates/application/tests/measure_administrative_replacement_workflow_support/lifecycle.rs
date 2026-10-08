use crate::workflow_support::*;
use domain::identity::Role;

#[test]
fn replacement_preparation_and_dual_confirmed_commit_preserve_both_owned_rows_and_exact_sources() {
    let fixture = Fixture::single();
    let review = fixture.review();
    let h = harness(fixture.store(), identity(fixture.actor.clone()));
    assert_eq!(
        h.service
            .prepare("session", fixture.case_id, fixture.command.clone())
            .unwrap(),
        review
    );
    assert_eq!(h.validator.calls(), 1);
    let expected = fixture.operation(now());
    let expected_review = review.clone();
    let material = fixture.material.clone();
    let actor = fixture.actor.clone();
    let case = fixture.case_id;
    let mut store = fixture.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |actual, actual_case, prepared| {
            assert_eq!((actual, actual_case), (&actor, case));
            assert_eq!(prepared.actor(), &actor);
            assert_eq!(prepared.review(), &expected_review);
            assert_eq!(prepared.material(), &material);
            prepared.into_operation(now())
        });
    let h = harness(store, identity(fixture.actor));
    let actual = h
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&review),
        )
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual.capture.records.len(), 2);
    assert!(actual.capture.replacement_link.is_some());
    assert_eq!(h.validator.calls(), 1);
    assert!(h.events().contains(&"open") && h.events().contains(&"hash"));
}

#[test]
fn both_confirmations_bind_the_instruction_and_complete_replacement_subject_provenance() {
    let fixture = Fixture::single();
    let original = fixture.review();
    for mutation in 0..3 {
        let mut changed = fixture.clone();
        let mut expected = confirmation(&original);
        if mutation == 0 {
            expected.submission_digest = Sha256Digest::from_array([4; 32]);
        }
        if mutation == 1 {
            expected.review_digest = Sha256Digest::from_array([5; 32]);
        }
        if mutation == 2 {
            changed
                .material
                .replacement_subject
                .as_mut()
                .unwrap()
                .changed_by
                .email = "another-historical-author@example.test".into();
            let altered = changed.review();
            assert_eq!(altered.submission_digest, original.submission_digest);
            assert_ne!(altered.review_digest, original.review_digest);
        }
        let h = harness(changed.store(), identity(changed.actor));
        let error = h
            .service
            .submit("session", changed.case_id, changed.command, expected)
            .unwrap_err();
        assert!(
            matches!(
                error,
                ApplicationError::MeasureAdministrative(
                    MeasureAdministrativeError::SubmissionMismatch
                )
            ) || matches!(
                error,
                ApplicationError::MeasureAdministrative(MeasureAdministrativeError::ReviewMismatch)
            )
        );
    }
}

#[test]
fn original_replacement_replay_survives_later_real_heads_and_profile_changes_without_readmission() {
    let mut fixture = Fixture::single();
    let saved =
        fixture.operation(crate::measure_decision_fixtures::at() + time::Duration::seconds(1));
    let mut next = crate::record_decision_support::FixtureV2::confirm(
        &saved.capture,
        &saved.record_history.records,
    );
    next.material.support = saved.capture.review.support.clone();
    let mut values = crate::measure_decision_fixtures::decision_input(&next.command.values);
    values.support = HearingSupportRef::new(
        next.material.support.reference,
        next.material.support.digest,
    );
    next.command.values = MeasureDecisionValues::new(values);
    let later = next.capture();
    assert_eq!(
        later.measures[0].result.id,
        saved.capture.review.replacement.as_ref().unwrap().id
    );
    assert_eq!(later.measures[0].result.revision.get(), 2);
    fixture.material.target_head = crate::record_decision_support::reference_v2(&later.measures[0]);
    fixture.actor.role = Role::Owner;
    fixture.actor.email = "updated-current-owner@example.test".into();
    let h = harness(
        fixture.replay_store(saved.clone()),
        identity(fixture.actor.clone()),
    );
    assert_eq!(
        h.service
            .prepare("session", fixture.case_id, fixture.command.clone())
            .unwrap(),
        saved.capture.review
    );
    assert_eq!(h.validator.calls(), 0);
    assert!(h.events().is_empty());
    let h = harness(fixture.replay_store(saved.clone()), identity(fixture.actor));
    let actual = h
        .service
        .submit(
            "session",
            fixture.case_id,
            fixture.command,
            confirmation(&saved.capture.review),
        )
        .unwrap();
    assert_eq!(actual, saved);
    assert_eq!(h.validator.calls(), 0);
    assert!(h.events().is_empty());
}
