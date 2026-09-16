#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod procedural_fact_service_support;
use application::{
    cases::{case_administration_digest, CaseAdministrationSnapshot, CurrentCaseAdministration},
    procedural_facts::*,
};
use domain::{
    case_administration::{CaseAdministrationValues, CaseAdministrativeStatus, CaseRevision},
    cases::CaseId,
    identity::UserId,
};
use procedural_fact_service_support::*;

#[test]
fn submission_binds_actor_scope_target_operation_action_digests_and_reason() {
    let actor = UserId::new();
    let case = CaseId::new();
    let cmd = command();
    let expected = fact_submission_bytes(actor, case, &cmd, digest(1), digest(2)).unwrap();
    assert_eq!(expected.len(), 141);
    assert_eq!(&expected[..6], b"PFTXN1");
    for (a, c, value, sources) in [
        (UserId::new(), case, digest(1), digest(2)),
        (actor, CaseId::new(), digest(1), digest(2)),
        (actor, case, digest(2), digest(2)),
        (actor, case, digest(1), digest(1)),
    ] {
        assert_ne!(
            expected,
            fact_submission_bytes(a, c, &cmd, value, sources).unwrap()
        );
    }
    let FactTarget::Resolution(id) = cmd.target() else {
        unreachable!()
    };
    for change in [
        FactChange::correct(FactRevision::initial(), values(), text("Correction")),
        FactChange::withdraw(FactRevision::initial(), text("Withdrawal")),
    ] {
        let changed = ProceduralFactCommand::Resolution(ResolutionCommand::new(
            cmd.operation_id(),
            id,
            change,
        ));
        assert_ne!(
            expected,
            fact_submission_bytes(actor, case, &changed, digest(1), digest(2)).unwrap()
        );
    }
}
#[test]
fn exhausted_revision_cannot_produce_submission() {
    let cmd = ProceduralFactCommand::Resolution(ResolutionCommand::new(
        FactOperationId::new(),
        ResolutionId::new(),
        FactChange::withdraw(FactRevision::new(u32::MAX).unwrap(), text("Withdraw")),
    ));
    assert!(matches!(
        fact_submission_bytes(UserId::new(), CaseId::new(), &cmd, digest(1), digest(2)),
        Err(application::ApplicationError::ProceduralFact(
            ProceduralFactError::RevisionExhausted
        ))
    ));
}
#[test]
fn complete_receipt_detects_snapshot_and_operation_tampering() {
    let base = detail(UserId::new(), CaseId::new(), &command(), values(), empty());
    fact_receipt_matches(hasher().as_ref(), &base).unwrap();
    fact_history_receipt_matches(hasher().as_ref(), &FactHistoryEntry::from(&base.snapshot))
        .unwrap();
    for mode in 0..14 {
        let mut changed = base.clone();
        let s = snapshot_mut(&mut changed);
        match mode {
            0 => s.root = ResolutionRoot::new(ResolutionId::new(), s.root.case_id()),
            1 => s.root = ResolutionRoot::new(s.root.id(), CaseId::new()),
            2 => s.metadata.recorded_by.id = UserId::new(),
            3 => s.metadata.receipt.operation_id = FactOperationId::new(),
            4 => s.metadata.receipt.action = FactAction::Correct,
            5 => s.metadata.receipt.expected_revision = 1,
            6 => s.metadata.revision = FactRevision::new(2).unwrap(),
            7 => s.metadata.status = FactStatus::Withdrawn,
            8 => s.metadata.reason = Some(text("Injected reason")),
            9 => s.metadata.values_digest = digest(77),
            10 => s.metadata.receipt.sources_digest = digest(78),
            11 => s.metadata.receipt.submission_digest = digest(79),
            12 => {
                s.values = ResolutionValues::new(ResolutionValuesInput {
                    class: s.values.class().clone(),
                    subtype: None,
                    issuer: s.values.issuer().clone(),
                    issued_at: s.values.issued_at(),
                    summary: text("Different declaration"),
                    provenance: s.values.provenance().clone(),
                })
            }
            _ => s.metadata.receipt.expected_revision = u32::MAX,
        }
        assert_inconsistent(fact_receipt_matches(hasher().as_ref(), &changed));
    }
}
#[test]
fn correct_and_withdraw_receipts_are_valid_without_time_inference() {
    let id = ResolutionId::new();
    let actor = UserId::new();
    let case = CaseId::new();
    for change in [
        FactChange::correct(FactRevision::new(7).unwrap(), values(), text("Correct")),
        FactChange::withdraw(FactRevision::new(7).unwrap(), text("Withdraw")),
    ] {
        let cmd = ProceduralFactCommand::Resolution(ResolutionCommand::new(
            FactOperationId::new(),
            id,
            change,
        ));
        let mut value = detail(actor, case, &cmd, values(), empty());
        snapshot_mut(&mut value).metadata.recorded_at -= time::Duration::days(1000);
        fact_receipt_matches(hasher().as_ref(), &value).unwrap();
    }
}
#[test]
fn extra_sources_are_rejected_even_when_their_digest_is_recomputed() {
    let case = CaseId::new();
    let mut sources = empty();
    sources
        .direct_supports
        .push(application::case_stages::StageSupportSnapshot {
            reference: domain::crypto::DocumentVersionRef {
                id: domain::crypto::DocumentId::new(),
                version: domain::crypto::DocumentVersion::initial(),
            },
            digest: digest(5),
            name: "file.pdf".into(),
            format: application::documents::StageDocumentFormat::Pdf,
            policy: application::documents::StageFormatPolicy::PdfDocxV1,
        });
    let value = detail(UserId::new(), case, &command(), values(), sources);
    assert_inconsistent(fact_receipt_matches(hasher().as_ref(), &value));
}
#[test]
fn historical_closed_administration_is_readable_but_its_digest_and_case_are_checked() {
    let case = CaseId::new();
    let mut value = detail(UserId::new(), case, &command(), values(), empty());
    let values =
        CaseAdministrationValues::basic(domain::cases::CaseMetadata::new("Case", "REF-1").unwrap())
            .with_status(CaseAdministrativeStatus::Closed);
    let s = snapshot_mut(&mut value);
    s.metadata.recorded_administration =
        CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
            case_id: case,
            revision: CaseRevision::new(1).unwrap(),
            values_digest: case_administration_digest(hasher().as_ref(), &values),
            values,
            changed_at: s.metadata.recorded_at,
            changed_by: s.metadata.recorded_by.clone(),
        }));
    fact_receipt_matches(hasher().as_ref(), &value).unwrap();
    let CurrentCaseAdministration::Recorded(admin) =
        &mut snapshot_mut(&mut value).metadata.recorded_administration
    else {
        unreachable!()
    };
    admin.case_id = CaseId::new();
    assert_inconsistent(fact_receipt_matches(hasher().as_ref(), &value));
}
