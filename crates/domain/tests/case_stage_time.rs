use domain::case_stages::{
    CaseStage, CaseStageChange, DeclaredStagePrecision, DeclaredStageTime, StageAdoption,
    StageCourt, StageNote, StageSupportRef, StageTransition,
};
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::DomainError;
use time::macros::{date, datetime, offset};
use time::{Duration, UtcOffset};

fn support() -> StageSupportRef {
    StageSupportRef::new(
        DocumentVersionRef {
            id: DocumentId::new(),
            version: DocumentVersion::initial(),
        },
        Sha256Digest::from_array([1; 32]),
    )
}

fn adoption(at: DeclaredStageTime) -> CaseStageChange {
    CaseStageChange::Adopt(StageAdoption::new(
        CaseStage::Investigation,
        at,
        StageNote::new("Known case").unwrap(),
        support(),
    ))
}

#[test]
fn date_preserves_day_precision_and_the_entire_inclusive_interval() {
    let value = DeclaredStageTime::date(date!(2026 - 09 - 15), offset!(-6)).unwrap();
    assert_eq!(value.precision(), DeclaredStagePrecision::Date);
    assert_eq!(value.local_date(), date!(2026 - 09 - 15));
    assert_eq!(value.offset(), offset!(-6));
    assert_eq!(value.instant_value(), None);
    assert_eq!(value.lower_bound(), datetime!(2026-09-15 06:00 UTC));
    assert_eq!(
        value.upper_bound(),
        datetime!(2026-09-16 05:59:59.999_999_999 UTC)
    );
    assert_eq!(value.lower_bound().offset(), UtcOffset::UTC);
    assert_eq!(value.upper_bound().offset(), UtcOffset::UTC);
}

#[test]
fn instant_preserves_declared_offset_and_nanoseconds() {
    let input = datetime!(2026-09-15 00:15:12.123_456_789 +5:30);
    let value = DeclaredStageTime::instant(input).unwrap();
    assert_eq!(value.precision(), DeclaredStagePrecision::Instant);
    assert_eq!(value.local_date(), date!(2026 - 09 - 15));
    assert_eq!(value.offset(), offset!(+5:30));
    assert_eq!(value.instant_value().unwrap().offset(), offset!(+5:30));
    assert_eq!(value.instant_value(), Some(input));
    assert_eq!(
        value.lower_bound(),
        datetime!(2026-09-14 18:45:12.123_456_789 UTC)
    );
    assert_eq!(value.upper_bound(), value.lower_bound());
}

#[test]
fn explicit_offsets_allow_whole_minutes_through_fourteen_hours() {
    for seconds in [-50_400, -60, 0, 60, 50_400] {
        let offset = UtcOffset::from_whole_seconds(seconds).unwrap();
        assert!(DeclaredStageTime::date(date!(2026 - 09 - 15), offset).is_ok());
        assert!(
            DeclaredStageTime::instant(date!(2026 - 09 - 15).midnight().assume_offset(offset))
                .is_ok()
        );
    }
    for seconds in [-50_460, -1, 1, 50_460] {
        let offset = UtcOffset::from_whole_seconds(seconds).unwrap();
        assert_eq!(
            DeclaredStageTime::date(date!(2026 - 09 - 15), offset),
            Err(DomainError::InvalidDeclaredStageTime)
        );
        assert_eq!(
            DeclaredStageTime::instant(date!(2026 - 09 - 15).midnight().assume_offset(offset)),
            Err(DomainError::InvalidDeclaredStageTime)
        );
    }
}

#[test]
fn both_local_and_utc_years_must_be_representable() {
    for date in [date!(0000 - 01 - 01), date!(-0001 - 12 - 31)] {
        assert!(DeclaredStageTime::date(date, UtcOffset::UTC).is_err());
        assert!(DeclaredStageTime::instant(date.midnight().assume_utc()).is_err());
    }
    assert!(DeclaredStageTime::date(date!(0001 - 01 - 01), offset!(+0:01)).is_err());
    assert!(DeclaredStageTime::date(date!(9999 - 12 - 31), offset!(-0:01)).is_err());
    assert!(DeclaredStageTime::instant(datetime!(0001-01-01 00:00 +0:01)).is_err());
    assert!(DeclaredStageTime::instant(datetime!(9999-12-31 23:59:59.999_999_999 -0:01)).is_err());
    assert!(DeclaredStageTime::date(date!(0001 - 01 - 01), UtcOffset::UTC).is_ok());
    assert!(DeclaredStageTime::date(date!(9999 - 12 - 31), UtcOffset::UTC).is_ok());
    assert!(DeclaredStageTime::instant(datetime!(0001-01-01 00:00 UTC)).is_ok());
    assert!(DeclaredStageTime::instant(datetime!(9999-12-31 23:59:59.999_999_999 UTC)).is_ok());
}

