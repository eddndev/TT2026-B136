//! Authenticated commands for the separate administrative CA executor.
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ca_access::{CaOperatorAccess, OperatorIdentity};
use crate::ApplicationError;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaPurpose {
    Partner,
    DocumentSigning,
}

/// Fixed operations accept public material and identifiers, never shell or paths.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "kebab-case", deny_unknown_fields)]
pub enum CaCommand {
    Status {},
    Inventory {
        offset: u32,
        limit: u16,
    },
    Certificate {
        serial: String,
        fingerprint: String,
    },
    Preview {
        csr_pem: String,
        purpose: CaPurpose,
    },
    Issue {
        operation_id: String,
        csr_pem: String,
        purpose: CaPurpose,
        holder: String,
        reference: String,
        criterion: String,
        expected_root_fingerprint: String,
    },
    ResumeIssue {
        operation_id: String,
    },
    History {
        offset: u32,
        limit: u16,
    },
    Revoke {
        operation_id: String,
        serial: String,
        fingerprint: String,
        reason: String,
        expected_root_fingerprint: String,
    },
    ResumeRevoke {
        operation_id: String,
    },
    RenewCrl {
        operation_id: String,
        expected_revision: u32,
        acknowledge_maintenance: bool,
    },
    ResumeCrl {
        operation_id: String,
    },
}

pub trait CaExecutor: Send + Sync {
    /// Actor comes exclusively from the current admitted CA session.
    fn execute(
        &self,
        actor: &OperatorIdentity,
        command: CaCommand,
    ) -> Result<Value, ApplicationError>;
}

pub struct CaOperations {
    access: Arc<CaOperatorAccess>,
    executor: Arc<dyn CaExecutor>,
}

impl CaOperations {
    pub fn new(access: Arc<CaOperatorAccess>, executor: Arc<dyn CaExecutor>) -> Self {
        Self { access, executor }
    }

    pub fn execute(&self, token: &str, command: CaCommand) -> Result<Value, ApplicationError> {
        let actor = self.access.authenticate(token)?;
        match &command {
            CaCommand::Inventory { limit, .. } | CaCommand::History { limit, .. }
                if !(1..=100).contains(limit) =>
            {
                return Err(ApplicationError::InvalidInput(
                    "CA page size must be 1 through 100".into(),
                ))
            }
            CaCommand::RenewCrl {
                acknowledge_maintenance: false,
                ..
            } => {
                return Err(ApplicationError::InvalidInput(
                    "CA publication requires maintenance acknowledgement".into(),
                ))
            }
            _ => {}
        }
        self.executor.execute(&actor, command)
    }
}
