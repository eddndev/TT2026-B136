mod procedural_fact_support;
mod procedural_resource_support;
use domain::{
    procedural_facts::*,
    procedural_resources::*,
    procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime},
    DomainError,
};
use procedural_fact_support::{evidence, label, text};
use procedural_resource_support::*;
use time::UtcOffset;

#[test]
fn written_revocation_keeps_exact_resolution_document_and_declared_appellant_capture() {
    let expected = input();
    let values = ResourceValues::new(expected.clone()).unwrap();
    assert_eq!(values.kind(), ResourceKind::Revocation);
    assert_eq!(
        values.mode(),
        &FactDeclaration::Known(ResourceMode::Written)
    );
    assert_eq!(values.resolution(), expected.resolution);
    assert_eq!(values.resolution_evidence(), &expected.resolution_evidence);
    assert_eq!(
        values.direct_supports(),
        vec![expected.resolution_evidence.support()]
    );
    let person = &values.appellants()[0];
    assert_eq!(person.name().as_str(), "Captured name");
    assert_eq!(
        person.role(),
        &FactDeclaration::Known(label("Declared affected party"))
    );
    assert_eq!(person.participant().unwrap().revision.get(), 4);
    assert_eq!(values.title(), &expected.title);
    assert_eq!(values.challenged_part(), &expected.challenged_part);
    assert_eq!(values.grounds(), &expected.grounds);
}

#[test]
fn both_families_preserve_unknown_mode_authorities_and_temporal_absence() {
    for kind in [ResourceKind::Revocation, ResourceKind::Appeal] {
        let mut declared = input();
        declared.kind = kind;
        declared.mode = FactDeclaration::Unknown(text("Mode is not known"));
        declared.resolution_reference = FactDeclaration::Unknown(text("Reference not available"));
        declared.receiving_authority = Some(FactDeclaration::Unknown(text("Recipient not known")));
        declared.notification_at = Some(DeclaredProceduralTime::unknown());
        let values = ResourceValues::new(declared.clone()).unwrap();
        assert_eq!(values.mode(), &declared.mode);
        assert_eq!(values.issuing_authority(), &declared.issuing_authority);
        assert_eq!(
            values.receiving_authority(),
            declared.receiving_authority.as_ref()
        );
        assert_eq!(
            values.resolution_reference(),
            &declared.resolution_reference
        );
        assert_eq!(
            values.resolution_at().precision(),
            DeclaredProceduralPrecision::Unknown
        );
        assert_eq!(
            values.notification_at(),
            Some(DeclaredProceduralTime::unknown())
        );
        assert!(values.resolution_at().instant_value().is_none());
    }
    let values = ResourceValues::new(input()).unwrap();
    assert_eq!(values.receiving_authority(), None);
    assert_eq!(values.notification_at(), None);
}

#[test]
fn captured_people_keep_input_order_and_equal_names_without_equating_identity() {
    let mut declared = input();
    declared.appellants = vec![
        appellant(Some(9), 2, "Same name"),
        appellant(None, 1, "Same name"),
        appellant(Some(1), 3, "Same name"),
        appellant(None, 1, "Same name"),
    ];
    assert_eq!(
        ResourceValues::new(declared.clone()).unwrap().appellants(),
        declared.appellants
    );
    for revision in [2, 3] {
        let mut duplicate = declared.clone();
        duplicate
            .appellants
            .push(appellant(Some(9), revision, "Another declared name"));
        assert_eq!(
            ResourceValues::new(duplicate),
            Err(DomainError::InvalidProceduralResource("appellants"))
        );
    }
}

#[test]
fn appellant_count_is_a_bounded_record_requirement_without_name_deduplication() {
    assert_eq!(MAX_RESOURCE_APPELLANTS, 32);
    let mut declared = input();
    declared.appellants.clear();
    assert!(ResourceValues::new(declared.clone()).is_err());
    declared.appellants = (0..32)
        .map(|_| appellant(None, 1, "Declared name"))
        .collect();
    assert!(ResourceValues::new(declared.clone()).is_ok());
    declared
        .appellants
        .push(appellant(None, 1, "Declared name"));
    assert!(ResourceValues::new(declared).is_err());
}

#[test]
fn declared_acts_need_exact_evidence_but_no_inferred_time_or_legal_sequence() {
    for kind in [
        ResourceActKind::Interposition,
        ResourceActKind::Admission,
        ResourceActKind::Inadmissibility,
        ResourceActKind::Withdrawal,
        ResourceActKind::Resolution,
    ] {
        let mut declared = act_input();
        declared.kind = kind;
        declared.mode = FactDeclaration::Unknown(text("Mode not captured"));
        let act = ResourceActValues::new(declared.clone()).unwrap();
        assert_eq!(act.kind(), kind);
        assert_eq!(act.mode(), &declared.mode);
        assert_eq!(act.occurred_at(), DeclaredProceduralTime::unknown());
        assert_eq!(act.authority(), &declared.authority);
        assert_eq!(act.statement(), &declared.statement);
        assert_eq!(act.evidence(), declared.evidence);
        declared.evidence.clear();
        assert_eq!(
            ResourceActValues::new(declared),
            Err(DomainError::InvalidProceduralResource("act_evidence"))
        );
    }
}

#[test]
fn support_admission_deduplicates_exact_content_without_losing_each_locator() {
    let mut declared = act_input();
    declared.evidence = vec![evidence(1, 2, 3, "Page 1"), evidence(1, 2, 3, "Page 2")];
    let act = ResourceActValues::new(declared.clone()).unwrap();
    assert_eq!(act.evidence(), declared.evidence);
    assert_eq!(act.direct_supports(), vec![declared.evidence[0].support()]);
    declared.evidence[1] = evidence(1, 2, 99, "Conflicting expectation");
    assert_eq!(
        ResourceActValues::new(declared),
        Err(DomainError::InvalidProceduralResource("support_digest"))
    );
    assert_eq!(MAX_RESOURCE_ACT_EVIDENCE, 2);
    let mut declared = act_input();
    declared.evidence = (0..2).map(|id| evidence(id, 1, 3, "Locator")).collect();
    assert!(ResourceActValues::new(declared.clone()).is_ok());
    declared.evidence.push(evidence(3, 1, 3, "Locator"));
    assert!(ResourceActValues::new(declared).is_err());
}

#[test]
fn oral_interposition_preserves_minute_precision_and_original_offset() {
    let mut declared = act_input();
    declared.mode = FactDeclaration::Known(ResourceMode::Oral);
    declared.occurred_at = DeclaredProceduralTime::minute(
        "2026-09-19".parse().unwrap(),
        12,
        30,
        Some(UtcOffset::from_hms(-6, 0, 0).unwrap()),
    )
    .unwrap();
    let act = ResourceActValues::new(declared).unwrap();
    assert_eq!(act.mode(), &FactDeclaration::Known(ResourceMode::Oral));
    assert_eq!(act.occurred_at().local_second(), None);
    assert_eq!(act.occurred_at().offset().unwrap().whole_seconds(), -21600);
    assert!(act.occurred_at().instant_value().is_none());
}