#[test]
fn a_declared_day_is_future_only_before_its_lower_bound() {
    let change = adoption(DeclaredStageTime::date(date!(2026 - 09 - 15), offset!(-6)).unwrap());
    let start = datetime!(2026-09-15 06:00 UTC);
    assert_eq!(
        change.validate_recording_at(start - Duration::NANOSECOND),
        Err(DomainError::StageActInFuture)
    );
    assert!(change.validate_recording_at(start).is_ok());
    assert!(change.validate_recording_at(start + Duration::HOUR).is_ok());
}

#[test]
fn exact_instant_future_check_retains_nanosecond_precision() {
    let at = datetime!(2026-09-15 06:00:00.000_000_001 UTC);
    let change = adoption(DeclaredStageTime::instant(at).unwrap());
    assert_eq!(
        change.validate_recording_at(at - Duration::NANOSECOND),
        Err(DomainError::StageActInFuture)
    );
    assert!(change.validate_recording_at(at).is_ok());
}

#[test]
fn overlapping_declared_precision_does_not_invent_act_order() {
    let day = DeclaredStageTime::date(date!(2026 - 09 - 15), offset!(-6)).unwrap();
    let early = DeclaredStageTime::instant(datetime!(2026-09-15 06:01 UTC)).unwrap();
    let late = DeclaredStageTime::instant(datetime!(2026-09-16 05:59 UTC)).unwrap();
    for (issued, received) in [(day, early), (late, day), (day, day)] {
        assert!(StageTransition::to_trial(
            issued,
            support(),
            received,
            StageCourt::new("Court").unwrap(),
            None,
            None,
            None
        )
        .is_ok());
    }
}

#[test]
fn disjoint_intervals_reject_incompatible_order() {
    let issued = DeclaredStageTime::date(date!(2026 - 09 - 16), offset!(-6)).unwrap();
    let received = DeclaredStageTime::date(date!(2026 - 09 - 15), offset!(-6)).unwrap();
    assert_eq!(
        StageTransition::to_trial(
            issued,
            support(),
            received,
            StageCourt::new("Court").unwrap(),
            None,
            None,
            None
        ),
        Err(DomainError::InvalidStageActOrder)
    );
    let issued =
        DeclaredStageTime::instant(datetime!(2026-09-16 06:00:00.000_000_001 UTC)).unwrap();
    let received = DeclaredStageTime::instant(datetime!(2026-09-16 06:00 UTC)).unwrap();
    assert!(StageTransition::to_trial(
        issued,
        support(),
        received,
        StageCourt::new("Court").unwrap(),
        None,
        None,
        None
    )
    .is_err());
}

#[test]
fn transition_capture_checks_each_declared_time() {
    let at = datetime!(2026-09-15 06:00 UTC);
    let future = DeclaredStageTime::instant(at + Duration::NANOSECOND).unwrap();
    let past = DeclaredStageTime::instant(at - Duration::DAY).unwrap();
    let intermediate =
        CaseStageChange::Transition(StageTransition::to_intermediate(future, support(), None));
    assert_eq!(
        intermediate.validate_recording_at(at),
        Err(DomainError::StageActInFuture)
    );
    let trial = CaseStageChange::Transition(
        StageTransition::to_trial(
            past,
            support(),
            future,
            StageCourt::new("Court").unwrap(),
            None,
            None,
            None,
        )
        .unwrap(),
    );
    assert_eq!(
        trial.validate_recording_at(at),
        Err(DomainError::StageActInFuture)
    );
    assert!(trial
        .validate_recording_at(at + Duration::NANOSECOND)
        .is_ok());
}
