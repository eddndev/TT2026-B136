mod precautionary_command_support;

use application::precautionary_hearings::{
    precautionary_hearing_submission_bytes, PrecautionaryHearingAction, PrecautionaryHearingChange,
};
use domain::case_administration::{CaseRevision, CaseStageRevision};
use domain::cases::CaseId;
use domain::crypto::Sha256Digest;
use domain::hearings::{HearingModality, HearingParticipantRef, HearingTime, HearingVenue};
use domain::identity::{Role, UserId};
use domain::participants::{ParticipantId, ParticipantRevision};
use domain::precautionary_hearings::{
    MeasureId, MeasureRevision, PrecautionaryHearingId, PrecautionaryHearingOperationId,
    PrecautionaryHearingPurpose, PrecautionaryHearingRevision, PrecautionaryHearingValues,
    PrecautionaryHearingValuesInput, PrecautionaryMeasureRef,
};
use precautionary_command_support::{basis, hex, note, values_input, Fixture};
use time::{macros::datetime, UtcOffset};
use uuid::Uuid;

#[test]
fn schedule_submission_matches_an_independently_framed_vector() {
    let expected = concat!(
        "504854584e31",                     // PHTXN1
        "00000000000000000000000000000001", // Actor
        "00000003614062",                   // a@b
        "00",                               // Owner
        "00000000000000000000000000000002", // Case
        "00000000000000000000000000000003", // Operation
        "00000000000000000000000000000004", // Hearing
        "00",                               // Schedule
        "00000000",                         // No prior revision
        "00",                               // No prior capture
        "01",                               // Context present
        "00000005",                         // Administration revision
        "00000006",                         // Stage revision
        "3333333333333333333333333333333333333333333333333333333333333333",
        "00000070",                         // 112-byte PHEAR1 values
        "504845415231",                     // PHEAR1
        "00",                               // Imposition
        "0000000000000000",                 // Unix epoch
        "00000000",                         // UTC offset
        "00",                               // InPerson
        "00000007436f7572742041",           // Court A
        "00",                               // No note
        "00",                               // No participants
        "0000000c536574206279206f72646572", // Set by order
        "0000000000000000000000000000004d", // Document 77
        "00000004",                         // Exact version 4
        "4242424242424242424242424242424242424242424242424242424242424242",
        "00000006506167652032", // Page 2
        "00",                   // No review targets
        "00"                    // No command reason
    );
    let bytes = Fixture::schedule().bytes();
    assert_eq!(bytes.len(), 242);
    assert_eq!(hex(&bytes), expected);
}

#[test]
fn commands_have_distinct_action_tags_names_and_checked_result_revisions() {
    for (fixture, action, tag, name, expected, result) in [
        (
            Fixture::schedule(),
            PrecautionaryHearingAction::Schedule,
            0,
            "schedule",
            0,
            1,
        ),
        (
            Fixture::replace(),
            PrecautionaryHearingAction::Replace,
            1,
            "replace",
            7,
            8,
        ),
        (
            Fixture::cancel(),
            PrecautionaryHearingAction::Cancel,
            2,
            "cancel",
            7,
            8,
        ),
    ] {
        assert_eq!(fixture.command.action(), action);
        assert_eq!(action.tag(), tag);
        assert_eq!(action.as_str(), name);
        assert_eq!(fixture.command.expected_revision(), expected);
        assert_eq!(fixture.command.result_revision().unwrap().get(), result);
        assert_eq!(fixture.bytes()[78], tag);
    }
}

#[test]
fn replacement_and_cancellation_require_positive_nonexhausted_revisions() {
    assert!(PrecautionaryHearingRevision::new(0).is_err());
    for mut fixture in [Fixture::replace(), Fixture::cancel()] {
        for revision in [1, u32::MAX - 1, u32::MAX] {
            match &mut fixture.command.change {
                PrecautionaryHearingChange::Replace {
                    expected_revision, ..
                }
                | PrecautionaryHearingChange::Cancel {
                    expected_revision, ..
                } => {
                    *expected_revision = PrecautionaryHearingRevision::new(revision).unwrap();
                }
                PrecautionaryHearingChange::Schedule { .. } => unreachable!(),
            }
            let result = fixture.command.result_revision();
            if revision == u32::MAX {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap().get(), revision + 1);
            }
        }
    }
}

