//! Workflows composed by the full API router over one shared HTTP runtime.
use std::sync::Arc;

use application::case_stages::CaseStageWorkflow;
use application::cases::CaseWorkflow;
use application::participants::ParticipantWorkflow;

use crate::PrecautionaryWorkflows;

/// Identity evidence and case workflows injected into the shared HTTP runtime.
pub struct CaseWorkflows {
    pub precautionary: PrecautionaryWorkflows,
    pub owner_certificates: Arc<application::identity::owner_certificates::OwnerCertificateService>,
    pub members: Arc<dyn application::members::MemberWorkflow>,
    pub cases: Arc<dyn CaseWorkflow>,
    pub participants: Arc<dyn ParticipantWorkflow>,
    pub stages: Arc<dyn CaseStageWorkflow>,
    pub typed: Arc<dyn application::typed_participants::TypedParticipantWorkflow>,
    pub hearings: Arc<dyn application::hearings::HearingWorkflow>,
    pub hearing_results: Arc<dyn application::hearing_results::HearingResultWorkflow>,
    pub hearing_derived_deadlines:
        Arc<dyn application::hearing_derived_deadlines::HearingDerivedDeadlineWorkflow>,
    pub procedural_facts: Arc<dyn application::procedural_facts::ProceduralFactWorkflow>,
    pub procedural_resources:
        Arc<dyn application::procedural_resources::ProceduralResourceWorkflow>,
    pub resource_activities: Arc<dyn application::resource_activities::ResourceActivityWorkflow>,
    pub resource_deadlines: Arc<dyn application::resource_deadlines::ResourceDeadlineWorkflow>,
    pub resource_hearings: Arc<dyn application::resource_hearings::ResourceHearingWorkflow>,
    pub resource_hearing_reads:
        Arc<dyn application::resource_hearings::ResourceHearingReadWorkflow>,
    pub deadlines: Arc<dyn application::deadlines::DeadlineWorkflow>,
    pub agenda: Arc<dyn application::agenda::AgendaWorkflow>,
    pub dashboard: Arc<dyn application::dashboard::DashboardWorkflow>,
    pub audit_events: Arc<dyn application::audit_query::AuditEventWorkflow>,
    pub case_reports: Arc<dyn application::case_reports::CaseReportWorkflow>,
    pub alerts: Arc<dyn application::alerts::AlertWorkflow>,
    pub document_content: Arc<dyn application::document_content::DocumentContentWorkflow>,
    pub document_integrity: Arc<dyn application::document_integrity::DocumentIntegrityWorkflow>,
    pub evidence_trust: Arc<dyn application::evidence_trust::EvidenceTrustWorkflow>,
}
