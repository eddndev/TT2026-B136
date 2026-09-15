#[path = "../version_database_support/mod.rs"]
mod version_database_support;

use application::documents::{DocumentMetadata, DocumentRecord};
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion, Sha256Digest};
use domain::identity::UserId;
use infrastructure::{initialize_database, PostgresCaseDocumentStore};
use time::OffsetDateTime;
use version_database_support::Database;

pub struct Fixture {
    pub db: Database,
    pub runtime_url: String,
    pub role: String,
    pub owner: UserId,
    pub case: CaseId,
    pub at: OffsetDateTime,
}

impl Fixture {
    pub fn new() -> Option<Self> {
        let mut db = Database::old_schema()?;
        let role = format!("metadata_runtime_{}", uuid::Uuid::new_v4().simple());
        db.control
            .batch_execute(&format!(
                "CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEROLE PASSWORD 'runtime-test-only'"
            ))
            .unwrap();
        initialize_database(&db.url, &role).unwrap();
        let case = db.seed_case();
        let owner = UserId::from_uuid(
            db.client
                .query_one("SELECT created_by FROM cases WHERE id=$1", &[&case])
                .unwrap()
                .get(0),
        );
        let mut url = reqwest::Url::parse(&db.url).unwrap();
        url.set_username(&role).unwrap();
        url.set_password(Some("runtime-test-only")).unwrap();
        Some(Self {
            db,
            runtime_url: url.to_string(),
            role,
            owner,
            case: CaseId::from_uuid(case),
            at: OffsetDateTime::from_unix_timestamp_nanos(1_735_689_600_123_456_789).unwrap(),
        })
    }

    pub fn store(&self) -> PostgresCaseDocumentStore {
        PostgresCaseDocumentStore::open(&self.runtime_url).unwrap()
    }

    pub fn snapshot(&mut self) -> serde_json::Value {
        self.db.client.query_one("SELECT jsonb_build_object(
            'series',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM document_series s),
            'documents',(SELECT jsonb_agg(to_jsonb(d) ORDER BY id,version) FROM documents d),
            'metadata',(SELECT jsonb_agg(to_jsonb(m) ORDER BY document_id,metadata_revision) FROM document_metadata_revisions m),
            'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))",&[]).unwrap().get(0)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.db.control.batch_execute(&format!(
            "DROP SCHEMA {} CASCADE; DROP ROLE {}",
            self.db.schema, self.role
        ));
    }
}

pub fn document() -> DocumentRecord {
    DocumentRecord::pending(
        DocumentId::new(),
        DocumentVersion::initial(),
        "evidence.txt".into(),
        Sha256Digest::from_array([3; 32]),
        vec![8; 80],
    )
    .unwrap()
}

pub fn metadata(kind: &str, class: &str, tags: &[&str]) -> DocumentMetadata {
    DocumentMetadata::new(
        Some(kind),
        Some(class),
        &tags.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    )
    .unwrap()
}
