use std::sync::Arc;

use application::{
    document_content::DocumentContentService,
    document_integrity::{
        DocumentIntegrityIncident, DocumentIntegrityIncidentId, DocumentIntegrityObservation,
        DocumentIntegrityPage, DocumentIntegrityQuery, DocumentIntegrityReceipt,
        DocumentIntegrityStore,
    },
    documents::{DocumentProcessor, DocumentRecord},
    identity::Principal,
    ApplicationError,
};
use domain::{
    clock::OffsetDateTime,
    crypto::DocumentVersionRef,
    identity::{Role, UserId},
};
use mockall::mock;

use super::case_document_support::{MockIdentity, MockStore};

mock! {
    pub Incidents {}
    impl DocumentIntegrityStore for Incidents {
        fn record_rejection(&self, observation: &DocumentIntegrityObservation) -> Result<DocumentIntegrityReceipt, ApplicationError>;
        fn list(&self, actor: UserId, query: DocumentIntegrityQuery, at: OffsetDateTime) -> Result<DocumentIntegrityPage, ApplicationError>;
        fn get(&self, actor: UserId, id: DocumentIntegrityIncidentId, at: OffsetDateTime) -> Result<DocumentIntegrityIncident, ApplicationError>;
    }
}

pub fn principal() -> Principal {
    Principal {
        id: UserId::new(),
        email: "reader@example.test".into(),
        role: Role::Litigator,
    }
}

pub fn identity(principal: Principal, calls: usize) -> MockIdentity {
    let mut identity = MockIdentity::new();
    identity
        .expect_authenticate()
        .times(calls)
        .withf(|token| token == "session")
        .returning(move |_| Ok(principal.clone()));
    identity
}

pub fn reference(record: &DocumentRecord) -> DocumentVersionRef {
    DocumentVersionRef {
        id: record.id,
        version: record.version,
    }
}

pub fn service(
    store: MockStore,
    identity: MockIdentity,
    processor: DocumentProcessor,
    incidents: MockIncidents,
) -> DocumentContentService {
    DocumentContentService::new(
        Arc::new(store),
        Arc::new(identity),
        Arc::new(processor),
        Arc::new(super::crypto::TestClock),
        Arc::new(incidents),
    )
}

pub fn receipt(observation: &DocumentIntegrityObservation) -> DocumentIntegrityReceipt {
    DocumentIntegrityReceipt {
        incident_id: DocumentIntegrityIncidentId::new(),
        observation_id: observation.observation_id,
        recorded_at: observation.detected_at,
    }
}
