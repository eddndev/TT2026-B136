use super::super::{
    input_canonical::{invalid, MAX_CONDITIONS},
    DeadlineApplicability, DeadlineConditionAnswer, DeadlineEvaluationInput,
};
use crate::{
    deadline_inputs::encoding::{
        read::{calendar, selection},
        reader::Reader,
    },
    ApplicationError,
};
use domain::procedural_facts::{FactDeclaration, FactLabel, FactText};
use std::num::NonZeroU32;

pub(crate) fn decode(reader: &mut Reader<'_>) -> Result<DeadlineEvaluationInput, ApplicationError> {
    let selection = selection(reader)?;
    let calendar = calendar(reader)?;
    let ordered_quantity = if reader.flag()? {
        Some(NonZeroU32::new(reader.u32()?).ok_or_else(|| invalid("ordered quantity is zero"))?)
    } else {
        None
    };
    let statement = fact_text(reader)?;
    let locator = label(reader)?;
    let scope_applies = declaration(reader)?;
    let unresolved_incident = declaration(reader)?;
    let count = usize::try_from(reader.u32()?).map_err(invalid)?;
    if count > MAX_CONDITIONS {
        return Err(invalid("condition count exceeds sixteen"));
    }
    let mut conditions = Vec::with_capacity(count);
    for _ in 0..count {
        conditions.push(DeadlineConditionAnswer {
            id: reader.uuid()?,
            applies: declaration(reader)?,
            locator: label(reader)?,
        });
    }
    Ok(DeadlineEvaluationInput {
        selection,
        calendar,
        ordered_quantity,
        qualification: DeadlineApplicability {
            statement,
            locator,
            scope_applies,
            unresolved_incident,
            conditions,
        },
    })
}

fn fact_text(reader: &mut Reader<'_>) -> Result<FactText, ApplicationError> {
    FactText::new(reader.text(4000)?).map_err(invalid)
}
fn label(reader: &mut Reader<'_>) -> Result<FactLabel, ApplicationError> {
    FactLabel::new(reader.text(800)?).map_err(invalid)
}
fn declaration(reader: &mut Reader<'_>) -> Result<FactDeclaration<bool>, ApplicationError> {
    Ok(if reader.flag()? {
        FactDeclaration::Known(reader.flag()?)
    } else {
        FactDeclaration::Unknown(fact_text(reader)?)
    })
}
