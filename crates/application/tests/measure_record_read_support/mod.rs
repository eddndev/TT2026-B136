mod fixtures;
mod lifecycle;
mod ports;
pub use crate::record_decision_support::*;
pub use application::{identity::Principal, ApplicationError};
pub use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::Sha256Digest,
    identity::{Role, UserId},
};
pub use fixtures::*;
pub use ports::*;
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
pub fn page(case_id: CaseId, items: Vec<MeasureRecordDetail>) -> MeasureRecordPage {
    MeasureRecordPage {
        case_id,
        items,
        has_more: false,
        next_after_id: None,
    }
}

#[derive(Clone, Copy)]
pub enum ReadKind {
    List,
    Current,
    Exact,
}
pub const READS: [ReadKind; 3] = [ReadKind::List, ReadKind::Current, ReadKind::Exact];

pub fn read(
    service: &MeasureRecordReadService,
    kind: ReadKind,
    expected: &MeasureRecordDetail,
) -> Result<Vec<MeasureRecordDetail>, ApplicationError> {
    match kind {
        ReadKind::List => service
            .list(
                "session",
                expected.case_id,
                MeasureRecordReadQuery::default(),
            )
            .map(|page| page.items),
        ReadKind::Current => service
            .get("session", expected.case_id, expected.reference.id())
            .map(|row| vec![row]),
        ReadKind::Exact => service
            .exact("session", expected.case_id, expected.reference)
            .map(|row| vec![row]),
    }
}
pub fn successful_store(
    actor: &Principal,
    expected: &MeasureRecordDetail,
    returned: MeasureRecordDetail,
    kind: ReadKind,
) -> MockReads {
    let mut store = MockReads::new();
    let actor = actor.clone();
    let case_id = expected.case_id;
    let selected = expected.reference;
    match kind {
        ReadKind::List => {
            store.expect_list().times(1).return_once(move |a, c, q| {
                assert_eq!(
                    (a, c, q),
                    (&actor, case_id, MeasureRecordReadQuery::default())
                );
                Ok(page(case_id, vec![returned]))
            });
        }
        ReadKind::Current => {
            store.expect_get().times(1).return_once(move |a, c, id| {
                assert_eq!((a, c, id), (&actor, case_id, selected.id()));
                Ok(returned)
            });
        }
        ReadKind::Exact => {
            store
                .expect_exact()
                .times(1)
                .return_once(move |a, c, reference| {
                    assert_eq!((a, c, reference), (&actor, case_id, selected));
                    Ok(returned)
                });
        }
    }
    store
}
pub fn list_result(
    query: MeasureRecordReadQuery,
    returned: MeasureRecordPage,
) -> Result<MeasureRecordPage, ApplicationError> {
    let actor = reader(Role::Paralegal);
    let case_id = crate::participant_support::case_id();
    let expected = actor.clone();
    let mut store = MockReads::new();
    store.expect_list().times(1).return_once(move |a, c, q| {
        assert_eq!((a, c, q), (&expected, case_id, query));
        Ok(returned)
    });
    service(store, identity(&actor)).list("session", case_id, query)
}
pub fn reject_reads(expected: &MeasureRecordDetail, returned: &MeasureRecordDetail) {
    let actor = reader(Role::Paralegal);
    for kind in READS {
        let store = successful_store(&actor, expected, returned.clone(), kind);
        assert!(read(&service(store, identity(&actor)), kind, expected).is_err());
    }
}