#[test]
fn submission_binds_actor_case_operation_and_hearing_identity() {
    let source = Fixture::schedule();
    let original = source.bytes();
    let mutations: &[fn(&mut Fixture)] = &[
        |f| f.actor.id = UserId::from_uuid(Uuid::from_u128(101)),
        |f| f.actor.email = "other@b".to_owned(),
        |f| f.actor.role = Role::Litigator,
        |f| f.case_id = CaseId::from_uuid(Uuid::from_u128(102)),
        |f| {
            f.command.operation_id =
                PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(103))
        },
        |f| f.command.hearing_id = PrecautionaryHearingId::from_uuid(Uuid::from_u128(104)),
    ];
    for (index, mutate) in mutations.iter().enumerate() {
        let mut changed = source.clone();
        mutate(&mut changed);
        assert_ne!(changed.bytes(), original, "identity mutation {index}");
    }
}

#[test]
fn scheduling_and_replacement_bind_every_expected_context_field() {
    for source in [Fixture::schedule(), Fixture::replace()] {
        let original = source.bytes();
        for index in 0..3 {
            let mut changed = source.clone();
            match &mut changed.command.change {
                PrecautionaryHearingChange::Schedule { context, .. }
                | PrecautionaryHearingChange::Replace { context, .. } => match index {
                    0 => context.administration_revision = CaseRevision::new(8).unwrap(),
                    1 => context.stage_revision = CaseStageRevision::new(9).unwrap(),
                    _ => context.context_digest = Sha256Digest::from_array([0x34; 32]),
                },
                PrecautionaryHearingChange::Cancel { .. } => unreachable!(),
            }
            assert_ne!(changed.bytes(), original, "context mutation {index}");
        }
    }
}

#[test]
fn replacement_and_cancellation_bind_revision_capture_and_reason() {
    for source in [Fixture::replace(), Fixture::cancel()] {
        let original = source.bytes();
        for index in 0..3 {
            let mut changed = source.clone();
            match &mut changed.command.change {
                PrecautionaryHearingChange::Replace {
                    expected_revision,
                    expected_capture_digest,
                    reason,
                    ..
                }
                | PrecautionaryHearingChange::Cancel {
                    expected_revision,
                    expected_capture_digest,
                    reason,
                } => match index {
                    0 => *expected_revision = PrecautionaryHearingRevision::new(8).unwrap(),
                    1 => *expected_capture_digest = Sha256Digest::from_array([0x45; 32]),
                    _ => *reason = note("Correct the recorded appointment"),
                },
                PrecautionaryHearingChange::Schedule { .. } => unreachable!(),
            }
            assert_ne!(changed.bytes(), original, "base or reason mutation {index}");
        }
        assert_eq!(&original[79..83], &7_u32.to_be_bytes());
        assert_eq!(original[83], 1);
        assert_eq!(&original[84..116], &[0x44; 32]);
    }
}

#[test]
fn resolved_values_must_equal_the_schedule_or_replacement_values() {
    for mut fixture in [Fixture::schedule(), Fixture::replace()] {
        let mut input = values_input();
        input.venue = HearingVenue::new("Court B").unwrap();
        fixture.resolved_values = PrecautionaryHearingValues::new(input).unwrap();
        assert!(precautionary_hearing_submission_bytes(
            &fixture.actor,
            fixture.case_id,
            &fixture.command,
            &fixture.resolved_values,
        )
        .is_err());
    }
}

