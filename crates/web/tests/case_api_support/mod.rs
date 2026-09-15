use application::{cases::*, ApplicationError};
use domain::{cases::CaseId, identity::UserId, DomainError};
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Default)]
pub struct Workflow {
    pub calls: Mutex<Vec<String>>,
}

impl Workflow {
    fn record(&self, token: &str, call: String) -> Result<CaseRecord, ApplicationError> {
        self.calls.lock().unwrap().push(format!("{token} {call}"));
        match token {
            "expired" => return Err(ApplicationError::InvalidSession),
            "forbidden" => return Err(ApplicationError::PermissionDenied),
            "hidden" => return Err(ApplicationError::CaseNotFound),
            "missing-user" => return Err(ApplicationError::UserNotFound),
            "database-failed" => return Err(ApplicationError::Port("secret DSN".into())),
            _ => {}
        }
        Ok(CaseRecord {
            id: CaseId::from_uuid(Uuid::from_u128(1)),
            title: "Example case".into(),
            reference: "REF-123".into(),
            created_by: UserId::from_uuid(Uuid::from_u128(2)),
        })
    }
}

impl CaseWorkflow for Workflow {
    fn create(
        &self,
        token: &str,
        title: &str,
        reference: &str,
    ) -> Result<CaseRecord, ApplicationError> {
        if title.is_empty() {
            return Err(DomainError::InvalidCaseMetadata {
                field: "title",
                reason: "must not be empty",
            }
            .into());
        }
        self.record(token, format!("create {title} {reference}"))
    }

    fn list(
        &self,
        token: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<CaseRecord>, ApplicationError> {
        Ok(vec![self.record(token, format!("list {limit} {offset}"))?])
    }

    fn get(&self, token: &str, id: CaseId) -> Result<CaseRecord, ApplicationError> {
        self.record(token, format!("get {id}"))
    }

    fn assign(&self, token: &str, id: CaseId, user_id: UserId) -> Result<(), ApplicationError> {
        self.record(token, format!("assign {id} {user_id}"))?;
        Ok(())
    }

    fn remove(&self, token: &str, id: CaseId, user_id: UserId) -> Result<(), ApplicationError> {
        self.record(token, format!("remove {id} {user_id}"))?;
        Ok(())
    }
    fn register_penal(
        &self,
        _: &str,
        _: PenalCaseCreation,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        panic!("basic route called staff creation")
    }
    fn replace_administration(
        &self,
        _: &str,
        _: CaseId,
        _: CaseRevisionExpectation,
        _: CaseEditableValues,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        panic!("basic route called staff replacement")
    }
    fn change_administrative_status(
        &self,
        _: &str,
        _: CaseId,
        _: CaseRevisionExpectation,
        _: CaseAdministrativeStatus,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        panic!("basic route called staff status")
    }
    fn list_administrations(
        &self,
        _: &str,
        _: CaseAdministrationQuery,
    ) -> Result<CaseAdministrationPage, ApplicationError> {
        panic!("basic route called staff index")
    }
    fn get_administration(
        &self,
        _: &str,
        _: CaseId,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        panic!("basic route called staff detail")
    }
    fn administration_history(
        &self,
        _: &str,
        _: CaseId,
        _: CaseAdministrationHistoryQuery,
    ) -> Result<CaseAdministrationHistoryPage, ApplicationError> {
        panic!("basic route called staff history")
    }
}
