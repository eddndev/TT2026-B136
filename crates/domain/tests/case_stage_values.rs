use domain::case_stages::{
    CaseStage, CaseStageChange, CaseStageRevision, DeclaredStageTime, StageAdoption, StageCourt,
    StageNote, StageReceiptReference, StageSupportRef, StageTransition,
};
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::DomainError;
use time::macros::datetime;

fn at() -> DeclaredStageTime {
    DeclaredStageTime::instant(datetime!(2026-09-15 06:00 UTC)).unwrap()
}
fn support(version: u32, digest: u8) -> StageSupportRef {
    StageSupportRef::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(uuid::Uuid::nil()),
            version: DocumentVersion::new(version).unwrap(),
        },
        Sha256Digest::from_array([digest; 32]),
    )
}

#[test]
fn only_two_forward_edges_exist() {
    let stages = [
        CaseStage::Investigation,
        CaseStage::Intermediate,
        CaseStage::Trial,
    ];
    for from in stages {
        for to in stages {
            assert_eq!(
                from.permits(to),
                matches!(
                    (from, to),
                    (CaseStage::Investigation, CaseStage::Intermediate)
                        | (CaseStage::Intermediate, CaseStage::Trial)
                )
            );
        }
        assert_eq!(from.as_str().parse::<CaseStage>().unwrap(), from);
    }
    for unknown in ["", "Investigation", "trial ", "closed", "appeal"] {
        assert_eq!(
            unknown.parse::<CaseStage>(),
            Err(DomainError::InvalidCaseStage)
        );
    }
}

#[test]
fn stage_revisions_are_positive_and_cannot_wrap() {
    assert_eq!(
        CaseStageRevision::new(0),
        Err(DomainError::InvalidCaseStageRevision)
    );
    assert_eq!(CaseStageRevision::FIRST.next().unwrap().get(), 2);
    assert_eq!(CaseStageRevision::new(u32::MAX).unwrap().next(), None);
}

#[test]
fn notes_normalize_multiline_input_and_trim_unicode() {
    let value = StageNote::new("\u{2003}First\r\nSecond\u{2003}").unwrap();
    assert_eq!(value.as_str(), "First\nSecond");
    assert_eq!(StageNote::optional(Some(" \r\n ")).unwrap(), None);
    assert_eq!(StageNote::optional(None).unwrap(), None);
    assert_eq!(
        StageNote::optional(Some(" text "))
            .unwrap()
            .unwrap()
            .as_str(),
        "text"
    );
    for bad in [
        "",
        "  ",
        "\r",
        "text\rnext",
        "\ttext",
        "\0text",
        "text\u{85}",
    ] {
        assert_eq!(StageNote::new(bad), Err(DomainError::InvalidStageNote));
    }
}

#[test]
fn every_text_limit_counts_unicode_scalars() {
    let note = "\u{1f4dc}".repeat(1000);
    assert_eq!(StageNote::new(&note).unwrap().as_str(), note);
    assert!(StageNote::new(&(note + "x")).is_err());
    let short = "\u{1f4dc}".repeat(200);
    assert_eq!(StageCourt::new(&short).unwrap().as_str(), short);
    assert_eq!(StageReceiptReference::new(&short).unwrap().as_str(), short);
    assert!(StageCourt::new(&(short.clone() + "x")).is_err());
    assert!(StageReceiptReference::new(&(short + "x")).is_err());
}

#[test]
fn single_line_values_reject_controls_before_trimming() {
    assert_eq!(
        StageCourt::new("\u{2003}Court\u{2003}").unwrap().as_str(),
        "Court"
    );
    assert_eq!(StageReceiptReference::new(" Ref ").unwrap().as_str(), "Ref");
    assert_eq!(StageReceiptReference::optional(Some("  ")).unwrap(), None);
    assert_eq!(StageReceiptReference::optional(None).unwrap(), None);
    for value in ["", " ", "\nCourt", "Court\r\n", "\tCourt", "Court\u{7f}"] {
        assert_eq!(StageCourt::new(value), Err(DomainError::InvalidStageCourt));
        assert_eq!(
            StageReceiptReference::new(value),
            Err(DomainError::InvalidStageReceiptReference)
        );
    }
    assert!(StageReceiptReference::optional(Some("\n")).is_err());
    assert!(StageNote::optional(Some("\t")).is_err());
}

