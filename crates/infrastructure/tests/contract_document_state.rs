//! What document reads, seals and the dashboard see of a contracted
//! series: docs/adr/0086-bilateral-contract-signing.md.
#![allow(dead_code)]
mod case_administration_support;
mod contract_service_support;
mod contract_storage_support;
mod declaration_fixture;
#[path = "document_store_support/mod.rs"]
mod document_store_support;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use application::{
    contracts::{ContractState, ContractWorkflowError},
    dashboard::DashboardStore,
    documents::{CaseDocumentStore, DocumentQuery, VersionQuery},
    ApplicationError,
};
use contract_service_support::{Live, DAY};
use domain::clock::{Clock as _, OffsetDateTime};
use infrastructure::{PostgresCaseDocumentStore, PostgresDashboardStore, RingSha256Hasher};
use std::sync::Arc;

struct At(OffsetDateTime);
impl domain::clock::Clock for At {
    fn now(&self) -> OffsetDateTime {
        self.0
    }
}

fn listed(live: &Live, sealed: Option<bool>, at: OffsetDateTime) -> Vec<Option<ContractState>> {
    PostgresCaseDocumentStore::open(&live.test.individual.db.runtime_url)
        .unwrap()
        .list(
            live.firm().id,
            live.test.individual.db.case,
            DocumentQuery::new(10, 0, None, sealed).unwrap(),
            at,
        )
        .unwrap()
        .documents
        .into_iter()
        .map(|overview| overview.content.contract_state)
        .collect()
}

fn pending(live: &Live, at: OffsetDateTime) -> u64 {
    PostgresDashboardStore::open(
        &live.test.individual.db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(At(at)),
    )
    .unwrap()
    .read(live.firm().id)
    .unwrap()
    .pending_contracts
}

#[test]
fn an_open_request_freezes_its_version_until_its_validity_is_exhausted() {
    let Some(live) = Live::new() else { return };
    let live = live.requested();
    let now = live.clock.now();
    assert_eq!(listed(&live, None, now), [Some(ContractState::Signing)]);
    assert!(listed(&live, Some(false), now).is_empty());
    assert_eq!(pending(&live, now), 1);
    let docs = PostgresCaseDocumentStore::open(&live.test.individual.db.runtime_url).unwrap();
    let history = docs
        .history(
            live.firm().id,
            live.test.individual.db.case,
            live.test.individual.record.id,
            VersionQuery::new(10, None).unwrap(),
            now,
        )
        .unwrap();
    assert_eq!(
        history.versions[0].contract_state,
        Some(ContractState::Signing)
    );
    let mut sealed = live.test.individual.record.clone();
    sealed.seal(document_store_support::evidence()).unwrap();
    assert!(matches!(
        docs.seal(live.firm().id, live.test.individual.db.case, sealed, now),
        Err(ApplicationError::Contract(
            ContractWorkflowError::VersionFrozen
        ))
    ));

    let later = now + time::Duration::seconds(7 * DAY);
    assert_eq!(listed(&live, None, later), [Some(ContractState::Draft)]);
    assert_eq!(
        listed(&live, Some(false), later),
        [Some(ContractState::Draft)]
    );
    assert_eq!(pending(&live, later), 1, "a draft is still pending");
}
