use application::case_stages::*;
use application::precautionary_hearings::PrecautionaryContext;
use time::{Date, Duration, Month, UtcOffset};

use crate::precautionary_context_support::*;

#[test]
fn every_source_provenance_time_requires_utc_and_a_supported_year() {
    let invalid = [
        at().to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
        Date::from_calendar_date(0, Month::January, 1)
            .unwrap()
            .midnight()
            .assume_utc(),
    ];
    for timestamp in invalid {
        for location in 0..3 {
            let mut material = changed(trial());
            observed_newer(&mut material);
            match location {
                0 => material.administration.changed_at = timestamp,
                1 => material.stage_administration.changed_at = timestamp,
                _ => changed_mut(&mut material).recorded_at = timestamp,
            }
            rejected(material);
        }
    }
}

#[test]
fn initial_capture_rejects_non_utc_even_when_it_represents_the_same_instant() {
    let mut material = initial();
    let CaseStageEntry::Initial(stage) = &mut material.stage else {
        unreachable!()
    };
    stage.recorded_at = at().to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap());
    rejected(material);
}

#[test]
fn utc_year_boundaries_are_valid_for_consistent_initial_provenance() {
    for year in [1, 9999] {
        let mut material = initial();
        let timestamp = Date::from_calendar_date(year, Month::January, 1)
            .unwrap()
            .midnight()
            .assume_utc();
        material.administration.changed_at = timestamp;
        material.stage_administration.changed_at = timestamp;
        let CaseStageEntry::Initial(stage) = &mut material.stage else {
            unreachable!()
        };
        stage.recorded_at = timestamp;
        assert!(PrecautionaryContext::new(&Hasher, material).is_ok());
    }
}

#[test]
fn observed_administration_and_stage_capture_cannot_predate_the_parent_administration() {
    for observed in [false, true] {
        let mut material = changed(trial());
        observed_newer(&mut material);
        if observed {
            material.administration.changed_at = at() - Duration::nanoseconds(1);
        } else {
            changed_mut(&mut material).recorded_at = at() - Duration::nanoseconds(1);
        }
        rejected(material);
    }
}

#[test]
fn every_actor_capture_rejects_empty_untrimmed_and_control_character_strings() {
    for email in [
        "",
        "   ",
        " actor@example.com",
        "actor@example.com ",
        "actor\n@example.com",
        "actor\t@example.com",
        "actor\u{7f}@example.com",
    ] {
        for location in 0..3 {
            let mut material = changed(trial());
            observed_newer(&mut material);
            match location {
                0 => material.administration.changed_by.email = email.into(),
                1 => material.stage_administration.changed_by.email = email.into(),
                _ => changed_mut(&mut material).recorded_by.email = email.into(),
            }
            rejected(material);
        }
    }
}

#[test]
fn actor_capture_validation_does_not_invent_an_email_address_policy() {
    let mut material = changed(trial());
    observed_newer(&mut material);
    material.administration.changed_by.email = "historical account identifier".into();
    material.stage_administration.changed_by.email = "retained identity".into();
    changed_mut(&mut material).recorded_by.email = "original actor".into();
    assert!(PrecautionaryContext::new(&Hasher, material).is_ok());
}
