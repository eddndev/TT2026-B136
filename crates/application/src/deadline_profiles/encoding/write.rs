use super::{write_expected, write_fields, DeadlineProfileDefinition, PREFIX};
use crate::deadline_inputs::encoding::write as input;
use domain::typed_participants::Uuid;
use std::num::NonZeroU32;

pub(super) fn encode(profile: &DeadlineProfileDefinition) -> Vec<u8> {
    let mut bytes = PREFIX.to_vec();
    text(&mut bytes, profile.title().as_str());
    text(&mut bytes, profile.description().as_str());
    write_fields::scope(&mut bytes, profile.scope());
    count(&mut bytes, profile.references().len());
    for source in profile.references() {
        write_fields::source(&mut bytes, source);
    }
    input::requirement(&mut bytes, profile.trigger());
    write_fields::template(&mut bytes, profile.template());
    write_fields::completion(&mut bytes, profile.completion());
    count(&mut bytes, profile.conditions().len());
    for condition in profile.conditions() {
        bytes.extend_from_slice(condition.id.as_bytes());
        text(&mut bytes, condition.statement.as_str());
        reference_ids(&mut bytes, &condition.reference_ids);
    }
    count(&mut bytes, profile.examples().len());
    for example in profile.examples() {
        bytes.extend_from_slice(example.id.as_bytes());
        input::declared_time(&mut bytes, example.anchor);
        optional_quantity(&mut bytes, example.ordered_quantity);
        bytes.push(u8::from(example.calendar.is_some()));
        if let Some(calendar) = &example.calendar {
            let canonical = calendar.canonical_bytes();
            count(&mut bytes, canonical.len());
            bytes.extend_from_slice(&canonical);
        }
        write_expected::expected(&mut bytes, example.expected);
        reference_ids(&mut bytes, &example.reference_ids);
        text(&mut bytes, example.locator.as_str());
    }
    bytes
}
pub(super) fn count(bytes: &mut Vec<u8>, count: usize) {
    bytes.extend_from_slice(&(count as u32).to_be_bytes());
}
pub(super) fn text(bytes: &mut Vec<u8>, text: &str) {
    count(bytes, text.len());
    bytes.extend_from_slice(text.as_bytes());
}
fn reference_ids(bytes: &mut Vec<u8>, ids: &[Uuid]) {
    count(bytes, ids.len());
    for id in ids {
        bytes.extend_from_slice(id.as_bytes());
    }
}
pub(super) fn optional_quantity(bytes: &mut Vec<u8>, quantity: Option<NonZeroU32>) {
    bytes.push(u8::from(quantity.is_some()));
    if let Some(quantity) = quantity {
        bytes.extend_from_slice(&quantity.get().to_be_bytes());
    }
}
