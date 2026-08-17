use std::sync::Mutex;

use application::documents::{DocumentRecord, DocumentRepository};
use application::ApplicationError;
use domain::crypto::{DocumentId, DocumentVersion, Sha256Digest};

struct SingleRecordRepository {
    record: Mutex<Option<DocumentRecord>>,
}

impl DocumentRepository for SingleRecordRepository {
    fn insert(&self, record: DocumentRecord) -> Result<(), ApplicationError> {
        *self.record.lock().unwrap() = Some(record);
        Ok(())
    }

    fn replace(&self, record: DocumentRecord) -> Result<(), ApplicationError> {
        *self.record.lock().unwrap() = Some(record);
        Ok(())
    }

    fn find(&self, id: DocumentId) -> Result<Option<DocumentRecord>, ApplicationError> {
        Ok(self
            .record
            .lock()
            .unwrap()
            .clone()
            .filter(|record| record.id == id))
    }
}

fn record() -> DocumentRecord {
    DocumentRecord::pending(
        DocumentId::new(),
        DocumentVersion::initial(),
        "acta.txt".to_string(),
        Sha256Digest::from_array([3; 32]),
        vec![8; 60],
    )
    .unwrap()
}

#[test]
fn the_repository_port_is_object_safe_and_round_trips_a_record() {
    let repository: Box<dyn DocumentRepository> = Box::new(SingleRecordRepository {
        record: Mutex::new(None),
    });
    let stored = record();
    let id = stored.id;

    repository.insert(stored.clone()).unwrap();

    assert_eq!(repository.find(id).unwrap(), Some(stored));
}
