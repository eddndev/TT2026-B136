use crate::{
    procedural_facts::{resolve_fact_participants, FactParticipantProjection, FactSourceSelection},
    typed_participants::ParticipantDetail,
    ApplicationError,
};
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_hearings::PrecautionaryHearingValues,
    procedural_facts::FactParticipantRef,
};

/// Resolves exact historical values and subject bindings, including archived captures.
/// This does not authorize access, check current heads, or admit referenced documents.
pub fn resolve_precautionary_participants(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    values: &PrecautionaryHearingValues,
    material: &[ParticipantDetail],
) -> Result<Vec<FactParticipantProjection>, ApplicationError> {
    if material.len() != values.participants().len() {
        return Err(ApplicationError::InvalidInput(
            "precautionary participant count differs from exact selection".into(),
        ));
    }
    let mut ordered = material.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|detail| detail.id().as_uuid());
    ordered
        .into_iter()
        .zip(values.participants())
        .map(|(detail, reference)| {
            let selection = FactSourceSelection::select_participant(FactParticipantRef {
                id: reference.id(),
                revision: reference.revision(),
            });
            resolve_fact_participants(hasher, case_id, &selection, std::slice::from_ref(detail))?
                .into_iter()
                .next()
                .ok_or_else(|| {
                    ApplicationError::InvalidInput(
                        "selected precautionary participant was not resolved".into(),
                    )
                })
        })
        .collect()
}
