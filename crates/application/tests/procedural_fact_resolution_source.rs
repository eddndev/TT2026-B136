#[allow(dead_code)]
#[path = "procedural_fact_hearing_support/mod.rs"]
mod hearing_support;
#[path = "procedural_fact_resolution_support/mod.rs"]
mod support;
use application::{documents::StageDocumentFormat, procedural_facts::*};
use domain::{
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion},
    identity::UserId,
};
use support::*;
use uuid::Uuid;

#[test]
fn empty_parent_selection_needs_no_material_or_hash_work() {
    let hasher = Hasher::default();
    assert_eq!(
        resolve_fact_resolution(&hasher, case_id(), &selection(None), None).unwrap(),
        None
    );
    assert!(hasher.seen.borrow().is_empty());
}

#[test]
fn exact_parent_projects_its_values_and_both_distinct_digests() {
    let fixture = Fixture::new(1, false, None, None);
    let source = &fixture.material.snapshot;
    let hasher = fixture.hasher();
    let result = resolve_fact_resolution(
        &hasher,
        case_id(),
        &selection(Some(reference(1))),
        Some(&fixture.material),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        result.snapshot,
        FactResolutionSourceSnapshot {
            case_id: case_id(),
            reference: reference(1),
            values_digest: source.metadata.values_digest,
            submission_digest: source.metadata.receipt.submission_digest,
            status: FactStatus::Recorded,
        }
    );
    assert_ne!(
        result.snapshot.values_digest,
        result.snapshot.submission_digest
    );
    assert_eq!(
        result.view,
        FactResolutionView {
            reference: reference(1),
            class: source.values.class().clone(),
            issuer: source.values.issuer().clone(),
            issued_at: source.values.issued_at(),
            summary: source.values.summary().clone(),
        }
    );
    let seen = hasher.seen.borrow();
    for prefix in [b"PFRES1", b"PFSRC1", b"PFTXN1"] {
        assert!(seen.iter().any(|bytes| bytes.starts_with(prefix)));
    }
}

#[test]
fn historical_withdrawal_preserves_unknown_declarations_and_exact_revision() {
    let mut fixture = Fixture::new(7, true, None, None);
    fixture.material.snapshot.values = ResolutionValues::new(ResolutionValuesInput {
        class: FactDeclaration::Unknown(text("Classification not recorded")),
        subtype: None,
        issuer: FactDeclaration::Unknown(text("Issuer not recorded")),
        issued_at: domain::procedural_time::DeclaredProceduralTime::unknown(),
        summary: text("Historical withdrawn declaration"),
        provenance: FactProvenance::OperatorNote {
            note: text("Historical note"),
        },
    });
    let result = resolve_fact_resolution(
        &fixture.hasher(),
        case_id(),
        &selection(Some(reference(7))),
        Some(&fixture.material),
    )
    .unwrap()
    .unwrap();
    assert_eq!(result.snapshot.status, FactStatus::Withdrawn);
    assert_eq!(result.snapshot.reference, reference(7));
    assert_eq!(result.view.class, *fixture.material.snapshot.values.class());
    assert_eq!(
        result.view.issuer,
        *fixture.material.snapshot.values.issuer()
    );
    assert_eq!(
        result.view.issued_at,
        fixture.material.snapshot.values.issued_at()
    );
}

#[test]
fn missing_or_unselected_parent_is_inconsistent() {
    let fixture = Fixture::new(1, false, None, None);
    assert_inconsistent(resolve_fact_resolution(
        &fixture.hasher(),
        case_id(),
        &selection(Some(reference(1))),
        None,
    ));
    assert_inconsistent(resolve_fact_resolution(
        &fixture.hasher(),
        case_id(),
        &selection(None),
        Some(&fixture.material),
    ));
}

