use application::hearing_results::*;
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    hearings::{HearingId, HearingRevision},
    identity::UserId,
};
use uuid::Uuid;

fn values() -> HearingResultValues {
    HearingResultValues::new(HearingResultValuesInput {
        occurrence: HearingResultOccurrence::Occurred,
        extent: HearingResultExtent::Unspecified,
        event_time: DeclaredHearingResultTime::instant(
            OffsetDateTime::from_unix_timestamp(1000).unwrap(),
        )
        .unwrap(),
        summary: HearingResultText::new("a").unwrap(),
        attendees: vec![],
        agreements: vec![],
        provenance: HearingResultProvenance::new(
            HearingResultProvenanceKind::OperatorNote,
            None,
            None,
        )
        .unwrap(),
    })
    .unwrap()
}
fn command(action: HearingResultAction) -> HearingResultCommand {
    HearingResultCommand {
        operation_id: HearingResultOperationId::from_uuid(Uuid::from_u128(1)),
        hearing_id: HearingId::from_uuid(Uuid::from_u128(4)),
        result_id: HearingResultId::from_uuid(Uuid::from_u128(5)),
        change: match action {
            HearingResultAction::Record => HearingResultChange::Record {
                anchor_revision: HearingRevision::new(7).unwrap(),
                continuation: None,
                values: values(),
            },
            HearingResultAction::Correct => HearingResultChange::Correct {
                expected_revision: HearingResultRevision::new(2).unwrap(),
                values: values(),
                reason: HearingResultText::new(" Motivo\r\n\u{e1} ").unwrap(),
            },
            HearingResultAction::Withdraw => HearingResultChange::Withdraw {
                expected_revision: HearingResultRevision::new(2).unwrap(),
                reason: HearingResultText::new("Retiro").unwrap(),
            },
        },
    }
}
fn anchor() -> HearingResultAnchor {
    HearingResultAnchor {
        hearing_id: HearingId::from_uuid(Uuid::from_u128(4)),
        revision: HearingRevision::new(7).unwrap(),
        values_digest: Sha256Digest::from_array([17; 32]),
        submission_digest: Sha256Digest::from_array([34; 32]),
    }
}
fn continuation() -> HearingResultContinuation {
    HearingResultContinuation {
        hearing_id: HearingId::from_uuid(Uuid::from_u128(6)),
        result_id: HearingResultId::from_uuid(Uuid::from_u128(7)),
        revision: HearingResultRevision::new(9).unwrap(),
        values_digest: Sha256Digest::from_array([51; 32]),
        submission_digest: Sha256Digest::from_array([68; 32]),
    }
}
fn bytes(
    command: &HearingResultCommand,
    anchor: &HearingResultAnchor,
    continuation: Option<&HearingResultContinuation>,
) -> Vec<u8> {
    hearing_result_submission_bytes(
        UserId::from_uuid(Uuid::from_u128(2)),
        CaseId::from_uuid(Uuid::from_u128(3)),
        command,
        anchor,
        continuation,
        Sha256Digest::from_array(std::array::from_fn(|i| i as u8)),
    )
}
#[test]
fn three_actions_match_independent_vectors_and_exact_revision_successors() {
    let vectors: Vec<_> = include_str!("hearing_result_support/wire-vectors.txt")
        .lines()
        .collect();
    for (index, action) in [
        HearingResultAction::Record,
        HearingResultAction::Correct,
        HearingResultAction::Withdraw,
    ]
    .into_iter()
    .enumerate()
    {
        let command = command(action);
        let continuation = (index > 0).then(continuation);
        let encoded = bytes(&command, &anchor(), continuation.as_ref());
        assert_eq!(
            encoded
                .iter()
                .map(|v| format!("{v:02x}"))
                .collect::<String>(),
            vectors[index]
        );
        assert_eq!(command.action().tag(), index as u8);
        assert_eq!(
            command.action().as_str(),
            ["record", "correct", "withdraw"][index]
        );
        assert_eq!(
            command.result_revision().unwrap().get(),
            if index == 0 { 1 } else { 3 }
        );
    }
}
#[test]
fn minimum_and_maximum_receipts_are_bounded_in_bytes() {
    assert_eq!(
        bytes(&command(HearingResultAction::Record), &anchor(), None).len(),
        192
    );
    let mut command = command(HearingResultAction::Withdraw);
    command.change = HearingResultChange::Withdraw {
        expected_revision: HearingResultRevision::new(2).unwrap(),
        reason: HearingResultText::new(&"\u{10000}".repeat(1000)).unwrap(),
    };
    assert_eq!(
        bytes(&command, &anchor(), Some(&continuation())).len(),
        4296
    );
    command.change = HearingResultChange::Withdraw {
        expected_revision: HearingResultRevision::new(u32::MAX).unwrap(),
        reason: HearingResultText::new("exhausted").unwrap(),
    };
    assert!(matches!(
        command.result_revision(),
        Err(application::ApplicationError::HearingResult(
            HearingResultError::RevisionExhausted
        ))
    ));
}
#[test]
fn receipt_binds_fixed_sources_and_scope_independently_of_value_similarity() {
    let command = command(HearingResultAction::Correct);
    let reference = anchor();
    let previous = continuation();
    let original = bytes(&command, &reference, Some(&previous));
    for mode in 0..12 {
        let mut command = command.clone();
        let mut reference = reference;
        let mut previous = previous;
        match mode {
            0 => command.operation_id = HearingResultOperationId::new(),
            1 => command.hearing_id = HearingId::new(),
            2 => command.result_id = HearingResultId::new(),
            3 => reference.revision = HearingRevision::new(8).unwrap(),
            4 => reference.values_digest = Sha256Digest::from_array([18; 32]),
            5 => reference.submission_digest = Sha256Digest::from_array([18; 32]),
            6 => previous.hearing_id = HearingId::new(),
            7 => previous.result_id = HearingResultId::new(),
            8 => previous.revision = HearingResultRevision::new(10).unwrap(),
            9 => previous.values_digest = Sha256Digest::from_array([18; 32]),
            10 => previous.submission_digest = Sha256Digest::from_array([18; 32]),
            _ => {
                command.change = HearingResultChange::Withdraw {
                    expected_revision: HearingResultRevision::new(2).unwrap(),
                    reason: HearingResultText::new("Retiro").unwrap(),
                }
            }
        }
        assert_ne!(
            original,
            bytes(&command, &reference, Some(&previous)),
            "mode {mode}"
        );
    }
    assert_ne!(original, bytes(&command, &reference, None));
    for (actor, case_id, digest) in [(8, 3, 0), (2, 8, 0), (2, 3, 1)] {
        let encoded = hearing_result_submission_bytes(
            UserId::from_uuid(Uuid::from_u128(actor)),
            CaseId::from_uuid(Uuid::from_u128(case_id)),
            &command,
            &reference,
            Some(&previous),
            Sha256Digest::from_array(std::array::from_fn(|i| i as u8 + digest)),
        );
        assert_ne!(original, encoded);
    }
}
