use application::{
    agenda::*, audit_query::*, document_content::*, document_integrity::*, hearings::*,
    resource_activities::ResourceId, resource_deadlines::*, ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentVersionRef, Sha256Digest},
};

pub struct Unused;

impl AgendaWorkflow for Unused {
    fn list(&self, _: &str, _: AgendaQuery) -> Result<AgendaPage, ApplicationError> {
        unreachable!("composition test does not query the agenda")
    }
}
impl AuditEventWorkflow for Unused {
    fn read(&self, _: &str, _: AuditEventQuery) -> Result<AuditEventPage, ApplicationError> {
        unreachable!("composition test does not query audit events")
    }
}
impl DocumentContentWorkflow for Unused {
    fn content_version(
        &self,
        _: &str,
        _: CaseId,
        _: DocumentVersionRef,
    ) -> Result<DocumentContent, ApplicationError> {
        unreachable!("composition test does not release document content")
    }
}
impl DocumentIntegrityWorkflow for Unused {
    fn list(
        &self,
        _: &str,
        _: DocumentIntegrityQuery,
    ) -> Result<DocumentIntegrityPage, ApplicationError> {
        unreachable!("composition test does not list integrity incidents")
    }
    fn get(
        &self,
        _: &str,
        _: DocumentIntegrityIncidentId,
    ) -> Result<DocumentIntegrityIncident, ApplicationError> {
        unreachable!("composition test does not read integrity incidents")
    }
}
impl ResourceDeadlineWorkflow for Unused {
    fn prepare(
        &self,
        _: &str,
        _: CaseId,
        _: ResourceId,
        _: ResourceDeadlineCommand,
    ) -> Result<ResourceDeadlineDraft, ApplicationError> {
        unreachable!("composition test does not prepare resource deadlines")
    }
    fn submit(
        &self,
        _: &str,
        _: CaseId,
        _: ResourceId,
        _: ResourceDeadlineCommand,
        _: Sha256Digest,
    ) -> Result<ResourceDeadlineResult, ApplicationError> {
        unreachable!("composition test does not submit resource deadlines")
    }
}
impl HearingWorkflow for Unused {
    fn context(&self, _: &str, _: CaseId) -> Result<HearingCaseContext, ApplicationError> {
        unreachable!("composition test does not read hearing context")
    }
    fn list(&self, _: &str, _: CaseId, _: HearingQuery) -> Result<HearingPage, ApplicationError> {
        unreachable!("composition test does not list hearings")
    }
    fn get(
        &self,
        _: &str,
        _: CaseId,
        _: HearingId,
        _: Option<HearingRevision>,
    ) -> Result<HearingDetail, ApplicationError> {
        unreachable!("composition test does not read hearings")
    }
    fn history(
        &self,
        _: &str,
        _: CaseId,
        _: HearingId,
        _: HearingHistoryQuery,
    ) -> Result<HearingHistoryPage, ApplicationError> {
        unreachable!("composition test does not read hearing history")
    }
    fn agenda(
        &self,
        _: &str,
        _: HearingAgendaQuery,
    ) -> Result<HearingAgendaPage, ApplicationError> {
        unreachable!("composition test does not query hearing agenda")
    }
    fn prepare(
        &self,
        _: &str,
        _: CaseId,
        _: HearingCommand,
    ) -> Result<HearingDraft, ApplicationError> {
        unreachable!("composition test does not prepare hearings")
    }
    fn submit(
        &self,
        _: &str,
        _: CaseId,
        _: HearingCommand,
        _: Sha256Digest,
    ) -> Result<HearingDetail, ApplicationError> {
        unreachable!("composition test does not submit hearings")
    }
}

impl application::hearing_derived_deadlines::HearingDerivedDeadlineWorkflow for Unused {
    fn prepare(
        &self,
        _: &str,
        _: CaseId,
        _: application::hearing_derived_deadlines::HearingDerivedDeadlineCommand,
    ) -> Result<
        application::hearing_derived_deadlines::HearingDerivedDeadlineReview,
        ApplicationError,
    > {
        unreachable!("composition test does not prepare hearing consequences")
    }
    fn submit(
        &self,
        _: &str,
        _: CaseId,
        _: application::hearing_derived_deadlines::HearingDerivedDeadlineCommand,
        _: Sha256Digest,
    ) -> Result<
        application::hearing_derived_deadlines::HearingDerivedDeadlineRecord,
        ApplicationError,
    > {
        unreachable!("composition test does not submit hearing consequences")
    }
}
