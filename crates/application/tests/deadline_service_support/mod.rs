#![allow(dead_code)]
pub use crate::case_support::{CountingClock, MockIdentity};
pub use crate::deadline_support::evaluation::inputs::{case_id, hasher};
pub use crate::deadline_support::*;
use application::{deadlines::*, identity::Principal, ApplicationError};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    identity::{Role, UserId},
};
use mockall::mock;
use std::sync::Arc;

mock! {
    pub Store {}
    impl DeadlineStore for Store {
        fn responsibles(&self, actor:UserId, case_id:CaseId, query:DeadlineResponsibleQuery, at:OffsetDateTime)->Result<DeadlineResponsiblePage,ApplicationError>;
        fn list(&self, actor:UserId, case_id:CaseId, query:DeadlineQuery, at:OffsetDateTime)->Result<DeadlinePage,ApplicationError>;
        fn get(&self, actor:UserId, case_id:CaseId, id:DeadlineId, revision:Option<DeadlineRevision>, at:OffsetDateTime)->Result<DeadlineDetail,ApplicationError>;
        fn history(&self, actor:UserId, case_id:CaseId, id:DeadlineId, query:DeadlineHistoryQuery, at:OffsetDateTime)->Result<DeadlineHistoryPage,ApplicationError>;
        fn prepare(&self, actor:UserId, case_id:CaseId, command:&DeadlineCommand)->Result<DeadlinePreparation,ApplicationError>;
        fn commit(&self, actor:UserId, prepared:PreparedDeadlineChange)->Result<DeadlineDetail,ApplicationError>;
    }
}
pub fn principal(role: Role) -> Principal {
    Principal {
        id: owner(),
        email: "owner@example.com".into(),
        role,
    }
}
pub fn identity(role: Role, calls: usize) -> MockIdentity {
    let mut identity = MockIdentity::new();
    identity
        .expect_authenticate()
        .withf(|token| token == "session")
        .times(calls)
        .returning(move |_| Ok(principal(role)));
    identity
}
pub fn service(store: MockStore, identity: MockIdentity) -> (DeadlineService, Arc<CountingClock>) {
    let clock = Arc::new(CountingClock::default());
    (
        DeadlineService::new(Arc::new(store), Arc::new(identity), hasher(), clock.clone()),
        clock,
    )
}
pub fn captured() -> DeadlineDetail {
    let (command, preparation) = fixture();
    detail(&prepare(command, preparation).unwrap())
}
pub fn history_entry(value: &DeadlineDetail) -> DeadlineHistoryEntry {
    DeadlineHistoryEntry::from_detail(hasher().as_ref(), value).unwrap()
}
pub fn list_query(limit: u32) -> DeadlineQuery {
    DeadlineQuery::new(limit, None, DeadlineStatusFilter::All).unwrap()
}
pub fn history_query(limit: u32) -> DeadlineHistoryQuery {
    DeadlineHistoryQuery::new(limit, None).unwrap()
}
pub fn attention_revision(base: &DeadlineDetail) -> DeadlineDetail {
    let (command, preparation) = followup(
        base,
        DeadlineChange::SetAttention {
            expected_revision: base.revision,
            attention: attention(),
            reason: evaluation::text("Declared action"),
        },
    );
    detail(&prepare(command, preparation).unwrap())
}
pub fn retirement_revision(base: &DeadlineDetail) -> DeadlineDetail {
    let (command, preparation) = followup(
        base,
        DeadlineChange::Retire {
            expected_revision: base.revision,
            reason: evaluation::text("No further tracking"),
        },
    );
    detail(&prepare(command, preparation).unwrap())
}
pub fn expect_operation(store: &mut MockStore, operation: usize) {
    let row = captured();
    match operation {
        0 => {
            store
                .expect_list()
                .times(1)
                .return_once(move |actor, case, _, _| {
                    assert_eq!(actor, owner());
                    assert_eq!(case, case_id());
                    Ok(DeadlinePage {
                        deadlines: vec![DeadlineOverview::from(&row)],
                        has_more: false,
                        next_after_id: None,
                    })
                });
        }
        1 => {
            store
                .expect_get()
                .times(1)
                .return_once(move |actor, case, id, revision, _| {
                    assert_eq!(actor, owner());
                    assert_eq!(case, case_id());
                    assert_eq!(id, row.id);
                    assert_eq!(revision, Some(row.revision));
                    Ok(row)
                });
        }
        2 => {
            store
                .expect_history()
                .times(1)
                .return_once(move |actor, case, id, _, _| {
                    assert_eq!(actor, owner());
                    assert_eq!(case, case_id());
                    assert_eq!(id, row.id);
                    Ok(DeadlineHistoryPage {
                        revisions: vec![history_entry(&row)],
                        has_more: false,
                        next_before_revision: None,
                    })
                });
        }
        3 | 4 => {
            let (_, preparation) = fixture();
            store
                .expect_prepare()
                .times(1)
                .return_once(move |actor, case, _| {
                    assert_eq!(actor, owner());
                    assert_eq!(case, case_id());
                    Ok(preparation)
                });
        }
        _ => panic!("invalid test operation"),
    }
}
pub fn run(service: &DeadlineService, operation: usize) -> Result<(), ApplicationError> {
    let (command, preparation) = fixture();
    let digest = prepare(command.clone(), preparation)
        .unwrap()
        .submission_digest();
    let id = command.deadline_id;
    match operation {
        0 => service
            .list("session", case_id(), list_query(20))
            .map(|_| ()),
        1 => service
            .get("session", case_id(), id, Some(DeadlineRevision::initial()))
            .map(|_| ()),
        2 => service
            .history("session", case_id(), id, history_query(20))
            .map(|_| ()),
        3 => service.prepare("session", case_id(), command).map(|_| ()),
        4 => service
            .submit("session", case_id(), command, digest)
            .map(|_| ()),
        _ => panic!("invalid test operation"),
    }
}
pub fn invalid(result: Result<(), ApplicationError>) {
    assert!(matches!(
        result,
        Err(ApplicationError::Deadline(
            DeadlineError::StoredInconsistent(_)
        ))
    ));
}
pub fn digest() -> Sha256Digest {
    captured().receipt.submission_digest
}
