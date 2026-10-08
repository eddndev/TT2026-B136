mod anchors;
mod authorization;
mod bounds;
mod clocks;
mod ownership;
mod pagination;
mod ports;
mod proofs;
pub use ports::*;
#[allow(dead_code)]
#[path = "../measure_record_read_support/fixtures.rs"]
mod record_fixtures;
pub use crate::record_decision_support::*;
pub use application::{identity::Principal, ApplicationError};
pub use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::Sha256Digest,
    identity::{Role, UserId},
};
pub use record_fixtures::root_fixture;
pub use time::Duration;
pub use uuid::Uuid;

pub fn at() -> OffsetDateTime {
    crate::measure_decision_fixtures::at()
}
pub fn now() -> OffsetDateTime {
    at() + Duration::seconds(100)
}
pub fn reader(role: Role) -> Principal {
    Principal {
        id: UserId::from_uuid(Uuid::from_u128(900)),
        email: "reader@example.test".into(),
        role,
    }
}
pub fn v1(serial: u128) -> MeasureDecisionRecordReceipt {
    let group = root_fixture(serial).capture();
    let measure_history = crate::effect_support::empty_history();
    let origin = measure_group_origin(&Hasher, &group, &measure_history).unwrap();
    MeasureDecisionRecordReceipt::V1(Box::new(MeasureDecisionStoredOperation {
        group,
        origin,
        measure_history,
    }))
}
pub fn from_v2(fixture: &FixtureV2) -> MeasureDecisionRecordReceipt {
    let group = fixture.capture();
    let origin = measure_group_origin_v2(&Hasher, &group, &fixture.history).unwrap();
    MeasureDecisionRecordReceipt::V2(Box::new(MeasureDecisionRecordStoredOperation {
        group,
        origin,
        record_history: fixture.history.clone(),
    }))
}
pub fn v2(serial: u128) -> MeasureDecisionRecordReceipt {
    let rows = record_fixtures::mixed(serial);
    let mut history = rows[2].record_history.clone();
    let owner = history.decisions.pop().unwrap();
    MeasureDecisionRecordReceipt::V2(Box::new(MeasureDecisionRecordStoredOperation {
        group: owner.capture,
        origin: owner.origin,
        record_history: history,
    }))
}
pub fn page(
    case_id: CaseId,
    items: Vec<MeasureDecisionRecordReceipt>,
) -> MeasureDecisionRecordPage {
    MeasureDecisionRecordPage {
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
    service: &MeasureDecisionRecordReadService,
    kind: ReadKind,
    expected: &MeasureDecisionRecordReceipt,
) -> Result<Vec<MeasureDecisionRecordReceipt>, ApplicationError> {
    match kind {
        ReadKind::List => service
            .list(
                "session",
                expected.case_id(),
                MeasureDecisionReadQuery::default(),
            )
            .map(|p| p.items),
        ReadKind::Get => service
            .get("session", expected.case_id(), expected.origin().decision_id)
            .map(|r| vec![r]),
        ReadKind::Operation => service
            .get_operation(
                "session",
                expected.case_id(),
                expected.origin().operation_id,
            )
            .map(|r| vec![r]),
    }
}
pub fn successful_store(
    actor: &Principal,
    expected: &MeasureDecisionRecordReceipt,
    returned: MeasureDecisionRecordReceipt,
    kind: ReadKind,
) -> MockReads {
    let mut store = MockReads::new();
    let actor = actor.clone();
    let case = expected.case_id();
    match kind {
        ReadKind::List => {
            store.expect_list().times(1).return_once(move |a, c, q| {
                assert_eq!(
                    (a, c, q),
                    (&actor, case, MeasureDecisionReadQuery::default())
                );
                Ok(page(case, vec![returned]))
            });
        }
        ReadKind::Get => {
            let id = expected.origin().decision_id;
            store.expect_get().times(1).return_once(move |a, c, d| {
                assert_eq!((a, c, d), (&actor, case, id));
                Ok(returned)
            });
        }
        ReadKind::Operation => {
            let id = expected.origin().operation_id;
            store
                .expect_get_operation()
                .times(1)
                .return_once(move |a, c, o| {
                    assert_eq!((a, c, o), (&actor, case, id));
                    Ok(returned)
                });
        }
    }
    store
}
pub fn list_result(
    query: MeasureDecisionReadQuery,
    returned: MeasureDecisionRecordPage,
) -> Result<MeasureDecisionRecordPage, ApplicationError> {
    let actor = reader(Role::Paralegal);
    let expected = actor.clone();
    let case = crate::participant_support::case_id();
    let mut store = MockReads::new();
    store.expect_list().times(1).return_once(move |a, c, q| {
        assert_eq!((a, c, q), (&expected, case, query));
        Ok(returned)
    });
    service(store, identity(&actor)).list("session", case, query)
}
pub fn reject(expected: &MeasureDecisionRecordReceipt, returned: &MeasureDecisionRecordReceipt) {
    let actor = reader(Role::Paralegal);
    for kind in READS {
        assert!(read(
            &service(
                successful_store(&actor, expected, returned.clone(), kind),
                identity(&actor)
            ),
            kind,
            expected
        )
        .is_err());
    }
}
