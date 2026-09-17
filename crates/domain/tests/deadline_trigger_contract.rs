use domain::{
    cases::CaseId,
    deadline_triggers::{
        extract_trigger_time, TriggerBlock, TriggerField, TriggerOutcome, TriggerRequirement,
        TriggerSelection,
    },
    procedural_facts::{FactDeclaration, FactText},
};

#[test]
fn unresolved_selection_preserves_its_original_reason_after_input_changes() {
    let original = FactText::new("Exact resolution is not yet identified").unwrap();
    let mut selection = TriggerSelection {
        case_id: CaseId::new(),
        source: FactDeclaration::Unknown(original.clone()),
        qualification: None,
    };
    let requirement = TriggerRequirement::SourceField(TriggerField::ResolutionIssuedAt);
    let result = extract_trigger_time(requirement, &selection, None).unwrap();
    selection.source = FactDeclaration::Unknown(FactText::new("New intake finding").unwrap());
    assert_eq!(
        result.selection().source,
        FactDeclaration::Unknown(original)
    );
    assert_eq!(result.requirement(), requirement);
    assert_eq!(
        result.outcome(),
        &TriggerOutcome::Blocked(TriggerBlock::UnknownSource)
    );
    assert!(result.source().is_none());
}
