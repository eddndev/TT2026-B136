mod procedural_fact_support;
mod procedural_resource_support;
use domain::{
    procedural_facts::*, procedural_resources::*, procedural_time::DeclaredProceduralTime,
};
use procedural_fact_support::{evidence, label, text};
use procedural_resource_support::*;

#[test]
fn each_resource_field_changes_its_commitment_without_rebinding_historical_references() {
    let base = input();
    let original = ResourceValues::new(base.clone()).unwrap().canonical_bytes();
    let mut changes = Vec::new();
    let mut value = base.clone();
    value.kind = ResourceKind::Appeal;
    changes.push(value);
    let mut value = base.clone();
    value.mode = FactDeclaration::Known(ResourceMode::Oral);
    changes.push(value);
    let mut value = base.clone();
    value.title = label("Another title");
    changes.push(value);
    let mut value = base.clone();
    value.resolution.id = ResolutionId::new();
    changes.push(value);
    let mut value = base.clone();
    value.resolution.revision = FactRevision::new(3).unwrap();
    changes.push(value);
    let mut value = base.clone();
    value.resolution_evidence = evidence(21, 3, 42, "Order page 2");
    changes.push(value);
    let mut value = base.clone();
    value.resolution_evidence = evidence(20, 4, 42, "Order page 2");
    changes.push(value);
    let mut value = base.clone();
    value.resolution_evidence = evidence(20, 3, 43, "Order page 2");
    changes.push(value);
    let mut value = base.clone();
    value.resolution_evidence = evidence(20, 3, 42, "Order page 3");
    changes.push(value);
    let mut value = base.clone();
    value.resolution_reference = FactDeclaration::Known(label("Another reference"));
    changes.push(value);
    let mut value = base.clone();
    value.issuing_authority = FactDeclaration::Known(label("Named issuer"));
    changes.push(value);
    let mut value = base.clone();
    value.receiving_authority = Some(FactDeclaration::Known(label("Named receiver")));
    changes.push(value);
    let mut value = base.clone();
    value.resolution_at =
        DeclaredProceduralTime::date("2026-01-01".parse().unwrap(), None).unwrap();
    changes.push(value);
    let mut value = base.clone();
    value.notification_at = Some(DeclaredProceduralTime::unknown());
    changes.push(value);
    let mut value = base.clone();
    value.challenged_part = text("Another paragraph");
    changes.push(value);
    let mut value = base.clone();
    value.grounds = text("Other grounds");
    changes.push(value);
    let mut value = base.clone();
    value.appellants = vec![appellant(Some(30), 4, "Another name")];
    changes.push(value);
    let mut value = base.clone();
    value.appellants = vec![appellant(Some(30), 5, "Captured name")];
    changes.push(value);
    let mut value = base.clone();
    value.appellants = vec![appellant(None, 1, "Captured name")];
    changes.push(value);
    let mut value = base.clone();
    value.appellants = vec![ResourceAppellant::new(
        label("Captured name"),
        FactDeclaration::Unknown(text("Role not known")),
        base.appellants[0].participant(),
    )];
    changes.push(value);
    for changed in changes {
        assert_ne!(
            original,
            ResourceValues::new(changed).unwrap().canonical_bytes()
        );
    }
    let mut ordered = base.clone();
    ordered.appellants.push(appellant(None, 1, "Other name"));
    let first = ResourceValues::new(ordered.clone())
        .unwrap()
        .canonical_bytes();
    ordered.appellants.reverse();
    assert_ne!(
        first,
        ResourceValues::new(ordered).unwrap().canonical_bytes()
    );
}

#[test]
fn act_fields_and_support_locators_are_bound_even_with_one_admitted_content_version() {
    let base = act_input();
    let original = ResourceActValues::new(base.clone())
        .unwrap()
        .canonical_bytes();
    let mut changes = Vec::new();
    let mut value = base.clone();
    value.kind = ResourceActKind::Withdrawal;
    changes.push(value);
    let mut value = base.clone();
    value.mode = FactDeclaration::Known(ResourceMode::Oral);
    changes.push(value);
    let mut value = base.clone();
    value.authority = FactDeclaration::Known(label("Court"));
    changes.push(value);
    let mut value = base.clone();
    value.statement = text("Another declaration");
    changes.push(value);
    let mut value = base.clone();
    value.occurred_at = DeclaredProceduralTime::date("2026-09-19".parse().unwrap(), None).unwrap();
    changes.push(value);
    let mut value = base.clone();
    value.evidence = vec![evidence(40, 2, 21, "Another locator")];
    changes.push(value);
    for changed in changes {
        assert_ne!(
            original,
            ResourceActValues::new(changed).unwrap().canonical_bytes()
        );
    }
    let mut value = base;
    value.evidence.push(evidence(40, 2, 21, "Second locator"));
    let act = ResourceActValues::new(value.clone()).unwrap();
    assert_eq!(act.direct_supports().len(), 1);
    assert_ne!(original, act.canonical_bytes());
    value.evidence.reverse();
    assert_ne!(
        act.canonical_bytes(),
        ResourceActValues::new(value).unwrap().canonical_bytes()
    );
}
