use domain::precautionary_measures::{
    MeasureCorrectionOperationId, MeasureCorrectionValues, MeasureSupervision, MeasureValues,
};

#[path = "measure_correction_support/mod.rs"]
mod support;

#[path = "measure_correction_support/semantics.rs"]
mod semantics;
#[path = "measure_correction_support/vectors.rs"]
mod vectors;

use support::*;

#[test]
fn correction_replaces_declared_text_without_changing_measure_identity() {
    let original = MeasureValues::new(input());
    let before = original.clone();
    let changes = MeasureCorrectionValues::new(
        note("Corrected conditions"),
        original.validity().clone(),
        note("Corrected supervision statement"),
    );

    let result = original.correct_record(&changes).unwrap();

    assert_eq!(result.subject(), original.subject());
    assert_eq!(result.kind(), original.kind());
    assert_eq!(result.conditions(), changes.conditions());
    assert_eq!(result.validity(), changes.validity());
    assert_eq!(
        result.supervision(),
        &MeasureSupervision::Known {
            participant: participant(),
            statement: changes.supervision_text().clone(),
        }
    );
    assert_eq!(original, before);
}

#[test]
fn correction_operation_identity_retains_the_exact_uuid() {
    let uuid = uuid::Uuid::from_u128(23);
    let id = MeasureCorrectionOperationId::from_uuid(uuid);
    assert_eq!(id.as_uuid(), uuid);
    assert_eq!(id.to_string(), uuid.to_string());
    assert_eq!(serde_json::to_string(&id).unwrap(), format!("\"{uuid}\""));
    assert_eq!(
        serde_json::from_str::<MeasureCorrectionOperationId>(&format!("\"{uuid}\"")).unwrap(),
        id
    );
    let identities = std::collections::HashSet::from([id, id]);
    assert_eq!(identities.len(), 1);
    assert!(serde_json::from_str::<MeasureCorrectionOperationId>("\"invalid\"").is_err());
    for generated in [
        MeasureCorrectionOperationId::new(),
        MeasureCorrectionOperationId::default(),
    ] {
        assert_eq!(generated.as_uuid().get_version_num(), 4);
        assert!(!generated.as_uuid().is_nil());
    }
}