#[test]
fn adoption_retains_declared_stage_reason_and_exact_support() {
    for stage in [
        CaseStage::Investigation,
        CaseStage::Intermediate,
        CaseStage::Trial,
    ] {
        let support = support(7, 3);
        let adoption = StageAdoption::new(
            stage,
            at(),
            StageNote::new("Known history").unwrap(),
            support,
        );
        assert_eq!(adoption.stage(), stage);
        assert_eq!(adoption.known_at(), at());
        assert_eq!(adoption.reason().as_str(), "Known history");
        assert_eq!(adoption.support(), support);
        let change = CaseStageChange::Adopt(adoption);
        assert_eq!(change.stage(), stage);
        assert_eq!(change.supports(), vec![support]);
    }
}

#[test]
fn both_transition_variants_preserve_typed_fields() {
    let intermediate = StageTransition::to_intermediate(
        at(),
        support(1, 1),
        Some(StageNote::new("Accusation").unwrap()),
    );
    let StageTransition::ToIntermediate(values) = &intermediate else {
        panic!("wrong transition")
    };
    assert_eq!(values.accusation_declared_at(), at());
    assert_eq!(values.accusation(), support(1, 1));
    assert_eq!(values.note().unwrap().as_str(), "Accusation");
    assert_eq!(
        CaseStageChange::Transition(intermediate).stage(),
        CaseStage::Intermediate
    );
    let trial = StageTransition::to_trial(
        at(),
        support(1, 1),
        at(),
        StageCourt::new("Court").unwrap(),
        Some(StageReceiptReference::new("Receipt").unwrap()),
        Some(support(2, 2)),
        Some(StageNote::new("Trial").unwrap()),
    )
    .unwrap();
    let StageTransition::ToTrial(values) = &trial else {
        panic!("wrong transition")
    };
    assert_eq!(values.opening_order_issued_at(), at());
    assert_eq!(values.opening_order(), support(1, 1));
    assert_eq!(values.received_at(), at());
    assert_eq!(values.receiving_court().as_str(), "Court");
    assert_eq!(values.receipt_reference().unwrap().as_str(), "Receipt");
    assert_eq!(values.receipt_support(), Some(support(2, 2)));
    assert_eq!(values.note().unwrap().as_str(), "Trial");
    let change = CaseStageChange::Transition(trial);
    assert_eq!(change.stage(), CaseStage::Trial);
    assert_eq!(change.supports(), vec![support(1, 1), support(2, 2)]);
}

#[test]
fn identical_exact_support_is_validated_once_without_losing_roles() {
    let reference = support(7, 1);
    assert_eq!(reference.reference().version.get(), 7);
    assert_eq!(reference.digest(), Sha256Digest::from_array([1; 32]));
    let trial = StageTransition::to_trial(
        at(),
        reference,
        at(),
        StageCourt::new("Court").unwrap(),
        None,
        Some(reference),
        None,
    )
    .unwrap();
    let StageTransition::ToTrial(values) = &trial else {
        panic!("wrong transition")
    };
    assert_eq!(values.opening_order(), reference);
    assert_eq!(values.receipt_support(), Some(reference));
    assert_eq!(
        CaseStageChange::Transition(trial).supports(),
        vec![reference]
    );
}

#[test]
fn one_exact_reference_cannot_claim_two_content_digests() {
    assert_eq!(
        StageTransition::to_trial(
            at(),
            support(7, 1),
            at(),
            StageCourt::new("Court").unwrap(),
            None,
            Some(support(7, 2)),
            None
        ),
        Err(DomainError::ConflictingStageSupport)
    );
}
