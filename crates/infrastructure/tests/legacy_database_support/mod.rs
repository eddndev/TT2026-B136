use std::fs;
use std::path::PathBuf;

use application::documents::{DocumentRecord, DocumentRepository};
use application::vault::encrypt_with_ports;
use domain::audit::{AuditLog, ChainedEvent};
use domain::cases::CaseId;
use domain::clock::OffsetDateTime;
use domain::crypto::{DocumentHasher, DocumentId, DocumentVersion};
use infrastructure::{
    EnvelopeKeyManager, FileAuditLog, FileDocumentRepository, LegacyImport, PostgresUserRepository,
    RingAesGcmCipher, RingSha256Hasher,
};
use postgres::{Client, NoTls};
use uuid::Uuid;

pub const KEK: [u8; 32] = [3; 32];
pub const PLAINTEXT: &[u8] = b"unaltered legacy document bytes";

pub struct Source {
    pub dir: tempfile::TempDir,
    pub mapping: PathBuf,
    pub id: DocumentId,
    pub case_id: CaseId,
    pub vault: Vec<u8>,
    pub entries: Vec<ChainedEvent>,
}

impl Source {
    pub fn new() -> Self {
        Self::with_version(DocumentVersion::initial())
    }

    pub fn with_version(version: DocumentVersion) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let id = DocumentId::new();
        let case_id = CaseId::from_uuid(Uuid::new_v4());
        let vault = encrypt_with_ports(
            &RingAesGcmCipher::new(),
            &EnvelopeKeyManager::new(),
            &KEK,
            PLAINTEXT,
            id,
            version,
        )
        .unwrap();
        let repository = FileDocumentRepository::new(dir.path().join("documents")).unwrap();
        repository
            .insert(
                DocumentRecord::pending(
                    id,
                    version,
                    "legacy.txt".into(),
                    RingSha256Hasher::new().hash_bytes(PLAINTEXT),
                    vault.clone(),
                )
                .unwrap(),
            )
            .unwrap();
        let mut log = FileAuditLog::new(dir.path().join("audit.jsonl"), RingSha256Hasher::new());
        log.append(
            "owner@example.test",
            "identity.login_succeeded",
            "owner@example.test",
            OffsetDateTime::from_unix_timestamp_nanos(1_735_689_600_123_456_789).unwrap(),
        )
        .unwrap();
        log.append(
            "owner@example.test",
            "document.uploaded",
            &format!("document:{id}"),
            OffsetDateTime::from_unix_timestamp_nanos(1_735_689_601_987_654_321).unwrap(),
        )
        .unwrap();
        let entries = log.load_all().unwrap();
        let mapping = dir.path().join("mapping.json");
        fs::write(
            &mapping,
            serde_json::to_vec(&serde_json::json!({"documents":[{
                "document_id": id.to_string(), "case_id": case_id.to_string()
            }]}))
            .unwrap(),
        )
        .unwrap();
        Self {
            dir,
            mapping,
            id,
            case_id,
            vault,
            entries,
        }
    }

    pub fn inspect(&self) -> LegacyImport {
        LegacyImport::inspect(self.dir.path(), &self.mapping, &KEK).unwrap()
    }

    pub fn document_path(&self) -> PathBuf {
        self.dir.path().join(format!("documents/{}.json", self.id))
    }

    pub fn audit_path(&self) -> PathBuf {
        self.dir.path().join("audit.jsonl")
    }

    pub fn assert_unmarked(&self) {
        assert!(!self.dir.path().join("documents/.migrated").exists());
        assert!(!self.dir.path().join("audit.jsonl.migrated").exists());
    }
}

pub struct Database {
    pub url: String,
    pub client: Client,
    control: Client,
    schema: String,
    runtime_role: Option<String>,
}

impl Database {
    pub fn new() -> Option<Self> {
        let base = std::env::var("CASE_TEST_DATABASE_URL").ok()?;
        let schema = format!("legacy_{}", Uuid::new_v4().simple());
        let mut control = Client::connect(&base, NoTls).unwrap();
        control
            .batch_execute(&format!("CREATE SCHEMA {schema}"))
            .unwrap();
        let url = if base.starts_with("postgres://") || base.starts_with("postgresql://") {
            let separator = if base.contains('?') { '&' } else { '?' };
            format!("{base}{separator}options=-csearch_path%3D{schema}")
        } else {
            format!("{base} options='-c search_path={schema}'")
        };
        PostgresUserRepository::connect(&url).unwrap();
        let client = Client::connect(&url, NoTls).unwrap();
        Some(Self {
            url,
            client,
            control,
            schema,
            runtime_role: None,
        })
    }

    pub fn restricted_url(&mut self) -> String {
        let role = format!("legacy_runtime_{}", Uuid::new_v4().simple());
        self.control
            .batch_execute(&format!(
                "CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEROLE PASSWORD 'runtime-test-only'"
            ))
            .unwrap();
        infrastructure::initialize_database(&self.url, &role).unwrap();
        let url = if let Ok(mut parsed) = reqwest::Url::parse(&self.url) {
            parsed.set_username(&role).unwrap();
            parsed.set_password(Some("runtime-test-only")).unwrap();
            parsed.to_string()
        } else {
            format!("{} user='{role}' password='runtime-test-only'", self.url)
        };
        self.runtime_role = Some(role);
        url
    }

    pub fn seed_case(&mut self, id: CaseId) {
        let owner = Uuid::new_v4();
        self.client.execute(
            "INSERT INTO users(id,email,password_hash,role,protected_totp_secret,recovery_codes)
             VALUES($1,$2,'fixture','owner','\\x00','{}')",
            &[&owner, &format!("{owner}@example.test")],
        ).unwrap();
        self.client.execute(
            "INSERT INTO cases(id,title,reference,created_by,required_initial_revision) VALUES($1,'Legacy case','LEGACY',$2,NULL)",
            &[&id.as_uuid(), &owner],
        ).unwrap();
    }

    pub fn counts(&mut self) -> (i64, i64, i64) {
        let row = self
            .client
            .query_one(
                "SELECT (SELECT COUNT(*) FROM documents), (SELECT COUNT(*) FROM audit_events),
             (SELECT COUNT(*) FROM migration_receipts)",
                &[],
            )
            .unwrap();
        (row.get(0), row.get(1), row.get(2))
    }

    pub fn stored_state(&mut self) -> serde_json::Value {
        self.client.query_one(
            "SELECT jsonb_build_object(
                'documents', (SELECT jsonb_agg(to_jsonb(d) ORDER BY id) FROM documents d),
                'audit', (SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a),
                'receipts', (SELECT jsonb_agg(to_jsonb(r) ORDER BY fingerprint) FROM migration_receipts r)
            )", &[],
        ).unwrap().get(0)
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        let _ = self
            .control
            .batch_execute(&format!("DROP SCHEMA {} CASCADE", self.schema));
        if let Some(role) = &self.runtime_role {
            let _ = self.control.batch_execute(&format!("DROP ROLE {role}"));
        }
    }
}
