use super::{checked, label_text, text};
use crate::error::ApiError;
use application::procedural_facts::*;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Declaration<T> {
    Known { value: T },
    Unknown { reason: String },
}
impl<T> Declaration<T> {
    pub(super) fn validate<V>(
        self,
        parse: impl FnOnce(T) -> Result<V, ApiError>,
    ) -> Result<FactDeclaration<V>, ApiError> {
        match self {
            Self::Known { value } => Ok(FactDeclaration::Known(parse(value)?)),
            Self::Unknown { reason } => Ok(FactDeclaration::Unknown(text(&reason)?)),
        }
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Class {
    Order {},
    Judgment {},
    Other { label: String },
}
impl Class {
    pub(super) fn validate(self) -> Result<ResolutionClass, ApiError> {
        Ok(match self {
            Self::Order {} => ResolutionClass::Order,
            Self::Judgment {} => ResolutionClass::Judgment,
            Self::Other { label } => ResolutionClass::Other(label_text(&label)?),
        })
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Character {
    Personal {},
    Publication {},
    Other { label: String },
}
impl Character {
    pub(super) fn validate(self) -> Result<NotificationCharacter, ApiError> {
        Ok(match self {
            Self::Personal {} => NotificationCharacter::Personal,
            Self::Publication {} => NotificationCharacter::Publication,
            Self::Other { label } => NotificationCharacter::Other(label_text(&label)?),
        })
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Medium {
    InPerson {},
    Electronic {},
    Other { label: String },
}
impl Medium {
    pub(super) fn validate(self) -> Result<NotificationMedium, ApiError> {
        Ok(match self {
            Self::InPerson {} => NotificationMedium::InPerson,
            Self::Electronic {} => NotificationMedium::Electronic,
            Self::Other { label } => NotificationMedium::Other(label_text(&label)?),
        })
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Context {
    InHearing {},
    OutsideHearing {},
    Other { label: String },
}
impl Context {
    pub(super) fn validate(self) -> Result<NotificationContext, ApiError> {
        Ok(match self {
            Self::InHearing {} => NotificationContext::InHearing,
            Self::OutsideHearing {} => NotificationContext::OutsideHearing,
            Self::Other { label } => NotificationContext::Other(label_text(&label)?),
        })
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Outcome {
    Practiced {},
    Attempted {},
}
impl Outcome {
    pub(super) fn validate(self) -> Result<NotificationOutcome, ApiError> {
        checked(Ok(match self {
            Self::Practiced {} => NotificationOutcome::Practiced,
            Self::Attempted {} => NotificationOutcome::Attempted,
        }))
    }
}
pub(super) fn declaration<T>(
    value: &FactDeclaration<T>,
    project: impl FnOnce(&T) -> Value,
) -> Value {
    match value {
        FactDeclaration::Known(v) => json!({"kind":"known","value":project(v)}),
        FactDeclaration::Unknown(v) => json!({"kind":"unknown","reason":v.as_str()}),
    }
}
pub(super) fn class(value: &ResolutionClass) -> Value {
    match value {
        ResolutionClass::Order => json!({"kind":"order"}),
        ResolutionClass::Judgment => json!({"kind":"judgment"}),
        ResolutionClass::Other(v) => json!({"kind":"other","label":v.as_str()}),
    }
}
pub(super) fn character(value: &NotificationCharacter) -> Value {
    match value {
        NotificationCharacter::Personal => json!({"kind":"personal"}),
        NotificationCharacter::Publication => json!({"kind":"publication"}),
        NotificationCharacter::Other(v) => json!({"kind":"other","label":v.as_str()}),
    }
}
pub(super) fn medium(value: &NotificationMedium) -> Value {
    match value {
        NotificationMedium::InPerson => json!({"kind":"in_person"}),
        NotificationMedium::Electronic => json!({"kind":"electronic"}),
        NotificationMedium::Other(v) => json!({"kind":"other","label":v.as_str()}),
    }
}
pub(super) fn context(value: &NotificationContext) -> Value {
    match value {
        NotificationContext::InHearing => json!({"kind":"in_hearing"}),
        NotificationContext::OutsideHearing => json!({"kind":"outside_hearing"}),
        NotificationContext::Other(v) => json!({"kind":"other","label":v.as_str()}),
    }
}
pub(super) fn outcome(value: &NotificationOutcome) -> Value {
    match value {
        NotificationOutcome::Practiced => json!({"kind":"practiced"}),
        NotificationOutcome::Attempted => json!({"kind":"attempted"}),
    }
}
