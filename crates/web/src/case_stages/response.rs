use super::{request::Support, values::Values};
use crate::error::ApiError;
use application::{
    case_stages::{CaseStageDetail, CaseStageEntry, CaseStagePage, StageSupportSnapshot},
    cases::CaseActorSnapshot,
};
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    cases::CaseId,
    identity::UserId,
};
use serde::Serialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};

#[derive(Serialize)]
struct Actor {
    id: UserId,
    email: String,
}
impl From<CaseActorSnapshot> for Actor {
    fn from(value: CaseActorSnapshot) -> Self {
        Self {
            id: value.id,
            email: value.email,
        }
    }
}
#[derive(Serialize)]
struct Common {
    case_id: CaseId,
    stage_revision: u32,
    stage: &'static str,
    administration_revision: u32,
    administration_digest: String,
    recorded_at: String,
    recorded_by: Actor,
}
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Entry {
    Initial {
        #[serde(flatten)]
        common: Common,
    },
    Change {
        #[serde(flatten)]
        common: Common,
        from_stage: Option<&'static str>,
        values_digest: String,
        values: Box<Values>,
        supports: Vec<SupportSnapshot>,
    },
}
#[derive(Serialize)]
struct SupportSnapshot {
    #[serde(flatten)]
    reference: Support,
    name: String,
    format: &'static str,
    policy: &'static str,
}
impl From<StageSupportSnapshot> for SupportSnapshot {
    fn from(value: StageSupportSnapshot) -> Self {
        Self {
            reference: application::case_stages::StageSupportRef::new(
                value.reference,
                value.digest,
            )
            .into(),
            name: value.name,
            format: value.format.as_str(),
            policy: value.policy.as_str(),
        }
    }
}
impl Entry {
    fn from_row(value: CaseStageEntry, case_id: CaseId) -> Result<Self, ApiError> {
        if value.case_id() != case_id {
            return Err(ApiError::internal());
        }
        let stage = value.stage().as_str();
        match value {
            CaseStageEntry::Initial(value) => {
                if value.stage_revision != CaseStageRevision::FIRST
                    || value.administration_revision != CaseRevision::FIRST
                {
                    return Err(ApiError::internal());
                }
                Ok(Self::Initial {
                    common: Common {
                        case_id,
                        stage_revision: value.stage_revision.get(),
                        stage,
                        administration_revision: value.administration_revision.get(),
                        administration_digest: value.administration_digest.to_hex(),
                        recorded_at: utc(value.recorded_at)?,
                        recorded_by: value.recorded_by.into(),
                    },
                })
            }
            CaseStageEntry::Changed(value) => Ok(Self::Change {
                common: Common {
                    case_id,
                    stage_revision: value.stage_revision.get(),
                    stage,
                    administration_revision: value.administration_revision.get(),
                    administration_digest: value.administration_digest.to_hex(),
                    recorded_at: utc(value.recorded_at)?,
                    recorded_by: value.recorded_by.into(),
                },
                from_stage: value.from_stage.map(|v| v.as_str()),
                values_digest: value.values_digest.to_hex(),
                values: Box::new(value.values.try_into()?),
                supports: value.supports.into_iter().map(Into::into).collect(),
            }),
        }
    }
}
#[derive(Serialize)]
pub(super) struct Detail {
    case_id: CaseId,
    current: Option<Entry>,
}
impl Detail {
    pub fn from_row(value: CaseStageDetail, case_id: CaseId) -> Result<Self, ApiError> {
        if value.case_id != case_id {
            return Err(ApiError::internal());
        }
        let current = match value.current {
            application::case_stages::CurrentCaseStage::Unregistered => None,
            application::case_stages::CurrentCaseStage::Registered(entry) => {
                Some(Entry::from_row(*entry, case_id)?)
            }
        };
        Ok(Self { case_id, current })
    }
}
#[derive(Serialize)]
pub(super) struct HistoryPage {
    entries: Vec<Entry>,
    has_more: bool,
    next_before_revision: Option<u32>,
}
impl HistoryPage {
    pub fn from_page(value: CaseStagePage, case_id: CaseId) -> Result<Self, ApiError> {
        Ok(Self {
            entries: value
                .entries
                .into_iter()
                .map(|v| Entry::from_row(v, case_id))
                .collect::<Result<_, _>>()?,
            has_more: value.has_more,
            next_before_revision: value.next_before_revision.map(|v| v.get()),
        })
    }
}
fn utc(value: OffsetDateTime) -> Result<String, ApiError> {
    value
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|_| ApiError::internal())
}
