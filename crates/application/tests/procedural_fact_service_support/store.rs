use super::*;
#[allow(unused_imports)]
pub use crate::case_support::{identity, CountingClock, MockIdentity};
use application::{
    documents::{
        DocumentFormatBatch, DocumentFormatBatchValidator, DocumentRecord, StageDocumentFormat,
        StageSupportReadLimits,
    },
    ApplicationError,
};
use domain::{
    clock::OffsetDateTime,
    crypto::{DocumentId, DocumentVersion},
};
use mockall::mock;
use std::sync::atomic::{AtomicUsize, Ordering};
mock! {
    pub Store {}
    impl ProceduralFactStore for Store {
        fn list_resolutions(&self, actor:UserId, case_id:CaseId, query:ResolutionQuery, at:OffsetDateTime)->Result<ResolutionPage,ApplicationError>;
        fn list_notifications(&self, actor:UserId, case_id:CaseId, resolution_id:ResolutionId, query:NotificationQuery, at:OffsetDateTime)->Result<NotificationPage,ApplicationError>;
        fn get(&self, actor:UserId, case_id:CaseId, target:FactTarget, revision:Option<FactRevision>, at:OffsetDateTime)->Result<FactDetail,ApplicationError>;
        fn history(&self, actor:UserId, case_id:CaseId, target:FactTarget, query:FactHistoryQuery, at:OffsetDateTime)->Result<FactHistoryPage,ApplicationError>;
        fn prepare(&self, actor:UserId, case_id:CaseId, command:&ProceduralFactCommand, limits:&StageSupportReadLimits)->Result<FactPreparation,ApplicationError>;
        fn commit(&self, actor:UserId, case_id:CaseId, prepared:PreparedFactChange)->Result<FactDetail,ApplicationError>;
    }
}
#[derive(Default)]
pub struct Validator {
    pub calls: AtomicUsize,
    pub items: AtomicUsize,
    pub failure: bool,
}
impl DocumentFormatBatchValidator for Validator {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.items.store(batch.inputs().len(), Ordering::SeqCst);
        if self.failure {
            return Err(ApplicationError::StageSupportFormatRejected);
        }
        Ok(vec![StageDocumentFormat::Pdf; batch.inputs().len()])
    }
}
pub fn service(
    store: MockStore,
    identity: MockIdentity,
    validator: Arc<Validator>,
) -> (ProceduralFactService, Arc<CountingClock>) {
    let clock = Arc::new(CountingClock::default());
    (
        ProceduralFactService::new(
            Arc::new(store),
            Arc::new(identity),
            Arc::new(crate::crypto::processor()),
            validator,
            hasher(),
            clock.clone(),
        ),
        clock,
    )
}
pub fn record() -> DocumentRecord {
    crate::crypto::processor()
        .prepare_version(
            DocumentId::new(),
            DocumentVersion::initial(),
            "source.pdf",
            b"%PDF-declared source",
        )
        .unwrap()
}
pub fn with_support(record: &DocumentRecord) -> ResolutionValues {
    let base = values();
    ResolutionValues::new(ResolutionValuesInput {
        class: base.class().clone(),
        subtype: None,
        issuer: base.issuer().clone(),
        issued_at: base.issued_at(),
        summary: base.summary().clone(),
        provenance: FactProvenance::ExternalReference {
            reference: text("Declared written source"),
            support: Some(FactEvidence::new(
                domain::crypto::DocumentVersionRef {
                    id: record.id,
                    version: record.version,
                },
                record.digest,
                FactLabel::new("Page 1").unwrap(),
            )),
        },
    })
}
pub fn record_command(values: ResolutionValues) -> ProceduralFactCommand {
    ProceduralFactCommand::Resolution(ResolutionCommand::new(
        FactOperationId::new(),
        ResolutionId::new(),
        FactChange::record(values),
    ))
}
