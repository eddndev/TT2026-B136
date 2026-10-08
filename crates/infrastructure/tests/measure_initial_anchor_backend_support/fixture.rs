use super::*;

pub struct AnchorSeed {
    pub actor: Principal,
    pub hearing: HearingDetail,
    pub command: MeasureDecisionCommand,
}

pub fn anchor(hearing: &HearingDetail) -> MeasureDecisionAnchorRef {
    let snapshot = &hearing.snapshot;
    MeasureDecisionAnchorRef::Initial {
        hearing_id: snapshot.id,
        revision: snapshot.revision,
        values_digest: snapshot.values_digest,
        submission_digest: snapshot.receipt.submission_digest,
    }
}

pub fn hearing_service(db: &Fixture) -> HearingService {
    crate::hearing_database_support::service(db, db.owner, Role::Owner)
}

pub fn setup(db: &mut Fixture) -> AnchorSeed {
    let seed = crate::measure_fixture::setup(db);
    let hearing = crate::hearing_database_support::persist(
        &hearing_service(db),
        db.case,
        crate::hearing_database_support::schedule(),
    );
    let mut command = seed.command;
    command.anchor = Some(anchor(&hearing));
    AnchorSeed {
        actor: seed.actor,
        hearing,
        command,
    }
}

pub fn expected_anchor(hearing: &HearingDetail) -> Option<MeasureDecisionAnchorMaterial> {
    Some(MeasureDecisionAnchorMaterial::Initial(Box::new(
        hearing.clone(),
    )))
}

pub fn assert_anchor(operation: &MeasureDecisionStoredOperation, hearing: &HearingDetail) {
    assert_eq!(operation.group.review.command.anchor, Some(anchor(hearing)));
    assert_eq!(
        operation.group.review.material.anchor,
        expected_anchor(hearing)
    );
    assert_eq!(operation.group.decision.anchor, expected_anchor(hearing));
}

pub fn reopened(db: &Fixture, actor: &Principal, expected: &MeasureDecisionStoredOperation) {
    let query = reads(db, actor.clone());
    assert_eq!(
        query
            .get("session", db.case, expected.origin.decision_id)
            .unwrap(),
        *expected
    );
    assert_eq!(
        query
            .get_operation("session", db.case, expected.origin.operation_id)
            .unwrap(),
        *expected
    );
    let replay = service_with_format(
        db,
        actor.clone(),
        FormatCheck(Some(Box::new(|| {
            panic!("replay must retain the original admitted decision support")
        }))),
    );
    assert_eq!(
        replay
            .submit(
                "session",
                db.case,
                expected.group.review.command.clone(),
                confirmation(&expected.group.review)
            )
            .unwrap(),
        *expected
    );
}
