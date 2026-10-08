use application::cases::CaseRevision;
use application::precautionary_hearings::PrecautionaryContext;
use domain::identity::{Role, UserId};
use domain::procedural_time::DeclaredProceduralTime;
use time::{Date, Duration, Month, UtcOffset};
use uuid::Uuid;

use crate::correction_support::*;

#[test]
fn owner_and_litigator_record_their_own_historical_identity() {
    for role in [Role::Owner, Role::Litigator] {
        let mut fixture = CorrectionFixture::initial();
        fixture.actor.id = UserId::from_uuid(Uuid::from_u128(900));
        fixture.actor.email = "correcting@example.test".into();
        fixture.actor.role = role;
        let prior_actor = fixture.previous().actor;
        let capture = fixture.capture();
        assert_eq!(capture.review.actor, fixture.actor);
        assert_eq!(capture.records[0].actor, fixture.actor);
        assert_ne!(capture.review.actor, prior_actor);
        measure_administrative_capture_matches(&Hasher, &capture, &fixture.history).unwrap();
    }
}

#[test]
fn paralegal_and_client_cannot_prepare_a_recording_capture() {
    for role in [Role::Paralegal, Role::Client] {
        let mut fixture = CorrectionFixture::initial();
        fixture.actor.role = role;
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn historical_actor_text_rejects_empty_trim_or_control_without_inventing_rfc_validation() {
    for email in [
        "",
        " leading",
        "trailing ",
        "inside\ncontrol",
        "inside\tcontrol",
    ] {
        let mut fixture = CorrectionFixture::initial();
        fixture.actor.email = email.into();
        assert!(fixture.prepare().is_err(), "{email:?}");
    }
    let mut fixture = CorrectionFixture::initial();
    fixture.actor.email = "historical-identity".into();
    assert_eq!(fixture.capture().review.actor.email, "historical-identity");
}

#[test]
fn unchanged_terms_or_a_new_end_are_rejected_during_preparation() {
    for add_end in [false, true] {
        let mut fixture = CorrectionFixture::initial();
        let prior = fixture.previous();
        let MeasureSupervision::Known { statement, .. } = prior.result.values.supervision() else {
            panic!("known fixture expected")
        };
        let validity = if add_end {
            MeasureValidity::new(
                prior.result.values.validity().start().clone(),
                prior.result.values.validity().statement().clone(),
                Some(
                    MeasureTime::new(
                        DeclaredProceduralTime::unknown(),
                        Some(note("End not legible")),
                    )
                    .unwrap(),
                ),
            )
            .unwrap()
        } else {
            prior.result.values.validity().clone()
        };
        fixture.command.action =
            MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
                prior.result.values.conditions().clone(),
                validity,
                statement.clone(),
            ));
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn capture_clock_can_equal_the_selected_judicial_clock_but_cannot_precede_it() {
    let fixture = CorrectionFixture::initial();
    let boundary = fixture.previous_group().recorded_at;
    for at in [boundary, boundary + Duration::nanoseconds(1)] {
        let capture = fixture
            .prepare()
            .unwrap()
            .into_capture(&Hasher, at)
            .unwrap();
        assert_eq!(capture.recorded_at, at);
        measure_administrative_capture_matches(&Hasher, &capture, &fixture.history).unwrap();
    }
    assert!(fixture
        .prepare()
        .unwrap()
        .into_capture(&Hasher, boundary - Duration::nanoseconds(1))
        .is_err());
}

#[test]
fn newer_context_provenance_raises_the_capture_clock_floor() {
    let mut fixture = CorrectionFixture::initial();
    let mut material = fixture.context.material().clone();
    material.administration.revision = CaseRevision::new(2).unwrap();
    material.administration.changed_at = fixture.recorded_at + Duration::seconds(10);
    let boundary = material.administration.changed_at;
    fixture.context = PrecautionaryContext::new(&Hasher, material).unwrap();
    fixture.command.context = expectation(&fixture.context);
    assert!(fixture
        .prepare()
        .unwrap()
        .into_capture(&Hasher, fixture.recorded_at)
        .is_err());
    let capture = fixture
        .prepare()
        .unwrap()
        .into_capture(&Hasher, boundary)
        .unwrap();
    assert_eq!(capture.recorded_at, boundary);
    measure_administrative_capture_matches(&Hasher, &capture, &fixture.history).unwrap();
}

#[test]
fn capture_clock_requires_supported_utc_and_preserves_nanoseconds() {
    let fixture = CorrectionFixture::initial();
    for invalid in [
        fixture
            .recorded_at
            .to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
        Date::from_calendar_date(0, Month::January, 1)
            .unwrap()
            .midnight()
            .assume_utc(),
    ] {
        assert!(fixture
            .prepare()
            .unwrap()
            .into_capture(&Hasher, invalid)
            .is_err());
    }
    let capture = fixture.capture();
    assert_eq!(
        capture.recorded_at.nanosecond(),
        fixture.recorded_at.nanosecond()
    );
    assert_eq!(capture.records[0].recorded_at, fixture.recorded_at);
    let latest = Date::from_calendar_date(9999, Month::December, 31)
        .unwrap()
        .midnight()
        .assume_utc();
    let capture = fixture
        .prepare()
        .unwrap()
        .into_capture(&Hasher, latest)
        .unwrap();
    measure_administrative_capture_matches(&Hasher, &capture, &fixture.history).unwrap();
}
