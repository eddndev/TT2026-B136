mod precautionary_declaration_support;
#[path = "precautionary_declaration_support/vectors.rs"]
mod vectors;

use domain::crypto::Sha256Digest;
use domain::precautionary_hearings::{MeasureId, PrecautionaryHearingId};
use domain::precautionary_measures::{
    MeasureDecisionId, MeasureDecisionValues, MeasureKind, MeasureSupervision, MeasureTime,
    MeasureValidity, MeasureValues,
};
use domain::procedural_time::{DeclaredProceduralPrecision, DeclaredProceduralTime};
use domain::typed_participants::{CaseSubjectId, SubjectRevision};
use precautionary_declaration_support::*;
use std::any::TypeId;
use time::{Month, UtcOffset};
use uuid::Uuid;

#[test]
fn measure_values_preserve_exact_subject_terms_and_explicit_supervision() {
    let input = measure_input();
    let values = MeasureValues::new(input.clone());
    assert_eq!(values.subject(), input.subject);
    assert_eq!(values.kind(), input.kind);
    assert_eq!(values.conditions(), &input.conditions);
    assert_eq!(values.validity(), &input.validity);
    assert_eq!(values.supervision(), &input.supervision);
    assert_eq!(values, values.clone());
}

#[test]
fn all_fourteen_measure_classes_are_bound_without_reinterpreting_the_declarations() {
    for kind in [
        MeasureKind::PeriodicAppearance,
        MeasureKind::FinancialGuarantee,
        MeasureKind::AssetSeizure,
        MeasureKind::AccountFreeze,
        MeasureKind::TravelRestriction,
        MeasureKind::CustodyOrInstitution,
        MeasureKind::PlaceRestriction,
        MeasureKind::ContactRestriction,
        MeasureKind::HomeSeparation,
        MeasureKind::PublicOfficeSuspension,
        MeasureKind::ProfessionalSuspension,
        MeasureKind::ElectronicMonitoring,
        MeasureKind::HomeConfinement,
        MeasureKind::PretrialDetention,
    ] {
        let mut input = measure_input();
        input.kind = kind;
        let values = MeasureValues::new(input.clone());
        assert_eq!(values.kind(), kind);
        assert_eq!(values.canonical_bytes()[57], kind.tag());
        assert_eq!(values.subject(), input.subject);
        assert_eq!(values.conditions(), &input.conditions);
        assert_eq!(values.validity(), &input.validity);
        assert_eq!(values.supervision(), &input.supervision);
    }
}

#[test]
fn each_exact_subject_field_and_declared_condition_changes_the_commitment() {
    let expected = MeasureValues::new(measure_input()).canonical_bytes();
    for field in 0..4 {
        let mut input = measure_input();
        match field {
            0 => input.subject.id = CaseSubjectId::from_uuid(Uuid::from_u128(8)),
            1 => input.subject.revision = SubjectRevision::new(8).unwrap(),
            2 => input.subject.values_digest = Sha256Digest::from_array([0x12; 32]),
            _ => input.conditions = note("Different declared conditions"),
        }
        assert_ne!(MeasureValues::new(input).canonical_bytes(), expected);
    }
}

#[test]
fn supervision_commits_its_exact_participant_revision_statement_or_unknown_reason() {
    let expected = MeasureValues::new(measure_input()).canonical_bytes();
    for supervision in [
        MeasureSupervision::Known {
            participant: participant(8, 4),
            statement: note("S"),
        },
        MeasureSupervision::Known {
            participant: participant(3, 8),
            statement: note("S"),
        },
        MeasureSupervision::Known {
            participant: participant(3, 4),
            statement: note("Other declared supervision"),
        },
        MeasureSupervision::Unknown { reason: note("S") },
        MeasureSupervision::Unknown { reason: note("U") },
    ] {
        let mut input = measure_input();
        input.supervision = supervision.clone();
        let values = MeasureValues::new(input);
        assert_eq!(values.supervision(), &supervision);
        assert_ne!(values.canonical_bytes(), expected);
    }
    let mut left = measure_input();
    left.supervision = MeasureSupervision::Unknown { reason: note("S") };
    let mut right = left.clone();
    right.supervision = MeasureSupervision::Unknown { reason: note("U") };
    assert_ne!(
        MeasureValues::new(left).canonical_bytes(),
        MeasureValues::new(right).canonical_bytes()
    );
}

#[test]
fn absent_unknown_and_declared_end_remain_distinct_in_the_full_measure() {
    let mut encodings = Vec::new();
    for end in [
        None,
        Some(unknown_time("No end appears in the source")),
        Some(known_time(
            DeclaredProceduralTime::date(day(), None).unwrap(),
        )),
    ] {
        let mut input = measure_input();
        input.validity = MeasureValidity::new(
            input.validity.start().clone(),
            input.validity.statement().clone(),
            end.clone(),
        )
        .unwrap();
        let values = MeasureValues::new(input);
        assert_eq!(values.validity().end(), end.as_ref());
        let bytes = values.canonical_bytes();
        assert!(!encodings.contains(&bytes));
        encodings.push(bytes);
    }
}

#[test]
fn measure_commitment_binds_complete_validity_without_replacing_its_existing_canon() {
    let source = measure_input();
    let expected = MeasureValues::new(source.clone()).canonical_bytes();
    for validity in [
        MeasureValidity::new(unknown_time("Start is not stated"), note("V"), None).unwrap(),
        MeasureValidity::new(source.validity.start().clone(), note("Other terms"), None).unwrap(),
    ] {
        let mut input = source.clone();
        input.validity = validity;
        assert_ne!(MeasureValues::new(input).canonical_bytes(), expected);
    }
    assert_eq!(
        hex(&source.validity.canonical_bytes()),
        "4d56414c310107ea0a040000000000015600"
    );
}

