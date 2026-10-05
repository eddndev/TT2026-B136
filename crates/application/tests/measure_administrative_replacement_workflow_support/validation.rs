use crate::workflow_support::*;
use domain::identity::Role;

#[test]
fn replacement_subject_must_be_present_exact_and_no_later_than_the_observed_capture_floor() {
    for mutation in 0..3 {
        let mut fixture = Fixture::single();
        match mutation {
            0 => fixture.material.replacement_subject = None,
            1 => {
                fixture
                    .material
                    .replacement_subject
                    .as_mut()
                    .unwrap()
                    .case_id = CaseId::new()
            }
            _ => {
                fixture
                    .material
                    .replacement_subject
                    .as_mut()
                    .unwrap()
                    .values_digest = Sha256Digest::from_array([6; 32])
            }
        }
        let h = harness(fixture.store(), identity(fixture.actor));
        assert!(h
            .service
            .prepare("session", fixture.case_id, fixture.command)
            .is_err());
        assert!(h.events().is_empty());
    }
    let mut fixture = Fixture::single();
    let floor = now() + time::Duration::nanoseconds(1);
    fixture
        .material
        .replacement_subject
        .as_mut()
        .unwrap()
        .changed_at = floor;
    let h = harness(fixture.store(), identity(fixture.actor.clone()));
    assert!(h
        .service
        .prepare("session", fixture.case_id, fixture.command.clone())
        .is_err());
    for capture_at in [floor - time::Duration::nanoseconds(1), floor] {
        let review = fixture.review();
        let mut store = fixture.store();
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, prepared| prepared.into_operation(capture_at));
        let h = harness_with(
            store,
            identity(fixture.actor.clone()),
            Validator::default(),
            Arc::new(FixedClock(floor)),
        );
        let result = h.service.submit(
            "session",
            fixture.case_id,
            fixture.command.clone(),
            confirmation(&review),
        );
        if capture_at == floor {
            assert_eq!(result.unwrap().capture.recorded_at, floor);
        } else {
            assert!(result.is_err());
        }
    }
}

#[test]
fn stale_head_known_dependants_and_an_existing_new_identity_fail_before_document_admission() {
    for mutation in 0..3 {
        let mut fixture = Fixture::single();
        let prior = fixture.history.records.judicial.groups[0].capture.clone();
        match mutation {
            0 => {
                fixture.material.target_head = PrecautionaryMeasureRef::new(
                    fixture.command.target.id(),
                    fixture.command.target.revision().next().unwrap(),
                    fixture.command.target.digest(),
                )
            }
            1 => {
                let mut next = crate::effect_support::LaterFixture::confirm(&prior);
                next.request.material.support = prior.decision.support.clone();
                next.request.command.values = prior.review.command.values.clone();
                let later = next.clone().capture();
                fixture
                    .material
                    .dependency_inventory
                    .records
                    .records
                    .judicial = crate::effect_support::append_history(&next.evidence, &later);
            }
            _ => {
                let mut independent = crate::measure_decision_fixtures::Fixture::single();
                independent.command.operation_id =
                    MeasureDecisionOperationId::from_uuid(uuid::Uuid::from_u128(9001));
                independent.command.decision_id =
                    MeasureDecisionId::from_uuid(uuid::Uuid::from_u128(9002));
                independent.command.values = prior.review.command.values.clone();
                independent.material.support = prior.decision.support.clone();
                let proposal = MeasureProposal {
                    id: id(10),
                    values: prior.measures[0].result.values.clone(),
                };
                independent.command.outcome =
                    MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
                        MeasureEffect::Impose(proposal),
                    ]))
                    .unwrap();
                independent.material.result_sources[0].id = id(10);
                let group = independent.capture();
                fixture
                    .material
                    .dependency_inventory
                    .records
                    .records
                    .judicial
                    .groups
                    .extend(
                        crate::effect_support::append_history(
                            &crate::effect_support::empty_history(),
                            &group,
                        )
                        .groups,
                    );
            }
        }
        let h = harness(fixture.store(), identity(fixture.actor));
        let error = h
            .service
            .prepare("session", fixture.case_id, fixture.command)
            .unwrap_err();
        if mutation == 0 {
            assert!(matches!(
                error,
                ApplicationError::MeasureAdministrative(MeasureAdministrativeError::StaleHead)
            ));
        }
        if mutation == 1 {
            assert!(matches!(
                error,
                ApplicationError::MeasureAdministrative(
                    MeasureAdministrativeError::KnownDependants
                )
            ));
        }
        assert!(h.events().is_empty());
        assert_eq!(h.validator.calls(), 0);
    }
}

#[test]
fn full_principal_reauthentication_and_retained_support_admission_guard_joint_commit() {
    for mutation in 0..3 {
        let fixture = Fixture::single();
        let review = fixture.review();
        let before = fixture.actor.clone();
        let mut after = before.clone();
        match mutation {
            0 => after.id = domain::identity::UserId::new(),
            1 => after.email = "changed-current-recorder@example.test".into(),
            _ => after.role = Role::Owner,
        }
        let mut identity = crate::case_support::MockIdentity::new();
        let mut sequence = mockall::Sequence::new();
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| Ok(before));
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| Ok(after));
        let h = harness(fixture.store(), identity);
        assert!(matches!(
            h.service.submit(
                "session",
                fixture.case_id,
                fixture.command,
                confirmation(&review)
            ),
            Err(ApplicationError::InvalidSession)
        ));
    }
    for parser_failure in [false, true] {
        let mut fixture = Fixture::single();
        let mut validator = Validator::default();
        if parser_failure {
            validator.failure = true;
        } else {
            fixture.material.support_record.digest = Sha256Digest::from_array([9; 32]);
        }
        let h = harness_with(
            fixture.store(),
            identity(fixture.actor),
            validator,
            Arc::new(FixedClock(now())),
        );
        assert!(h
            .service
            .prepare("session", fixture.case_id, fixture.command)
            .is_err());
        assert_eq!(h.validator.calls(), usize::from(parser_failure));
        if !parser_failure {
            assert!(h.events().is_empty());
        }
    }
}

#[test]
fn commit_cannot_return_a_missing_sibling_or_a_valid_but_unreviewed_subject_snapshot() {
    for mutation in 0..2 {
        let fixture = Fixture::single();
        let review = fixture.review();
        let mut returned = fixture.operation(now());
        if mutation == 0 {
            returned.capture.records.pop();
        } else {
            let mut changed = fixture.clone();
            changed
                .material
                .replacement_subject
                .as_mut()
                .unwrap()
                .changed_by
                .email = "changed-original-subject-author@example.test".into();
            returned = changed.operation(now());
            assert_eq!(
                returned.capture.review.submission_digest,
                review.submission_digest
            );
            assert_ne!(returned.capture.review.review_digest, review.review_digest);
        }
        let mut store = fixture.store();
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, _| Ok(returned));
        let h = harness(store, identity(fixture.actor));
        assert!(h
            .service
            .submit(
                "session",
                fixture.case_id,
                fixture.command,
                confirmation(&review)
            )
            .is_err());
    }
}
