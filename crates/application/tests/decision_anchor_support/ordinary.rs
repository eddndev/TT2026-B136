use crate::{decision_anchor_support::*, decision_support::*};
use application::hearings::*;
use domain::cases::CaseId;
use domain::crypto::Sha256Digest;
use time::{Date, Duration, Month, UtcOffset};
use uuid::Uuid;

#[test]
fn an_exact_cancelled_initial_revision_remains_a_historical_anchor() {
    let detail = cancelled_initial();
    hearing_receipt_matches(&Hasher, &detail).unwrap();
    let mut fixture = Fixture::no_change();
    attach_initial(&mut fixture, detail.clone());
    let group = capture(fixture, &empty(), at());
    assert_eq!(
        group.decision.anchor,
        Some(MeasureDecisionAnchorMaterial::Initial(Box::new(detail)))
    );
    measure_decision_group_with_history_matches(&Hasher, &group, &empty()).unwrap();
}

#[test]
fn ordinary_anchor_requires_exact_id_revision_values_and_submission_digests() {
    for mutation in 0..4 {
        let mut fixture = Fixture::single();
        attach_initial(&mut fixture, ordinary_initial());
        let Some(MeasureDecisionAnchorRef::Initial {
            hearing_id,
            revision,
            values_digest,
            submission_digest,
        }) = &mut fixture.command.anchor
        else {
            panic!("ordinary anchor expected")
        };
        match mutation {
            0 => *hearing_id = HearingId::from_uuid(Uuid::from_u128(999)),
            1 => *revision = HearingRevision::new(2).unwrap(),
            2 => *values_digest = Sha256Digest::from_array([99; 32]),
            _ => *submission_digest = Sha256Digest::from_array([99; 32]),
        }
        assert!(prepare(fixture, &empty()).is_err());
    }
}

#[test]
fn ordinary_anchor_rejects_a_foreign_case_and_a_valid_noninitial_family() {
    let mut foreign = ordinary_initial();
    foreign.snapshot.case_id = CaseId::from_uuid(Uuid::from_u128(99));
    refresh_ordinary(&mut foreign);
    hearing_receipt_matches(&Hasher, &foreign).unwrap();
    let mut different = ordinary_initial();
    different.snapshot.values = HearingValues::new(HearingValuesInput {
        kind: HearingKind::Intermediate,
        scheduled_at: different.snapshot.values.scheduled_at(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Intermediate court").unwrap(),
        note: None,
        participants: vec![],
        conviction_basis: None,
    })
    .unwrap();
    different.snapshot.scheduling_context.stage = domain::case_stages::CaseStage::Intermediate;
    different.snapshot.scheduling_context.stage_revision =
        domain::case_administration::CaseStageRevision::new(2).unwrap();
    different.snapshot.scheduling_context.stage_digest = Some(Sha256Digest::from_array([8; 32]));
    different
        .snapshot
        .receipt
        .expected_context
        .as_mut()
        .unwrap()
        .stage_revision = different.snapshot.scheduling_context.stage_revision;
    refresh_ordinary(&mut different);
    hearing_receipt_matches(&Hasher, &different).unwrap();
    for detail in [foreign, different] {
        let mut fixture = Fixture::single();
        attach_initial(&mut fixture, detail);
        assert!(prepare(fixture, &empty()).is_err());
    }
}

#[test]
fn ordinary_anchor_revalidates_source_values_and_receipt_instead_of_trusting_selection() {
    for mutation in 0..3 {
        let mut detail = ordinary_initial();
        match mutation {
            0 => detail.snapshot.values_digest = Sha256Digest::from_array([99; 32]),
            1 => detail.snapshot.receipt.submission_digest = Sha256Digest::from_array([99; 32]),
            _ => detail.snapshot.receipt.expected_revision = 2,
        }
        let mut fixture = Fixture::single();
        attach_initial(&mut fixture, detail);
        assert!(prepare(fixture, &empty()).is_err());
    }
}

#[test]
fn ordinary_anchor_provenance_requires_clean_actor_and_supported_utc() {
    for actor in ["", " leading", "trailing ", "line\nbreak", "nul\0actor"] {
        let mut detail = ordinary_initial();
        detail.snapshot.recorded_by.email = actor.into();
        let mut fixture = Fixture::single();
        attach_initial(&mut fixture, detail);
        assert!(prepare(fixture, &empty()).is_err());
    }
    for clock in [
        at().to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
        Date::from_calendar_date(0, Month::January, 1)
            .unwrap()
            .midnight()
            .assume_utc(),
    ] {
        let mut detail = ordinary_initial();
        detail.snapshot.recorded_at = clock;
        let mut fixture = Fixture::single();
        attach_initial(&mut fixture, detail);
        assert!(prepare(fixture, &empty()).is_err());
    }
}

#[test]
fn decision_capture_cannot_predate_its_ordinary_anchor_capture() {
    let mut fixture = Fixture::single();
    let mut detail = ordinary_initial();
    detail.snapshot.recorded_at = at() + Duration::seconds(2);
    attach_initial(&mut fixture, detail);
    assert!(prepare(fixture.clone(), &empty())
        .unwrap()
        .into_group_capture(&Hasher, at())
        .is_err());
    capture(fixture, &empty(), at() + Duration::seconds(2));
}

#[test]
fn ordinary_recorded_context_cannot_be_later_or_conflict_at_the_same_revision() {
    for mutation in 0..2 {
        let mut detail = cancelled_initial();
        if mutation == 0 {
            detail.snapshot.recorded_administration_revision =
                application::cases::CaseRevision::new(2).unwrap();
        } else {
            let digest = Sha256Digest::from_array([99; 32]);
            detail.snapshot.recorded_administration_digest = digest;
            detail.snapshot.scheduling_context.administration_digest = digest;
        }
        hearing_receipt_matches(&Hasher, &detail).unwrap();
        let mut fixture = Fixture::single();
        attach_initial(&mut fixture, detail);
        assert!(prepare(fixture, &empty()).is_err());
    }
}

#[test]
fn full_ordinary_provenance_is_committed_beyond_the_legacy_submission_digest() {
    let detail = ordinary_initial();
    let mut first = Fixture::single();
    attach_initial(&mut first, detail.clone());
    let first = capture(first, &empty(), at() + Duration::seconds(2));
    for mutation in 0..2 {
        let mut changed = detail.clone();
        if mutation == 0 {
            changed.snapshot.recorded_by.email = "retained historical actor".into();
        } else {
            changed.snapshot.recorded_at += Duration::seconds(1);
        }
        assert_eq!(
            changed.snapshot.receipt.submission_digest,
            detail.snapshot.receipt.submission_digest
        );
        let mut fixture = Fixture::single();
        attach_initial(&mut fixture, changed);
        let group = capture(fixture, &empty(), at() + Duration::seconds(2));
        assert_ne!(group.decision.capture_digest, first.decision.capture_digest);
        assert_ne!(group.capture_digest, first.capture_digest);
    }
}