#[test]
fn wrong_case_identity_and_revision_reject_self_consistent_snapshots() {
    for mode in 0..3 {
        let mut fixture = Fixture::new(if mode == 2 { 2 } else { 1 }, false, None, None);
        let snapshot = &mut fixture.material.snapshot;
        match mode {
            0 => {
                snapshot.root =
                    ResolutionRoot::new(snapshot.root.id(), CaseId::from_uuid(Uuid::nil()))
            }
            1 => {
                snapshot.root = ResolutionRoot::new(ResolutionId::from_uuid(Uuid::nil()), case_id())
            }
            _ => {}
        }
        let hasher = fixture.hasher();
        fact_snapshot_receipt_matches(
            &hasher,
            &ProceduralFactSnapshot::Resolution(Box::new(fixture.material.snapshot.clone())),
        )
        .unwrap();
        assert_rejected(&hasher, &fixture.material);
    }
}

#[test]
fn altered_values_cannot_reuse_the_original_digest_and_receipt() {
    let mut fixture = Fixture::new(1, false, None, None);
    let hasher = fixture.hasher();
    fixture.material.snapshot.values = values(FactProvenance::OperatorNote {
        note: text("Altered source"),
    });
    assert_rejected(&hasher, &fixture.material);
}

#[test]
fn altered_receipt_fields_cannot_reuse_a_recorded_envelope() {
    for mode in 0..8 {
        let mut fixture = Fixture::new(1, false, None, None);
        let hasher = fixture.hasher();
        let metadata = &mut fixture.material.snapshot.metadata;
        match mode {
            0 => metadata.values_digest = digest(60),
            1 => metadata.receipt.sources_digest = digest(60),
            2 => metadata.receipt.submission_digest = digest(60),
            3 => metadata.recorded_by.id = UserId::from_uuid(Uuid::nil()),
            4 => metadata.receipt.operation_id = FactOperationId::from_uuid(Uuid::nil()),
            5 => metadata.receipt.expected_revision = 1,
            6 => metadata.status = FactStatus::Withdrawn,
            _ => metadata.reason = Some(text("Unexpected reason")),
        }
        assert_rejected(&hasher, &fixture.material);
    }
}

#[test]
fn exact_hearing_and_its_historical_document_do_not_require_readmission() {
    let hearing = hearing_support::source(5, 6, 2, true, &[0]);
    let fixture = Fixture::new(3, false, Some(hearing), Some(support()));
    let hasher = fixture.hasher();
    assert_ne!(
        fixture
            .material
            .admitted_support
            .as_ref()
            .unwrap()
            .reference,
        fixture
            .material
            .hearing
            .as_ref()
            .unwrap()
            .values
            .provenance()
            .support()
            .unwrap()
            .reference()
    );
    let result = resolve_fact_resolution(
        &hasher,
        case_id(),
        &selection(Some(reference(3))),
        Some(&fixture.material),
    )
    .unwrap()
    .unwrap();
    assert_eq!(result.snapshot.reference, reference(3));
    assert_eq!(
        result.view.summary,
        *fixture.material.snapshot.values.summary()
    );
    let seen = hasher.seen.borrow();
    assert!(seen.iter().any(|bytes| bytes.starts_with(b"HRES1")));
    assert!(seen.iter().any(|bytes| bytes.starts_with(b"HRTX1")));
}

#[test]
fn parent_hearing_material_must_be_present_exactly_when_declared() {
    let mut absent = Fixture::new(
        1,
        false,
        Some(hearing_support::source(5, 6, 1, false, &[])),
        None,
    );
    let hasher = absent.hasher();
    absent.material.hearing = None;
    assert_rejected(&hasher, &absent.material);
    let mut extra = Fixture::new(1, false, None, None);
    extra.material.hearing = Some(hearing_support::source(5, 6, 1, false, &[]));
    assert_rejected(&extra.hasher(), &extra.material);
}

#[test]
fn another_valid_hearing_revision_cannot_replace_the_captured_source() {
    let mut fixture = Fixture::new(
        1,
        false,
        Some(hearing_support::source(5, 6, 1, false, &[7])),
        None,
    );
    fixture.material.hearing = Some(hearing_support::source(5, 6, 2, false, &[7]));
    let hasher = fixture.hasher();
    application::hearing_results::hearing_result_snapshot_receipt_matches(
        &hasher,
        fixture.material.hearing.as_ref().unwrap(),
    )
    .unwrap();
    assert_rejected(&hasher, &fixture.material);
}

