//! HTTP values preserve declared components; optional input projects explicit nulls.
pub(crate) mod catalog;
mod input;
mod people;
pub(crate) mod provenance;
pub(crate) mod time;

use crate::error::ApiError;
use application::{procedural_facts::*, ApplicationError};
use domain::{procedural_time::DeclaredProceduralTime, DomainError};
pub(super) use input::{Notification, Resolution};
use serde_json::{json, Value};

fn checked<T>(value: Result<T, DomainError>) -> Result<T, ApiError> {
    value.map_err(|e| ApplicationError::from(e).into())
}
fn text(value: &str) -> Result<FactText, ApiError> {
    checked(FactText::new(value))
}
fn label_text(value: &str) -> Result<FactLabel, ApiError> {
    checked(FactLabel::new(value))
}

pub(super) fn time(value: DeclaredProceduralTime) -> Result<Value, ApiError> {
    time::project(value)
}
pub(super) fn class(value: &FactDeclaration<ResolutionClass>) -> Value {
    catalog::declaration(value, catalog::class)
}
pub(super) fn label(value: &FactDeclaration<FactLabel>) -> Value {
    catalog::declaration(value, |v| json!(v.as_str()))
}
pub(super) fn outcome(value: &FactDeclaration<NotificationOutcome>) -> Value {
    catalog::declaration(value, catalog::outcome)
}

pub(super) fn resolution(value: &ResolutionValues) -> Result<Value, ApiError> {
    Ok(
        json!({"class":class(value.class()),"subtype":value.subtype().map(FactLabel::as_str),
        "issuer":label(value.issuer()),"issued_at":time(value.issued_at())?,
        "summary":value.summary().as_str(),"provenance":provenance::project(value.provenance())}),
    )
}
pub(super) fn notification(value: &NotificationValues) -> Result<Value, ApiError> {
    let stated_effect = value
        .stated_effect()
        .map(|v| {
            Ok::<_, ApiError>(json!({
                "at":time(v.at)?,"statement":v.statement.as_str(),"locator":v.locator.as_str(),
            }))
        })
        .transpose()?;
    Ok(
        json!({"resolution":{"id":value.resolution().id.to_string(),"revision":value.resolution().revision.get()},
        "character":catalog::declaration(value.character(),catalog::character),
        "medium":catalog::declaration(value.medium(),catalog::medium),
        "context":catalog::declaration(value.context(),catalog::context),"outcome":outcome(value.outcome()),
        "subtype":value.subtype().map(FactLabel::as_str),"practiced_at":time(value.practiced_at())?,
        "received_at":value.received_at().map(time).transpose()?,"stated_effect":stated_effect,
        "intended_recipient":catalog::declaration(value.intended_recipient(),people::person),
        "actual_receiver":catalog::declaration(value.actual_receiver(),people::person),
        "representation":people::representation(value.representation()),
        "summary":value.summary().as_str(),"provenance":provenance::project(value.provenance())}),
    )
}
