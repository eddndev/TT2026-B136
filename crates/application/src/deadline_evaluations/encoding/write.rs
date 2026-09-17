use super::super::{input_canonical::PREFIX, DeadlineEvaluationInput};
use crate::deadline_inputs::encoding::write::{calendar, selection, text};
use domain::procedural_facts::FactDeclaration;

pub(crate) fn encode(input: &DeadlineEvaluationInput) -> Vec<u8> {
    let mut bytes = PREFIX.to_vec();
    selection(&mut bytes, &input.selection);
    calendar(&mut bytes, input.calendar);
    bytes.push(u8::from(input.ordered_quantity.is_some()));
    if let Some(quantity) = input.ordered_quantity {
        bytes.extend_from_slice(&quantity.get().to_be_bytes());
    }
    let applicability = &input.qualification;
    text(&mut bytes, applicability.statement.as_str());
    text(&mut bytes, applicability.locator.as_str());
    declaration(&mut bytes, &applicability.scope_applies);
    declaration(&mut bytes, &applicability.unresolved_incident);
    bytes.extend_from_slice(&(applicability.conditions.len() as u32).to_be_bytes());
    for condition in &applicability.conditions {
        bytes.extend_from_slice(condition.id.as_bytes());
        declaration(&mut bytes, &condition.applies);
        text(&mut bytes, condition.locator.as_str());
    }
    bytes
}

fn declaration(bytes: &mut Vec<u8>, value: &FactDeclaration<bool>) {
    match value {
        FactDeclaration::Unknown(reason) => {
            bytes.push(0);
            text(bytes, reason.as_str());
        }
        FactDeclaration::Known(value) => {
            bytes.extend_from_slice(&[1, u8::from(*value)]);
        }
    }
}