#[test]
fn altered_hearing_values_are_rejected_before_projecting_the_parent() {
    let mut fixture = Fixture::new(
        1,
        false,
        Some(hearing_support::source(5, 6, 1, false, &[])),
        None,
    );
    let hasher = fixture.hasher();
    let hearing = fixture.material.hearing.as_mut().unwrap();
    let mut input = hearing_support::values_input(&hearing.values);
    input.summary =
        application::hearing_results::HearingResultText::new("Altered session").unwrap();
    hearing.values = application::hearing_results::HearingResultValues::new(input).unwrap();
    assert_rejected(&hasher, &fixture.material);
}

#[test]
fn an_agreement_must_exist_in_the_captured_result_revision() {
    let mut fixture = Fixture::new(
        1,
        false,
        Some(hearing_support::source(5, 6, 1, false, &[7])),
        None,
    );
    fixture.material.hearing = Some(hearing_support::source(5, 6, 1, false, &[]));
    assert_rejected(&fixture.hasher(), &fixture.material);
}

#[test]
fn historical_support_must_be_present_exactly_when_declared() {
    let mut absent = Fixture::new(1, false, None, Some(support()));
    let hasher = absent.hasher();
    absent.material.admitted_support = None;
    assert_rejected(&hasher, &absent.material);
    let mut extra = Fixture::new(1, false, None, None);
    extra.material.admitted_support = Some(support());
    extra.sources.direct_supports = vec![support()];
    assert_rejected(&extra.hasher(), &extra.material);
}

#[test]
fn incompatible_support_identity_version_or_digest_reject_even_resigned_sources() {
    for mode in 0..3 {
        let mut fixture = Fixture::new(1, false, None, Some(support()));
        let material = fixture.material.admitted_support.as_mut().unwrap();
        match mode {
            0 => material.reference.id = DocumentId::from_uuid(Uuid::nil()),
            1 => material.reference.version = DocumentVersion::initial(),
            _ => material.digest = digest(60),
        }
        fixture.sources.direct_supports = vec![material.clone()];
        let hasher = fixture.hasher();
        fact_snapshot_receipt_matches(
            &hasher,
            &ProceduralFactSnapshot::Resolution(Box::new(fixture.material.snapshot.clone())),
        )
        .unwrap();
        assert_rejected(&hasher, &fixture.material);
    }
}

#[test]
fn historical_support_name_and_format_are_bound_by_the_sources_receipt() {
    for format in [false, true] {
        let mut fixture = Fixture::new(1, false, None, Some(support()));
        let hasher = fixture.hasher();
        let material = fixture.material.admitted_support.as_mut().unwrap();
        if format {
            material.format = StageDocumentFormat::Docx;
        } else {
            material.name = "renamed.pdf".into();
        }
        assert_rejected(&hasher, &fixture.material);
    }
}

#[test]
fn forged_readable_hearing_view_cannot_match_the_reconstructed_sources() {
    let mut fixture = Fixture::new(
        1,
        false,
        Some(hearing_support::source(5, 6, 1, false, &[7])),
        None,
    );
    fixture.sources.views.hearing_results[0].summary =
        application::hearing_results::HearingResultText::new("Forged readable summary").unwrap();
    let hasher = fixture.hasher();
    fact_snapshot_receipt_matches(
        &hasher,
        &ProceduralFactSnapshot::Resolution(Box::new(fixture.material.snapshot.clone())),
    )
    .unwrap();
    assert_rejected(&hasher, &fixture.material);
}

#[test]
fn forged_compact_hearing_digest_cannot_match_the_reconstructed_sources() {
    let mut fixture = Fixture::new(
        1,
        false,
        Some(hearing_support::source(5, 6, 1, false, &[])),
        None,
    );
    fixture.sources.resolved.hearing_results[0].values_digest = digest(60);
    let hasher = fixture.hasher();
    fact_snapshot_receipt_matches(
        &hasher,
        &ProceduralFactSnapshot::Resolution(Box::new(fixture.material.snapshot.clone())),
    )
    .unwrap();
    assert_rejected(&hasher, &fixture.material);
}
