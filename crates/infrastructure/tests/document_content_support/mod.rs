#![allow(dead_code)]

#[path = "../case_administration_support/mod.rs"]
mod administration;

use application::document_integrity::{
    DocumentIntegrityFailure, DocumentIntegrityObservation, DocumentIntegrityObservationId,
};
use application::documents::{CaseDocumentStore, DocumentRecord};
use application::vault::encrypt_with_ports;
use domain::crypto::{DocumentHasher, DocumentId, DocumentVersion};
use infrastructure::{
    EnvelopeKeyManager, PostgresCaseDocumentStore, RingAesGcmCipher, RingSha256Hasher,
};
use uuid::Uuid;

pub struct Fixture {
    pub db: administration::Fixture,
    pub store: PostgresCaseDocumentStore,
    pub record: DocumentRecord,
}

impl Fixture {
    pub fn new() -> Option<Self> {
        let db = administration::Fixture::new()?;
        let store = PostgresCaseDocumentStore::open(&db.runtime_url).unwrap();
        let record = record(
            DocumentId::new(),
            DocumentVersion::initial(),
            b"original content",
        );
        store
            .insert(db.owner, db.case, record.clone(), db.at)
            .unwrap();
        Some(Self { db, store, record })
    }

    pub fn observation(&self) -> DocumentIntegrityObservation {
        DocumentIntegrityObservation {
            observation_id: DocumentIntegrityObservationId::from_uuid(Uuid::new_v4()),
            requester: self.db.owner,
            case_id: self.db.case,
            record: self.record.clone(),
            failure: DocumentIntegrityFailure::DigestMismatch,
            detected_at: self.db.at,
        }
    }

    pub fn content_events(&mut self) -> i64 {
        self.db
            .admin
            .query_one(
                "SELECT count(*) FROM audit_events WHERE action='document.content_authorized'",
                &[],
            )
            .unwrap()
            .get(0)
    }
}

pub fn record(id: DocumentId, version: DocumentVersion, bytes: &[u8]) -> DocumentRecord {
    let vault = encrypt_with_ports(
        &RingAesGcmCipher::new(),
        &EnvelopeKeyManager::new(),
        &[0x39; 32],
        bytes,
        id,
        version,
    )
    .unwrap();
    DocumentRecord::pending(
        id,
        version,
        "content.txt".into(),
        RingSha256Hasher::new().hash_bytes(bytes),
        vault,
    )
    .unwrap()
}
