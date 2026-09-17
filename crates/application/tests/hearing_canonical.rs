#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod hearing_support;

use application::{hearings::*, ApplicationError};
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    cases::CaseId,
    crypto::Sha256Digest,
    identity::UserId,
};
use hearing_support::*;
use uuid::Uuid;

fn fixed(action: HearingAction) -> HearingCommand {
    let context = HearingContextExpectation {
        case_revision: CaseRevision::new(11).unwrap(),
        stage_revision: CaseStageRevision::new(13).unwrap(),
    };
    let reason = HearingNote::new(" Motivo\r\n\u{e1} ").unwrap();
    let expected_revision = HearingRevision::new(7).unwrap();
    HearingCommand {
        operation_id: HearingOperationId::from_uuid(Uuid::from_u128(1)),
        hearing_id: HearingId::from_uuid(Uuid::from_u128(4)),
        change: match action {
            HearingAction::Schedule => HearingChange::Schedule {
                context,
                values: values(),
            },
            HearingAction::Replace => HearingChange::Replace {
                expected_revision,
                context,
                values: values(),
                reason,
            },
            HearingAction::Cancel => HearingChange::Cancel {
                expected_revision,
                reason,
            },
        },
    }
}

#[test]
fn all_action_encodings_match_independent_python_bytes() {
    let actor = UserId::from_uuid(Uuid::from_u128(2));
    let case_id = CaseId::from_uuid(Uuid::from_u128(3));
    let digest = Sha256Digest::from_array(std::array::from_fn(|i| i as u8));
    let vectors = include_str!("hearing_support/wire-vectors.txt")
        .lines()
        .collect::<Vec<_>>();
    for (index, action) in [
        HearingAction::Schedule,
        HearingAction::Replace,
        HearingAction::Cancel,
    ]
    .into_iter()
    .enumerate()
    {
        let command = fixed(action);
        let bytes = hearing_submission_bytes(actor, case_id, &command, digest);
        let hex = bytes.iter().map(|v| format!("{v:02x}")).collect::<String>();
        assert_eq!(hex, vectors[index]);
        assert_eq!(action.tag(), index as u8);
        assert_eq!(action.as_str(), ["schedule", "replace", "cancel"][index]);
        assert_eq!(
            command.result_revision().unwrap().get(),
            if index == 0 { 1 } else { 8 }
        );
        assert_eq!(
            hearing_submission_digest(hasher().as_ref(), actor, case_id, &command, digest),
            hasher().hash_bytes(&bytes)
        );
    }
}

#[test]
fn actor_case_operation_hearing_action_context_reason_and_values_are_bound() {
    let actor = UserId::new();
    let case_id = CaseId::new();
    let digest = Sha256Digest::from_array([7; 32]);
    let base = fixed(HearingAction::Replace);
    let expected = hearing_submission_bytes(actor, case_id, &base, digest);
    assert_ne!(
        expected,
        hearing_submission_bytes(UserId::new(), case_id, &base, digest)
    );
    assert_ne!(
        expected,
        hearing_submission_bytes(actor, CaseId::new(), &base, digest)
    );
    assert_ne!(
        expected,
        hearing_submission_bytes(actor, case_id, &base, Sha256Digest::from_array([8; 32]))
    );
    for mode in 0..7 {
        let mut changed = base.clone();
        match mode {
            0 => changed.operation_id = HearingOperationId::new(),
            1 => changed.hearing_id = HearingId::new(),
            2 => {
                if let HearingChange::Replace {
                    ref mut expected_revision,
                    ..
                } = changed.change
                {
                    *expected_revision = HearingRevision::new(8).unwrap()
                }
            }
            3 => {
                if let HearingChange::Replace {
                    ref mut context, ..
                } = changed.change
                {
                    context.case_revision = CaseRevision::new(12).unwrap()
                }
            }
            4 => {
                if let HearingChange::Replace {
                    ref mut context, ..
                } = changed.change
                {
                    context.stage_revision = CaseStageRevision::new(14).unwrap()
                }
            }
            5 => {
                if let HearingChange::Replace { ref mut reason, .. } = changed.change {
                    *reason = HearingNote::new("Other reason").unwrap()
                }
            }
            _ => changed = fixed(HearingAction::Cancel),
        }
        assert_ne!(
            expected,
            hearing_submission_bytes(actor, case_id, &changed, digest)
        );
    }
}

#[test]
fn receipt_validation_detects_tampering_without_requiring_private_account_data() {
    let actor = UserId::new();
    let case_id = CaseId::new();
    let cmd = command();
    let original = detail(case_id, actor, &cmd, values(), &context(case_id, actor));
    hearing_receipt_matches(hasher().as_ref(), &original).unwrap();
    let mut renamed = original.clone();
    renamed.snapshot.recorded_by.email = "renamed@example.com".into();
    hearing_receipt_matches(hasher().as_ref(), &renamed).unwrap();
    for mode in 0..8 {
        let mut changed = original.clone();
        match mode {
            0 => changed.snapshot.recorded_by.id = UserId::new(),
            1 => changed.snapshot.case_id = CaseId::new(),
            2 => changed.snapshot.id = HearingId::new(),
            3 => changed.snapshot.receipt.operation_id = HearingOperationId::new(),
            4 => changed.snapshot.revision = HearingRevision::new(2).unwrap(),
            5 => changed.snapshot.status = HearingStatus::Cancelled,
            6 => changed.snapshot.reason = Some(HearingNote::new("Injected reason").unwrap()),
            _ => changed.snapshot.values_digest = Sha256Digest::from_array([0; 32]),
        }
        assert!(
            matches!(
                hearing_receipt_matches(hasher().as_ref(), &changed),
                Err(ApplicationError::Hearing(HearingError::StoredInconsistent(
                    _
                )))
            ),
            "mode{mode}"
        );
    }
}

#[test]
fn scheduling_receipt_cannot_claim_a_different_recorded_administrative_revision() {
    let actor = UserId::new();
    let case_id = CaseId::new();
    let cmd = command();
    let mut stored = detail(case_id, actor, &cmd, values(), &context(case_id, actor));
    stored.snapshot.recorded_administration_revision = CaseRevision::new(2).unwrap();
    assert!(hearing_receipt_matches(hasher().as_ref(), &stored).is_err());
}
