use super::{
    object::Object,
    request::{checked, uuid},
};
use crate::{error::ApiError, procedural_facts::values::time::DeclaredTime};
use domain::{
    cases::CaseId,
    deadline_triggers::*,
    hearing_results::{HearingResultAgreementId, HearingResultId, HearingResultRevision},
    hearings::HearingId,
    procedural_facts::*,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Declaration<T> {
    Unknown { reason: String },
    Known { value: T },
}
impl<T> Declaration<T> {
    pub(super) fn validate<U>(
        self,
        convert: impl FnOnce(T) -> Result<U, ApiError>,
    ) -> Result<FactDeclaration<U>, ApiError> {
        match self {
            Self::Unknown { reason } => {
                Ok(FactDeclaration::Unknown(checked(FactText::new(&reason))?))
            }
            Self::Known { value } => Ok(FactDeclaration::Known(convert(value)?)),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    case_id: String,
    source: Object<Declaration<Object<Source>>>,
    qualification: Option<Object<Qualification>>,
}
#[derive(Deserialize)]
#[serde(tag = "family", rename_all = "snake_case", deny_unknown_fields)]
enum Source {
    Resolution {
        id: String,
        revision: u32,
    },
    Notification {
        id: String,
        revision: u32,
        resolution: Object<Resolution>,
    },
    HearingResult {
        hearing_id: String,
        result_id: String,
        revision: u32,
        agreement_id: Option<String>,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resolution {
    id: String,
    revision: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Qualification {
    purpose: Purpose,
    at: Object<DeclaredTime>,
    statement: String,
    locator: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Purpose {
    HearingEnd,
    OrderedPeriodStart,
}
impl Selection {
    pub(super) fn validate(self) -> Result<TriggerSelection, ApiError> {
        Ok(TriggerSelection {
            case_id: CaseId::from_uuid(uuid(&self.case_id, "invalid_case_id")?),
            source: self.source.0.validate(|v| v.0.validate())?,
            qualification: self.qualification.map(|v| v.0.validate()).transpose()?,
        })
    }
}
impl Source {
    fn validate(self) -> Result<TriggerSourceRef, ApiError> {
        Ok(match self {
            Self::Resolution { id, revision } => {
                TriggerSourceRef::Resolution(Resolution { id, revision }.validate()?)
            }
            Self::Notification {
                id,
                revision,
                resolution,
            } => TriggerSourceRef::Notification {
                id: NotificationId::from_uuid(uuid(&id, "invalid_notification_id")?),
                revision: checked(FactRevision::new(revision))?,
                resolution: resolution.0.validate()?,
            },
            Self::HearingResult {
                hearing_id,
                result_id,
                revision,
                agreement_id,
            } => TriggerSourceRef::HearingResult(FactHearingRef {
                hearing_id: HearingId::from_uuid(uuid(&hearing_id, "invalid_hearing_id")?),
                result_id: HearingResultId::from_uuid(uuid(
                    &result_id,
                    "invalid_hearing_result_id",
                )?),
                revision: checked(HearingResultRevision::new(revision))?,
                agreement_id: agreement_id
                    .map(|v| {
                        uuid(&v, "invalid_hearing_agreement_id")
                            .map(HearingResultAgreementId::from_uuid)
                    })
                    .transpose()?,
            }),
        })
    }
}
impl Resolution {
    fn validate(self) -> Result<FactResolutionRef, ApiError> {
        Ok(FactResolutionRef {
            id: ResolutionId::from_uuid(uuid(&self.id, "invalid_resolution_id")?),
            revision: checked(FactRevision::new(self.revision))?,
        })
    }
}
impl Qualification {
    fn validate(self) -> Result<QualifiedTriggerTime, ApiError> {
        Ok(QualifiedTriggerTime {
            purpose: match self.purpose {
                Purpose::HearingEnd => QualifiedTriggerPurpose::HearingEnd,
                Purpose::OrderedPeriodStart => QualifiedTriggerPurpose::OrderedPeriodStart,
            },
            at: self.at.0.validate()?,
            statement: checked(FactText::new(&self.statement))?,
            locator: checked(FactLabel::new(&self.locator))?,
        })
    }
}
