mod authorization;
mod bounds;
mod clocks;
mod evidence;
mod integrity;
mod mixed;
mod pagination;
mod ports;
pub use crate::record_decision_support::*;
pub use application::{identity::Principal, ApplicationError};
pub use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::Sha256Digest,
    identity::{Role, UserId},
};
pub use ports::*;
pub use time::Duration;
pub use uuid::Uuid;

pub fn at() -> OffsetDateTime {
    crate::measure_decision_fixtures::at()
}
pub fn now() -> OffsetDateTime {
    at() + Duration::seconds(100)
}
pub fn operation_id(serial: u128) -> MeasureCorrectionOperationId {
    MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(5000 + serial))
}
pub fn reader(role: Role) -> Principal {
    Principal {
        id: UserId::from_uuid(Uuid::from_u128(900)),
        email: "reader@example.test".into(),
        role,
    }
}

pub fn root_fixture(serial: u128) -> crate::measure_decision_fixtures::Fixture {
    let mut fixture = crate::measure_decision_fixtures::Fixture::single();
    fixture.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(1000 + serial));
    fixture.command.operation_id =
        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(2000 + serial));
    let measure_id = id(3000 + serial);
    let mut effects = fixture.command.outcome.changes().unwrap().to_vec();
    let MeasureEffect::Impose(proposal) = &mut effects[0] else {
        unreachable!()
    };
    proposal.id = measure_id;
    fixture.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap();
    fixture.material.result_sources[0].id = measure_id;
    fixture
}

pub fn from_group(
    group: &MeasureDecisionGroupCapture,
    ancestors: &MeasureHistoryEvidence,
    serial: u128,
) -> RecordFixture {
    let mut fixture = RecordFixture::from_first(CorrectionFixture::from_group(
        group,
        ancestors,
        group.measures[0].result.id,
    ));
    fixture.command.operation_id = operation_id(serial);
    fixture
}
pub fn stored(fixture: &RecordFixture) -> MeasureAdministrativeStoredOperation {
    let history = MeasureDecisionRecordHistoryEvidence {
        records: fixture.history.clone(),
        decisions: vec![],
    };
    capture(
        fixture.command.clone(),
        fixture.context.clone(),
        history,
        fixture.recorded_at,
    )
}
pub fn capture(
    command: MeasureAdministrativeCommand,
    context: application::precautionary_hearings::PrecautionaryContext,
    record_history: MeasureDecisionRecordHistoryEvidence,
    recorded_at: OffsetDateTime,
) -> MeasureAdministrativeStoredOperation {
    let actor = crate::measure_decision_fixtures::Fixture::single().actor;
    let capture = prepare_measure_administrative_record_with_decision_history(
        &Hasher,
        &actor,
        context.material().case_id,
        command,
        context,
        &record_history,
    )
    .unwrap()
    .into_capture(&Hasher, recorded_at)
    .unwrap();
    let origin =
        measure_administrative_origin_with_decision_history(&Hasher, &capture, &record_history)
            .unwrap();
    MeasureAdministrativeStoredOperation {
        capture,
        origin,
        record_history,
    }
}
pub fn operation(serial: u128) -> MeasureAdministrativeStoredOperation {
    stored(&from_group(
        &root_fixture(serial).capture(),
        &crate::effect_support::empty_history(),
        serial,
    ))
}
pub fn page(
    case_id: CaseId,
    items: Vec<MeasureAdministrativeStoredOperation>,
) -> MeasureAdministrativePage {
    MeasureAdministrativePage {
        case_id,
        items,
        has_more: false,
        next_after_operation_id: None,
    }
}

#[derive(Clone, Copy)]
pub enum ReadKind {
    List,
    Operation,
}
pub const READS: [ReadKind; 2] = [ReadKind::List, ReadKind::Operation];

pub fn read(
    service: &MeasureAdministrativeReadService,
    kind: ReadKind,
    expected: &MeasureAdministrativeStoredOperation,
) -> Result<Vec<MeasureAdministrativeStoredOperation>, ApplicationError> {
    let case_id = expected.capture.review.case_id;
    match kind {
        ReadKind::List => service
            .list(
                "session",
                case_id,
                MeasureAdministrativeReadQuery::default(),
            )
            .map(|page| page.items),
        ReadKind::Operation => service
            .get_operation(
                "session",
                case_id,
                expected.capture.review.command.operation_id,
            )
            .map(|value| vec![value]),
    }
}

pub fn successful_store(
    actor: &Principal,
    expected: &MeasureAdministrativeStoredOperation,
    returned: MeasureAdministrativeStoredOperation,
    kind: ReadKind,
) -> MockReads {
    let mut store = MockReads::new();
    let actor = actor.clone();
    let case_id = expected.capture.review.case_id;
    match kind {
        ReadKind::List => {
            store
                .expect_list()
                .times(1)
                .return_once(move |a, c, query| {
                    assert_eq!((a, c), (&actor, case_id));
                    assert_eq!(query, MeasureAdministrativeReadQuery::default());
                    Ok(page(case_id, vec![returned]))
                });
        }
        ReadKind::Operation => {
            let operation = expected.capture.review.command.operation_id;
            store
                .expect_get_operation()
                .times(1)
                .return_once(move |a, c, o| {
                    assert_eq!((a, c, o), (&actor, case_id, operation));
                    Ok(returned)
                });
        }
    }
    store
}

pub fn list_result(
    query: MeasureAdministrativeReadQuery,
    returned: MeasureAdministrativePage,
) -> Result<MeasureAdministrativePage, ApplicationError> {
    let actor = reader(Role::Paralegal);
    let case_id = crate::participant_support::case_id();
    let expected = actor.clone();
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |a, c, actual| {
            assert_eq!((a, c, actual), (&expected, case_id, query));
            Ok(returned)
        });
    service(store, identity(&actor)).list("session", case_id, query)
}

pub fn reject_reads(
    expected: &MeasureAdministrativeStoredOperation,
    returned: &MeasureAdministrativeStoredOperation,
) {
    let actor = reader(Role::Paralegal);
    for kind in READS {
        let store = successful_store(&actor, expected, returned.clone(), kind);
        assert!(read(&service(store, identity(&actor)), kind, expected).is_err());
    }
}
