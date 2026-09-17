#![allow(dead_code)]

use application::case_stages::*;
use application::cases::*;
use application::documents::{
    CaseDocumentStore, DocumentFormatBatch, DocumentFormatBatchValidator, DocumentProcessor,
    DocumentRecord,
};
use application::identity::*;
use application::ApplicationError;
use domain::cases::{CaseId, CaseMetadata};
use domain::clock::{Clock, OffsetDateTime};
use domain::identity::{Permission, Role, UserId};
use infrastructure::{
    EnvelopeKeyManager, PostgresCaseDocumentStore, PostgresCaseStageStore, RingAesGcmCipher,
    RingSha256Hasher,
};
use std::sync::Arc;
use zeroize::Zeroizing;

pub use super::case_administration_support::Fixture;

pub fn processor() -> Arc<DocumentProcessor> {
    let mut ports = super::crypto::processor_ports();
    ports.hasher = Box::new(RingSha256Hasher);
    ports.cipher = Box::new(RingAesGcmCipher::new());
    ports.keys = Box::new(EnvelopeKeyManager::new());
    Arc::new(
        DocumentProcessor::new(
            ports,
            super::crypto::evidence_material(),
            Zeroizing::new(vec![0x44; 32]),
        )
        .unwrap(),
    )
}

pub fn creation(nuc: &str) -> PenalCaseCreation {
    PenalCaseCreation::new(
        CaseMetadata::new("Stage case", "REF").unwrap(),
        PenalCaseProfile::new(nuc, "Authority", nuc, "Court", &["Offense"], None, None).unwrap(),
    )
}

pub fn complete(db: &Fixture) {
    db.store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::Unrevised,
            creation(&db.case.to_string())
                .into_values()
                .editable()
                .clone(),
            db.at,
        )
        .unwrap();
}

pub fn store(db: &Fixture) -> Arc<PostgresCaseStageStore> {
    Arc::new(PostgresCaseStageStore::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap())
}

pub fn upload(db: &Fixture, case: CaseId, name: &str) -> DocumentRecord {
    let record = processor().prepare(name, b"documentary support").unwrap();
    PostgresCaseDocumentStore::open(&db.runtime_url)
        .unwrap()
        .insert(db.owner, case, record.clone(), db.at)
        .unwrap();
    record
}

pub fn reference(record: &DocumentRecord) -> StageSupportRef {
    StageSupportRef::new(
        application::documents::DocumentVersionRef {
            id: record.id,
            version: record.version,
        },
        record.digest,
    )
}

pub fn adoption(db: &Fixture, record: &DocumentRecord, stage: CaseStage) -> StageAdoption {
    StageAdoption::new(
        stage,
        DeclaredStageTime::instant(db.at).unwrap(),
        StageNote::new("Previously known case").unwrap(),
        reference(record),
    )
}

pub struct FixedClock(pub OffsetDateTime);
impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        self.0
    }
}

pub struct FormatCheck(pub Option<Box<dyn Fn() + Send + Sync>>);
impl DocumentFormatBatchValidator for FormatCheck {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        if let Some(callback) = &self.0 {
            callback();
        }
        Ok(vec![StageDocumentFormat::Pdf; batch.inputs().len()])
    }
}

pub struct TestIdentity(pub Principal);
impl IdentityWorkflow for TestIdentity {
    fn authenticate(&self, _: &str) -> Result<Principal, ApplicationError> {
        Ok(self.0.clone())
    }
    fn authorize(&self, _: &str, _: Permission) -> Result<Principal, ApplicationError> {
        unreachable!()
    }
    fn bootstrap_owner(&self, _: &str, _: &str) -> Result<EnrollmentResult, ApplicationError> {
        unreachable!()
    }
    fn create_user(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: Role,
    ) -> Result<EnrollmentResult, ApplicationError> {
        unreachable!()
    }
    fn start_login(&self, _: &str, _: &str) -> Result<LoginChallenge, ApplicationError> {
        unreachable!()
    }
    fn complete_totp(&self, _: &str, _: &str) -> Result<SessionResult, ApplicationError> {
        unreachable!()
    }
    fn complete_recovery(&self, _: &str, _: &str) -> Result<SessionResult, ApplicationError> {
        unreachable!()
    }
    fn logout(&self, _: &str) -> Result<(), ApplicationError> {
        unreachable!()
    }
}

pub fn service(db: &Fixture, actor: UserId, role: Role, format: FormatCheck) -> CaseStageService {
    CaseStageService::new(
        store(db),
        Arc::new(TestIdentity(Principal {
            id: actor,
            email: "session@example.test".into(),
            role,
        })),
        processor(),
        Arc::new(format),
        Arc::new(FixedClock(db.at)),
    )
}

pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object('stages',(SELECT jsonb_agg(to_jsonb(s) ORDER BY case_id,revision) FROM case_stage_revisions s),'initial',(SELECT jsonb_agg(to_jsonb(s) ORDER BY case_id) FROM case_initial_stage_registrations s),'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))",&[]).unwrap().get(0)
}

pub fn row_value(db: &Fixture, record: &DocumentRecord) -> serde_json::Value {
    serde_json::json!({
        "case_id":db.case.to_string(),"revision":1,"change_kind":"adoption","stage":"investigation",
        "administration_revision":1,"act_precision":"instant","act_seconds":db.at.unix_timestamp(),
        "act_nanoseconds":db.at.nanosecond(),"act_offset_seconds":0,"reason":"Known stage",
        "support_id":record.id.to_string(),"support_version":record.version.get(),
        "support_digest":format!("\\x{}",record.digest.to_hex()),"support_name":record.name,
        "support_format":"pdf","support_policy":"pdf_docx_v1",
        "recorded_at_seconds":db.at.unix_timestamp(),"recorded_at_nanoseconds":db.at.nanosecond(),
        "recorded_by":db.owner.to_string(),"recorded_by_email":"owner@example.test"
    })
}
pub fn signed_value(
    client: &mut impl postgres::GenericClient,
    value: &serde_json::Value,
) -> serde_json::Value {
    client.query_one("SELECT jsonb_set($1,'{values_digest}',to_jsonb('\\x'||encode(sha256(case_stage_values_bytes(jsonb_populate_record(NULL::case_stage_revisions,$1))),'hex')))",&[value]).unwrap().get(0)
}
pub const INSERT: &str = "INSERT INTO case_stage_revisions SELECT (jsonb_populate_record(NULL::case_stage_revisions,$1)).*";
