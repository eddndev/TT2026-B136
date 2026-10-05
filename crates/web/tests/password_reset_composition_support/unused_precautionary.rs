use super::unused::Unused;
use application::{measure_corrections::*, precautionary_hearings::*, ApplicationError};
use domain::{
    cases::CaseId,
    precautionary_hearings::{
        MeasureId, PrecautionaryHearingId, PrecautionaryHearingOperationId,
        PrecautionaryHearingRevision, PrecautionaryMeasureRef,
    },
    precautionary_measures::MeasureCorrectionOperationId,
};

impl PrecautionaryContextReadWorkflow for Unused {
    fn get(&self, _: &str, _: CaseId) -> Result<PrecautionaryContext, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }
}

impl PrecautionaryHearingRecordWorkflow for Unused {
    fn prepare(
        &self,
        _: &str,
        _: CaseId,
        _: PrecautionaryHearingCommand,
    ) -> Result<PrecautionaryHearingReview, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }

    fn submit(
        &self,
        _: &str,
        _: CaseId,
        _: PrecautionaryHearingCommand,
        _: PrecautionaryHearingConfirmation,
    ) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }
}

impl PrecautionaryHearingRecordReadWorkflow for Unused {
    fn list(
        &self,
        _: &str,
        _: CaseId,
        _: PrecautionaryHearingReadQuery,
    ) -> Result<PrecautionaryHearingRecordPage, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }

    fn get(
        &self,
        _: &str,
        _: CaseId,
        _: PrecautionaryHearingId,
        _: Option<PrecautionaryHearingRevision>,
    ) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }

    fn get_operation(
        &self,
        _: &str,
        _: CaseId,
        _: PrecautionaryHearingOperationId,
    ) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }
}

impl domain::crypto::DocumentHasher for Unused {
    fn hash_bytes(&self, _: &[u8]) -> domain::crypto::Sha256Digest {
        unreachable!("denied precautionary workflows must not project context")
    }
    fn hash_stream(
        &self,
        _: &mut dyn std::io::Read,
    ) -> Result<domain::crypto::Sha256Digest, domain::DomainError> {
        unreachable!("denied precautionary workflows must not stream content")
    }
}

impl MeasureAdministrativeWorkflow for Unused {
    fn prepare(
        &self,
        _: &str,
        _: CaseId,
        _: MeasureAdministrativeCommand,
    ) -> Result<MeasureAdministrativeReview, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }

    fn submit(
        &self,
        _: &str,
        _: CaseId,
        _: MeasureAdministrativeCommand,
        _: MeasureAdministrativeConfirmation,
    ) -> Result<MeasureAdministrativeStoredOperation, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }
}

impl MeasureAdministrativeReadWorkflow for Unused {
    fn list(
        &self,
        _: &str,
        _: CaseId,
        _: MeasureAdministrativeReadQuery,
    ) -> Result<MeasureAdministrativePage, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }

    fn get_operation(
        &self,
        _: &str,
        _: CaseId,
        _: MeasureCorrectionOperationId,
    ) -> Result<MeasureAdministrativeStoredOperation, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }
}

impl MeasureRecordReadWorkflow for Unused {
    fn list(
        &self,
        _: &str,
        _: CaseId,
        _: MeasureRecordReadQuery,
    ) -> Result<MeasureRecordPage, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }

    fn get(
        &self,
        _: &str,
        _: CaseId,
        _: MeasureId,
    ) -> Result<MeasureRecordDetail, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }

    fn exact(
        &self,
        _: &str,
        _: CaseId,
        _: PrecautionaryMeasureRef,
    ) -> Result<MeasureRecordDetail, ApplicationError> {
        Err(ApplicationError::PermissionDenied)
    }
}

impl application::precautionary_measures::MeasureDecisionRecordWorkflow for Unused {
    fn prepare(
        &self,
        _: &str,
        _: CaseId,
        _: application::precautionary_measures::MeasureDecisionCommand,
    ) -> Result<application::precautionary_measures::MeasureDecisionRecordReview, ApplicationError>
    {
        Err(ApplicationError::PermissionDenied)
    }
    fn submit(
        &self,
        _: &str,
        _: CaseId,
        _: application::precautionary_measures::MeasureDecisionCommand,
        _: application::precautionary_measures::MeasureDecisionConfirmation,
    ) -> Result<application::precautionary_measures::MeasureDecisionRecordReceipt, ApplicationError>
    {
        Err(ApplicationError::PermissionDenied)
    }
}
impl application::precautionary_measures::MeasureDecisionRecordReadWorkflow for Unused {
    fn list(
        &self,
        _: &str,
        _: CaseId,
        _: application::precautionary_measures::MeasureDecisionReadQuery,
    ) -> Result<application::precautionary_measures::MeasureDecisionRecordPage, ApplicationError>
    {
        Err(ApplicationError::PermissionDenied)
    }
    fn get(
        &self,
        _: &str,
        _: CaseId,
        _: domain::precautionary_measures::MeasureDecisionId,
    ) -> Result<application::precautionary_measures::MeasureDecisionRecordReceipt, ApplicationError>
    {
        Err(ApplicationError::PermissionDenied)
    }
    fn get_operation(
        &self,
        _: &str,
        _: CaseId,
        _: domain::precautionary_measures::MeasureDecisionOperationId,
    ) -> Result<application::precautionary_measures::MeasureDecisionRecordReceipt, ApplicationError>
    {
        Err(ApplicationError::PermissionDenied)
    }
}
