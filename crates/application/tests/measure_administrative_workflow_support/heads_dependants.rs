use super::*;

#[test]
fn stale_target_head_is_rejected_before_retained_support_admission() {
    for mutation in 0..3 {
        let mut fixture = Fixture::single();
        let target = fixture.command.target;
        fixture.material.target_head = match mutation {
            0 => PrecautionaryMeasureRef::new(id(999), target.revision(), target.digest()),
            1 => PrecautionaryMeasureRef::new(
                target.id(),
                target.revision().next().unwrap(),
                target.digest(),
            ),
            _ => PrecautionaryMeasureRef::new(
                target.id(),
                target.revision(),
                Sha256Digest::from_array([99; 32]),
            ),
        };
        let harness = harness(fixture.store(), identity(fixture.actor));
        assert!(matches!(
            harness
                .service
                .prepare("session", fixture.case_id, fixture.command),
            Err(ApplicationError::MeasureAdministrative(
                MeasureAdministrativeError::StaleHead
            ))
        ));
        assert!(harness.events().is_empty());
        assert_eq!(harness.validator.calls(), 0);
    }
}

fn with_dependant(mut fixture: Fixture, kind: u8) -> Fixture {
    let prior = base(&fixture);
    match kind {
        0 => {
            let mut next = crate::effect_support::LaterFixture::confirm(&prior);
            next.request.material.support = prior.decision.support.clone();
            let mut values =
                crate::measure_decision_fixtures::decision_input(&next.request.command.values);
            values.support = prior.review.command.values.support();
            next.request.command.values = MeasureDecisionValues::new(values);
            let captured = next.clone().capture();
            fixture
                .material
                .dependency_inventory
                .records
                .records
                .judicial = crate::effect_support::append_history(&next.evidence, &captured);
        }
        1 => {
            let captured = fixture.operation(at() + Duration::seconds(1)).capture;
            fixture.material.dependency_inventory.records =
                append_administrative_decision_history(&fixture.history, &captured);
            fixture.command.operation_id =
                MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(999));
        }
        _ => {
            let hearing = DecisionReviewFixture::schedule(
                vec![fixture.command.target],
                fixture.history.clone(),
            )
            .capture(None, at());
            fixture.material.dependency_inventory.hearings.push(
                crate::measure_dependency_support::hearing_prefix(
                    vec![hearing.clone()],
                    &fixture.history,
                ),
            );
            if kind == 3 {
                let mut request =
                    FixtureV2::initial(crate::measure_decision_fixtures::Fixture::no_change());
                request.identities(55);
                request.history = fixture.history.clone();
                request.material.support = prior.decision.support.clone();
                let mut values =
                    crate::measure_decision_fixtures::decision_input(&request.command.values);
                values.support = prior.review.command.values.support();
                request.command.values = MeasureDecisionValues::new(values);
                request.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
                    hearing_id: hearing.review.command.hearing_id,
                    revision: hearing.review.result_revision,
                    capture_digest: hearing.capture_digest,
                });
                request.material.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(
                    Box::new(hearing),
                ));
                request.recorded_at = at() + Duration::seconds(1);
                let group = request.capture();
                assert!(group.measures.is_empty());
                fixture.material.dependency_inventory.records = append_v2(&request.history, &group);
            }
        }
    }
    fixture
}

#[test]
fn judicial_administrative_review_and_zero_row_review_anchor_uses_block_fresh_administration() {
    for kind in 0..4 {
        let fixture = with_dependant(Fixture::single(), kind);
        let report = inspect_measure_administrative_dependencies(
            &Hasher,
            fixture.case_id,
            fixture.command.target,
            &fixture.material.dependency_inventory,
        )
        .unwrap();
        assert!(!report.dependants().is_empty());
        let harness = harness(fixture.store(), identity(fixture.actor));
        assert!(matches!(
            harness
                .service
                .prepare("session", fixture.case_id, fixture.command),
            Err(ApplicationError::MeasureAdministrative(
                MeasureAdministrativeError::KnownDependants
            ))
        ));
        assert!(harness.events().is_empty());
        assert_eq!(harness.validator.calls(), 0);
    }
}

#[test]
fn an_exact_marked_head_is_not_a_fresh_correction_or_mark_target() {
    let mut original = Fixture::single();
    original.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    let marked = original.operation(at() + Duration::seconds(1)).capture;
    for remark in [false, true] {
        let mut fixture = Fixture::single();
        fixture.command.operation_id =
            MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(501));
        fixture.command.target = record_reference(&marked.records[0]);
        fixture.material.target_head = fixture.command.target;
        if remark {
            fixture.command.action = MeasureAdministrativeAction::MarkEnteredInError;
        }
        fixture.material.dependency_inventory.records =
            append_administrative_decision_history(&original.history, &marked);
        reject_before_admission(fixture);
    }
}

#[test]
fn malformed_supplied_forest_and_missing_anchor_prefixes_cannot_authorize_a_write() {
    for mutation in 0..4 {
        let mut fixture = if mutation == 3 {
            with_dependant(Fixture::single(), 3)
        } else {
            Fixture::single()
        };
        match mutation {
            0 => fixture
                .material
                .dependency_inventory
                .records
                .records
                .judicial
                .groups
                .clear(),
            1 => {
                fixture
                    .material
                    .dependency_inventory
                    .records
                    .records
                    .judicial
                    .groups[0]
                    .origin
                    .group_digest = Sha256Digest::from_array([99; 32])
            }
            2 => fixture
                .material
                .dependency_inventory
                .records
                .records
                .judicial
                .groups[0]
                .capture
                .measures
                .clear(),
            _ => fixture.material.dependency_inventory.hearings.clear(),
        }
        reject_before_admission(fixture);
    }
}
