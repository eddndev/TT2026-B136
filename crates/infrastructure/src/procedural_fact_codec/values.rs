use super::{declarations::*, helpers::*, inconsistent, provenance::*, temporal, Result};
use application::procedural_facts::*;
use serde_json::Value;

pub(super) fn resolution(value: &Value) -> Result<ResolutionValues> {
    fields(
        value,
        &[
            "class",
            "subtype",
            "issuer",
            "issued_at",
            "summary",
            "provenance",
        ],
    )?;
    Ok(ResolutionValues::new(ResolutionValuesInput {
        class: declaration(&value["class"], class)?,
        subtype: optional(&value["subtype"], label)?,
        issuer: declaration(&value["issuer"], label)?,
        issued_at: temporal::value_time(&value["issued_at"])?,
        summary: text(&value["summary"])?,
        provenance: provenance(&value["provenance"])?,
    }))
}
fn resolution_reference(value: &Value) -> Result<FactResolutionRef> {
    fields(value, &["id", "revision"])?;
    Ok(FactResolutionRef {
        id: ResolutionId::from_uuid(uuid(&value["id"])?),
        revision: FactRevision::new(counter(&value["revision"])?).map_err(|_| inconsistent())?,
    })
}
fn effect(value: &Value) -> Result<FactStatedEffect> {
    fields(value, &["at", "statement", "locator"])?;
    Ok(FactStatedEffect {
        at: temporal::value_time(&value["at"])?,
        statement: text(&value["statement"])?,
        locator: label(&value["locator"])?,
    })
}
pub(super) fn notification(value: &Value) -> Result<NotificationValues> {
    fields(
        value,
        &[
            "resolution",
            "character",
            "medium",
            "context",
            "outcome",
            "subtype",
            "practiced_at",
            "received_at",
            "stated_effect",
            "intended_recipient",
            "actual_receiver",
            "representation",
            "summary",
            "provenance",
        ],
    )?;
    NotificationValues::new(NotificationValuesInput {
        resolution: resolution_reference(&value["resolution"])?,
        character: declaration(&value["character"], character)?,
        medium: declaration(&value["medium"], medium)?,
        context: declaration(&value["context"], context)?,
        outcome: declaration(&value["outcome"], outcome)?,
        subtype: optional(&value["subtype"], label)?,
        practiced_at: temporal::value_time(&value["practiced_at"])?,
        received_at: optional(&value["received_at"], temporal::value_time)?,
        stated_effect: optional(&value["stated_effect"], effect)?,
        intended_recipient: declaration(&value["intended_recipient"], person)?,
        actual_receiver: declaration(&value["actual_receiver"], person)?,
        representation: representation(&value["representation"])?,
        summary: text(&value["summary"])?,
        provenance: provenance(&value["provenance"])?,
    })
    .map_err(|_| inconsistent())
}
