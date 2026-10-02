//! Authorized operational aggregates over complete, verified resource heads.

mod model;
mod service;
pub use model::*;
pub use service::DashboardService;

use crate::ApplicationError;
use domain::identity::UserId;

pub trait DashboardWorkflow: Send + Sync {
    fn read(&self, token: &str) -> Result<DashboardSnapshot, ApplicationError>;
}
pub trait DashboardStore: Send + Sync {
    fn read(&self, actor: UserId) -> Result<DashboardSnapshot, ApplicationError>;
}