#[test]
fn decision_values_preserve_factual_source_without_inventing_actions_or_capture_time() {
    let input = decision_input();
    let values = MeasureDecisionValues::new(input.clone());
    assert_eq!(values.authority(), &input.authority);
    assert_eq!(values.declared_at(), &input.declared_at);
    assert_eq!(values.justification(), &input.justification);
    assert_eq!(values.support(), input.support);
    assert_eq!(values.locator(), &input.locator);
    assert_eq!(values, values.clone());
}

#[test]
fn decision_commitment_binds_authority_justification_and_every_exact_support_field() {
    let expected = MeasureDecisionValues::new(decision_input()).canonical_bytes();
    for field in 0..6 {
        let mut input = decision_input();
        match field {
            0 => input.authority = note("Another declared authority"),
            1 => input.justification = note("Another stated justification"),
            2 => input.support = support(8, 6, 0x22),
            3 => input.support = support(5, 8, 0x22),
            4 => input.support = support(5, 6, 0x23),
            _ => input.locator = note("Another source locator"),
        }
        assert_ne!(
            MeasureDecisionValues::new(input).canonical_bytes(),
            expected
        );
    }
}

#[test]
fn decision_times_keep_unknown_date_minute_second_and_absent_offsets_distinct() {
    let times = [
        unknown_time("No time is supplied"),
        known_time(DeclaredProceduralTime::date(day(), None).unwrap()),
        known_time(DeclaredProceduralTime::date(day(), Some(UtcOffset::UTC)).unwrap()),
        known_time(DeclaredProceduralTime::minute(day(), 1, 2, None).unwrap()),
        known_time(DeclaredProceduralTime::minute(day(), 1, 2, Some(UtcOffset::UTC)).unwrap()),
        known_time(DeclaredProceduralTime::second(day(), 1, 2, 0, None).unwrap()),
        known_time(DeclaredProceduralTime::second(day(), 1, 2, 0, Some(UtcOffset::UTC)).unwrap()),
    ];
    let mut encodings = Vec::new();
    for declared_at in times {
        let mut input = decision_input();
        input.declared_at = declared_at.clone();
        let values = MeasureDecisionValues::new(input);
        assert_eq!(values.declared_at(), &declared_at);
        let bytes = values.canonical_bytes();
        assert!(!encodings.contains(&bytes));
        encodings.push(bytes);
    }
}

#[test]
fn decision_time_commitment_binds_all_components_and_preserves_original_offset() {
    let expected = MeasureDecisionValues::new(decision_input()).canonical_bytes();
    let offset = Some(UtcOffset::from_hms(-6, 0, 0).unwrap());
    for declaration in [
        DeclaredProceduralTime::second(date(2027, Month::October, 4), 1, 2, 3, offset),
        DeclaredProceduralTime::second(date(2026, Month::November, 4), 1, 2, 3, offset),
        DeclaredProceduralTime::second(date(2026, Month::October, 5), 1, 2, 3, offset),
        DeclaredProceduralTime::second(day(), 2, 2, 3, offset),
        DeclaredProceduralTime::second(day(), 1, 3, 3, offset),
        DeclaredProceduralTime::second(day(), 1, 2, 4, offset),
        DeclaredProceduralTime::second(day(), 7, 2, 3, Some(UtcOffset::UTC)),
        DeclaredProceduralTime::second(day(), 1, 2, 3, None),
    ] {
        let mut input = decision_input();
        input.declared_at = known_time(declaration.unwrap());
        assert_ne!(
            MeasureDecisionValues::new(input).canonical_bytes(),
            expected
        );
    }
    let mut left = decision_input();
    left.declared_at = unknown_time("Source unreadable");
    let mut right = left.clone();
    right.declared_at = unknown_time("Source lacks the time");
    assert_ne!(
        MeasureDecisionValues::new(left).canonical_bytes(),
        MeasureDecisionValues::new(right).canonical_bytes()
    );
}

#[test]
fn unknown_decision_time_requires_reason_while_known_time_prohibits_one() {
    assert!(MeasureTime::new(DeclaredProceduralTime::unknown(), None).is_err());
    assert!(MeasureTime::new(
        DeclaredProceduralTime::date(day(), None).unwrap(),
        Some(note("A reason cannot change known precision")),
    )
    .is_err());
    let mut input = decision_input();
    input.declared_at = unknown_time("Decision time is not stated");
    let values = MeasureDecisionValues::new(input);
    assert_eq!(
        values.declared_at().declared().precision(),
        DeclaredProceduralPrecision::Unknown
    );
    assert_eq!(values.declared_at().declared().local_date(), None);
    assert_eq!(values.declared_at().declared().offset(), None);
    assert_eq!(
        values.declared_at().unknown_reason().unwrap().as_str(),
        "Decision time is not stated"
    );
}

#[test]
fn decision_identity_is_stable_and_distinct_from_measure_and_appointment_identity() {
    let raw = Uuid::from_u128(17);
    let id = MeasureDecisionId::from_uuid(raw);
    assert_eq!(id.as_uuid(), raw);
    assert_eq!(id.to_string(), "00000000-0000-0000-0000-000000000011");
    assert_eq!(id, MeasureDecisionId::from_uuid(raw));
    assert_ne!(TypeId::of::<MeasureDecisionId>(), TypeId::of::<MeasureId>());
    assert_ne!(
        TypeId::of::<MeasureDecisionId>(),
        TypeId::of::<PrecautionaryHearingId>()
    );
    assert_eq!(MeasureDecisionId::new().as_uuid().get_version_num(), 4);
    assert_eq!(MeasureDecisionId::default().as_uuid().get_version_num(), 4);
}
