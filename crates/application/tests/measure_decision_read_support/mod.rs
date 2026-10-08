mod ports;
pub use ports::*;
mod authorization;
mod pagination;
mod selectors;

pub use crate::context_support::Hasher;
pub use application::{identity::Principal, precautionary_measures::*, ApplicationError};
pub use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::Sha256Digest,
    identity::{Role, UserId},
    precautionary_measures::*,
};
use time::Duration;
use uuid::Uuid;

pub fn at() -> OffsetDateTime {
    crate::measure_decision_fixtures::at()
}
pub fn now() -> OffsetDateTime {
    at() + Duration::seconds(100)
}
pub fn id(value: u128) -> MeasureDecisionId {
    MeasureDecisionId::from_uuid(Uuid::from_u128(value))
}

pub fn reader(role: Role) -> Principal {
    Principal {
        id: UserId::from_uuid(Uuid::from_u128(900)),
        email: "reader@example.test".into(),
        role,
    }
}

pub fn empty_history() -> MeasureHistoryEvidence {
    MeasureHistoryEvidence { groups: vec![] }
}

pub fn root_fixture(value: u128) -> crate::measure_decision_fixtures::Fixture {
    let mut fixture = crate::measure_decision_fixtures::Fixture::single();
    fixture.command.decision_id = id(value);
    fixture.command.operation_id =
        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(1000 + value));
    let measure = crate::measure_decision_fixtures::id(2000 + value);
    let values = match &fixture.command.outcome.changes().unwrap()[0] {
        MeasureEffect::Impose(proposal) => proposal.values.clone(),
        _ => unreachable!(),
    };
    fixture.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Impose(MeasureProposal {
                id: measure,
                values,
            }),
        ]))
        .unwrap();
    fixture.material.result_sources[0].id = measure;
    fixture
}

pub fn operation(value: u128) -> MeasureDecisionStoredOperation {
    stored(root_fixture(value).capture(), empty_history())
}

pub fn stored(
    group: MeasureDecisionGroupCapture,
    measure_history: MeasureHistoryEvidence,
) -> MeasureDecisionStoredOperation {
    let origin = measure_group_origin(&Hasher, &group, &measure_history).unwrap();
    MeasureDecisionStoredOperation {
        group,
        origin,
        measure_history,
    }
}

pub fn confirmed(
    previous: &MeasureDecisionStoredOperation,
    value: u128,
) -> MeasureDecisionStoredOperation {
    let mut next = crate::effect_support::LaterFixture::next(
        &previous.group,
        &previous.measure_history,
        value,
    );
    next.request.command.decision_id = id(value);
    next.request.command.operation_id =
        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(1000 + value));
    let evidence = next.evidence.clone();
    stored(next.capture(), evidence)
}

pub fn page(case_id: CaseId, items: Vec<MeasureDecisionStoredOperation>) -> MeasureDecisionPage {
    MeasureDecisionPage {
        case_id,
        items,
        has_more: false,
        next_after_id: None,
    }
}

#[derive(Clone, Copy)]
pub enum ReadKind {
    List,
    Get,
    Operation,
}
pub const READS: [ReadKind; 3] = [ReadKind::List, ReadKind::Get, ReadKind::Operation];

pub fn read(
    service: &MeasureDecisionReadService,
    kind: ReadKind,
    expected: &MeasureDecisionStoredOperation,
) -> Result<Vec<MeasureDecisionStoredOperation>, ApplicationError> {
    let case_id = expected.group.review.case_id;
    match kind {
        ReadKind::List => service
            .list("session", case_id, MeasureDecisionReadQuery::default())
            .map(|page| page.items),
        ReadKind::Get => service
            .get("session", case_id, expected.group.decision.decision_id)
            .map(|value| vec![value]),
        ReadKind::Operation => service
            .get_operation("session", case_id, expected.group.decision.operation_id)
            .map(|value| vec![value]),
    }
}

pub fn successful_store(
    actor: &Principal,
    expected: &MeasureDecisionStoredOperation,
    returned: MeasureDecisionStoredOperation,
    kind: ReadKind,
) -> MockReads {
    let mut store = MockReads::new();
    let actor = actor.clone();
    let case_id = expected.group.review.case_id;
    match kind {
        ReadKind::List => {
            store
                .expect_list()
                .times(1)
                .return_once(move |a, c, query| {
                    assert_eq!((a, c), (&actor, case_id));
                    assert_eq!(query, MeasureDecisionReadQuery::default());
                    Ok(page(case_id, vec![returned]))
                });
        }
        ReadKind::Get => {
            let decision = expected.group.decision.decision_id;
            store.expect_get().times(1).return_once(move |a, c, d| {
                assert_eq!((a, c, d), (&actor, case_id, decision));
                Ok(returned)
            });
        }
        ReadKind::Operation => {
            let operation = expected.group.decision.operation_id;
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

mod evidence_tests;

mod ordinary_operation_tests;
