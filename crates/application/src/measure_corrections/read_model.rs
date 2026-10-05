use super::MeasureAdministrativeStoredOperation;
use crate::ApplicationError;
use domain::{cases::CaseId, precautionary_measures::MeasureCorrectionOperationId};

/// Ascending immutable administrative operation UUIDs within one case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasureAdministrativeReadQuery {
    limit: u16,
    after_operation_id: Option<MeasureCorrectionOperationId>,
}
impl MeasureAdministrativeReadQuery {
    pub fn new(
        limit: u16,
        after_operation_id: Option<MeasureCorrectionOperationId>,
    ) -> Result<Self, ApplicationError> {
        if !(1..=20).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "administrative measure list limit must be 1..20".into(),
            ));
        }
        Ok(Self {
            limit,
            after_operation_id,
        })
    }
    pub const fn limit(self) -> u16 {
        self.limit
    }
    pub const fn after_operation_id(self) -> Option<MeasureCorrectionOperationId> {
        self.after_operation_id
    }
}
impl Default for MeasureAdministrativeReadQuery {
    fn default() -> Self {
        Self {
            limit: 10,
            after_operation_id: None,
        }
    }
}

/// Original captures and complete ancestor closures, including erroneous-capture marks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureAdministrativePage {
    pub case_id: CaseId,
    pub items: Vec<MeasureAdministrativeStoredOperation>,
    pub has_more: bool,
    pub next_after_operation_id: Option<MeasureCorrectionOperationId>,
}