#[test]
fn each_declared_value_and_exact_support_are_bound_to_the_submission() {
    let mutations: &[fn(&mut PrecautionaryHearingValuesInput)] = &[
        |v| v.scheduled_at = HearingTime::new(datetime!(1970-01-01 00:00:01 UTC)).unwrap(),
        |v| {
            v.scheduled_at = HearingTime::new(
                v.scheduled_at
                    .value()
                    .to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap()),
            )
            .unwrap();
        },
        |v| v.modality = HearingModality::Videoconference,
        |v| v.venue = HearingVenue::new("Court B").unwrap(),
        |v| v.note = Some(note("Operator note")),
        |v| {
            v.participants.push(HearingParticipantRef::new(
                ParticipantId::from_uuid(Uuid::from_u128(9)),
                ParticipantRevision::new(2).unwrap(),
            ));
        },
        |v| v.scheduling_basis = basis("Another order", 77, 4, 0x42, "Page 2"),
        |v| v.scheduling_basis = basis("Set by order", 78, 4, 0x42, "Page 2"),
        |v| v.scheduling_basis = basis("Set by order", 77, 5, 0x42, "Page 2"),
        |v| v.scheduling_basis = basis("Set by order", 77, 4, 0x43, "Page 2"),
        |v| v.scheduling_basis = basis("Set by order", 77, 4, 0x42, "Page 3"),
        |v| {
            v.purpose = PrecautionaryHearingPurpose::Review;
            v.review_targets.push(PrecautionaryMeasureRef::new(
                MeasureId::from_uuid(Uuid::from_u128(11)),
                MeasureRevision::new(3).unwrap(),
                Sha256Digest::from_array([0x55; 32]),
            ));
        },
    ];
    for source in [Fixture::schedule(), Fixture::replace(), Fixture::cancel()] {
        let original = source.bytes();
        for (index, mutate) in mutations.iter().enumerate() {
            let mut changed = source.clone();
            let mut input = values_input();
            mutate(&mut input);
            changed.set_values(PrecautionaryHearingValues::new(input).unwrap());
            assert_ne!(changed.bytes(), original, "values mutation {index}");
        }
    }
}

#[test]
fn cancellation_frames_exact_prior_values_without_a_replacement_context() {
    let fixture = Fixture::cancel();
    let bytes = fixture.bytes();
    let prior = fixture.resolved_values.canonical_bytes();
    assert_eq!(bytes[116], 0);
    assert_eq!(&bytes[117..121], &(prior.len() as u32).to_be_bytes());
    assert_eq!(&bytes[121..121 + prior.len()], prior.as_slice());
    let reason = b"Remove duplicate appointment";
    let suffix = &bytes[121 + prior.len()..];
    assert_eq!(suffix[0], 1);
    assert_eq!(&suffix[1..5], &(reason.len() as u32).to_be_bytes());
    assert_eq!(&suffix[5..], reason);
}

#[test]
fn historical_identity_framing_accepts_every_role_without_authorizing_a_write() {
    let source = Fixture::schedule();
    let original = source.bytes();
    for (role, tag) in [
        (Role::Owner, 0),
        (Role::Litigator, 1),
        (Role::Paralegal, 2),
        (Role::Client, 3),
    ] {
        let mut changed = source.clone();
        changed.actor.role = role;
        let bytes = changed.bytes();
        assert_eq!(bytes[29], tag);
        assert_eq!(&bytes[..29], &original[..29]);
        assert_eq!(&bytes[30..], &original[30..]);
    }
}

#[test]
fn malformed_actor_email_is_rejected_without_normalizing_captured_identity() {
    for email in [
        "",
        " ",
        " a@b",
        "a@b ",
        "a\n@b",
        "a\r@b",
        "a\t@b",
        "a\0@b",
        "a\u{7f}@b",
        "a\u{85}@b",
    ] {
        let mut fixture = Fixture::schedule();
        fixture.actor.email = email.to_owned();
        assert!(
            precautionary_hearing_submission_bytes(
                &fixture.actor,
                fixture.case_id,
                &fixture.command,
                &fixture.resolved_values,
            )
            .is_err(),
            "email {email:?}"
        );
    }
}

#[test]
fn identity_framing_preserves_non_rfc_labels_and_utf8_byte_lengths() {
    for email in ["historical-actor", "actor\u{e9}@example.test"] {
        let mut fixture = Fixture::schedule();
        fixture.actor.email = email.to_owned();
        let bytes = fixture.bytes();
        assert_eq!(&bytes[22..26], &(email.len() as u32).to_be_bytes());
        assert_eq!(&bytes[26..26 + email.len()], email.as_bytes());
    }
}
