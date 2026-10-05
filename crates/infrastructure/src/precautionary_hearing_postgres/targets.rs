use super::inconsistent;
use application::ApplicationError;
use domain::precautionary_hearings::PrecautionaryMeasureRef;
use std::collections::BTreeMap;

pub(super) fn union<'a>(
    selections: impl IntoIterator<Item = &'a [PrecautionaryMeasureRef]>,
) -> Result<Vec<PrecautionaryMeasureRef>, ApplicationError> {
    let mut refs = BTreeMap::new();
    for selected in selections {
        for reference in selected {
            let key = (reference.id().as_uuid(), reference.revision().get());
            if refs
                .insert(key, *reference)
                .is_some_and(|old| old != *reference)
            {
                return Err(inconsistent("one target revision has conflicting digests"));
            }
            if refs.len() > 8192 {
                return Err(inconsistent("hearing target union exceeds its bound"));
            }
        }
    }
    Ok(refs.into_values().collect())
}
