use application::cases::*;
use domain::{cases::CaseId, identity::UserId};
use serde::Serialize;
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};

use super::request::ProfileData;
use crate::error::ApiError;

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
pub(super) struct Administration {
    case_id: CaseId,
    revision: u32,
    title: String,
    reference: String,
    administrative_status: &'static str,
    profile: Option<ProfileData>,
    values_digest: Option<String>,
    changed_at: Option<String>,
    changed_by: Option<Actor>,
}

impl Administration {
    fn current(id: CaseId, current: CurrentCaseAdministration) -> Result<Self, ApiError> {
        match current {
            CurrentCaseAdministration::Recorded(snapshot) => {
                if snapshot.case_id != id {
                    return Err(ApiError::internal());
                }
                (*snapshot).try_into()
            }
            CurrentCaseAdministration::Unrevised(metadata) => Ok(Self {
                case_id: id,
                revision: 0,
                title: metadata.title().into(),
                reference: metadata.reference().into(),
                administrative_status: "active",
                profile: None,
                values_digest: None,
                changed_at: None,
                changed_by: None,
            }),
        }
    }
}
impl TryFrom<CaseAdministrationSnapshot> for Administration {
    type Error = ApiError;
    fn try_from(value: CaseAdministrationSnapshot) -> Result<Self, Self::Error> {
        Ok(Self {
            case_id: value.case_id,
            revision: value.revision.get(),
            title: value.values.metadata().title().into(),
            reference: value.values.metadata().reference().into(),
            administrative_status: value.values.status().as_str(),
            profile: value.values.profile().map(ProfileData::from),
            values_digest: Some(value.values_digest.to_hex()),
            changed_at: Some(utc(value.changed_at)?),
            changed_by: Some(value.changed_by.into()),
        })
    }
}

#[derive(Serialize)]
struct InitialStage {
    case_id: CaseId,
    stage_revision: u32,
    administration_revision: u32,
    stage: &'static str,
    administration_digest: String,
    recorded_at: String,
    recorded_by: Actor,
}
impl TryFrom<CaseInitialStageRegistration> for InitialStage {
    type Error = ApiError;
    fn try_from(value: CaseInitialStageRegistration) -> Result<Self, Self::Error> {
        if value.stage_revision != CaseStageRevision::FIRST
            || value.administration_revision != CaseRevision::FIRST
        {
            return Err(ApiError::internal());
        }
        Ok(Self {
            case_id: value.case_id,
            stage_revision: value.stage_revision.get(),
            administration_revision: value.administration_revision.get(),
            stage: value.stage.as_str(),
            administration_digest: value.administration_digest.to_hex(),
            recorded_at: utc(value.recorded_at)?,
            recorded_by: value.recorded_by.into(),
        })
    }
}

#[derive(Serialize)]
pub(super) struct Detail {
    id: CaseId,
    created_by: UserId,
    created_at: String,
    administration: Administration,
    initial_stage: Option<InitialStage>,
}
impl TryFrom<CaseAdministrationDetail> for Detail {
    type Error = ApiError;
    fn try_from(value: CaseAdministrationDetail) -> Result<Self, Self::Error> {
        if value
            .initial_stage
            .as_ref()
            .is_some_and(|stage| stage.case_id != value.origin.id)
        {
            return Err(ApiError::internal());
        }
        Ok(Self {
            id: value.origin.id,
            created_by: value.origin.created_by,
            created_at: utc(value.origin.created_at)?,
            administration: Administration::current(value.origin.id, value.administration)?,
            initial_stage: value
                .initial_stage
                .map(InitialStage::try_from)
                .transpose()?,
        })
    }
}

#[derive(Serialize)]
struct Identifiers {
    nuc: String,
    judicial_case_number: String,
}
#[derive(Serialize)]
struct Overview {
    id: CaseId,
    title: String,
    reference: String,
    created_by: UserId,
    created_at: String,
    revision: u32,
    administrative_status: &'static str,
    profile_status: &'static str,
    penal_identifiers: Option<Identifiers>,
    initial_stage: Option<&'static str>,
}
impl TryFrom<CaseAdministrationOverview> for Overview {
    type Error = ApiError;
    fn try_from(value: CaseAdministrationOverview) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.origin.id,
            title: value.metadata.title().into(),
            reference: value.metadata.reference().into(),
            created_by: value.origin.created_by,
            created_at: utc(value.origin.created_at)?,
            revision: value.revision.map_or(0, CaseRevision::get),
            administrative_status: value.administrative_status.as_str(),
            profile_status: if value.penal_identifiers.is_some() {
                "complete"
            } else {
                "pending"
            },
            penal_identifiers: value.penal_identifiers.map(|v| Identifiers {
                nuc: v.nuc,
                judicial_case_number: v.judicial_case_number,
            }),
            initial_stage: value.initial_stage.map(|stage| stage.as_str()),
        })
    }
}

#[derive(Serialize)]
pub(super) struct Page {
    cases: Vec<Overview>,
    has_more: bool,
    next_after_id: Option<CaseId>,
}
impl TryFrom<CaseAdministrationPage> for Page {
    type Error = ApiError;
    fn try_from(value: CaseAdministrationPage) -> Result<Self, Self::Error> {
        Ok(Self {
            cases: value
                .cases
                .into_iter()
                .map(Overview::try_from)
                .collect::<Result<_, _>>()?,
            has_more: value.has_more,
            next_after_id: value.next_after_id,
        })
    }
}

#[derive(Serialize)]
pub(super) struct HistoryPage {
    revisions: Vec<Administration>,
    has_more: bool,
    next_before_revision: Option<u32>,
}
impl TryFrom<CaseAdministrationHistoryPage> for HistoryPage {
    type Error = ApiError;
    fn try_from(value: CaseAdministrationHistoryPage) -> Result<Self, Self::Error> {
        Ok(Self {
            revisions: value
                .revisions
                .into_iter()
                .map(Administration::try_from)
                .collect::<Result<_, _>>()?,
            has_more: value.has_more,
            next_before_revision: value.next_before_revision.map(CaseRevision::get),
        })
    }
}

fn utc(value: OffsetDateTime) -> Result<String, ApiError> {
    value
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|_| ApiError::internal())
}
