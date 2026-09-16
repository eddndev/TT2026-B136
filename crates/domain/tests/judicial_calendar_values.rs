mod judicial_calendar_support;
use domain::{
    identity::{Permission, Role},
    judicial_calendars::*,
    DomainError,
};
use judicial_calendar_support::*;
use uuid::Uuid;

#[test]
fn entities_are_explicit_exact_unique_and_independent_from_jurisdiction() {
    for jurisdiction in [
        JudicialCalendarJurisdiction::Federal,
        JudicialCalendarJurisdiction::Local,
    ] {
        for codes in [vec!["09"], vec!["32", "01"]] {
            let result = JudicialCalendarScope::new(JudicialCalendarScopeInput {
                title: " Scope ",
                jurisdiction,
                entity_codes: &codes,
                authority: "A",
                organ: "O",
                territory: "T",
                use_description: "Line 1\r\nLine 2",
            })
            .unwrap();
            assert_eq!(result.title(), "Scope");
            assert_eq!(result.use_description(), "Line 1\nLine 2");
            assert!(result.entity_codes().windows(2).all(|w| w[0] < w[1]));
        }
    }
    for codes in [
        vec![],
        vec!["09", "09"],
        vec!["1"],
        vec!["00"],
        vec!["33"],
        vec![" 01"],
        vec!["01 "],
        vec!["001"],
        vec!["\u{ff10}\u{ff11}"],
    ] {
        assert!(JudicialCalendarScope::new(JudicialCalendarScopeInput {
            title: "S",
            jurisdiction: JudicialCalendarJurisdiction::Federal,
            entity_codes: &codes,
            authority: "A",
            organ: "O",
            territory: "T",
            use_description: "D",
        })
        .is_err());
    }
}
#[test]
fn source_urls_are_https_with_no_user_information_or_repairs() {
    for url in [
        "http://example.org",
        "https:///path",
        "https://",
        "https://a@host/",
        "https://@host/",
        "https://a:b@host/",
        "https://host\\path",
        "https://host/a b",
        "https://host/\t",
        "https://host/\u{e9}",
        "https://[invalid]/",
        "https://host:99999/",
    ] {
        assert!(
            JudicialCalendarSource::new(JudicialCalendarSourceInput {
                id: Uuid::nil(),
                title: "T",
                issuer: "I",
                official_url: url,
                published_on: None,
                consulted_on: date("0001-01-01"),
                locator: "L",
            })
            .is_err(),
            "{url}"
        );
    }
    let source = JudicialCalendarSource::new(JudicialCalendarSourceInput {
        id: Uuid::nil(),
        title: "T",
        issuer: "I",
        official_url: " https://Example.org/a?q=%20#part ",
        published_on: None,
        consulted_on: date("9999-12-31"),
        locator: "L",
    })
    .unwrap();
    assert_eq!(source.official_url(), "https://Example.org/a?q=%20#part");
    assert!(JudicialCalendarSource::new(JudicialCalendarSourceInput {
        id: Uuid::nil(),
        title: "T",
        issuer: "I",
        official_url: "https://host/",
        published_on: Some(date("2026-01-02")),
        consulted_on: date("2026-01-01"),
        locator: "L",
    })
    .is_err());
}
#[test]
fn rules_require_sources_unless_unresolved_and_never_deduplicate() {
    for classification in [
        JudicialCalendarClassification::Countable,
        JudicialCalendarClassification::Excluded,
    ] {
        assert!(JudicialCalendarRule::new(classification, vec![], "D").is_err());
    }
    assert!(
        JudicialCalendarRule::new(JudicialCalendarClassification::Unresolved, vec![], "D").is_ok()
    );
    assert!(JudicialCalendarRule::new(
        JudicialCalendarClassification::Unresolved,
        vec![Uuid::nil(), Uuid::nil()],
        "D"
    )
    .is_err());
    assert!(
        JudicialCalendarRule::new(JudicialCalendarClassification::Unresolved, vec![], "\tbad")
            .is_err()
    );
}
#[test]
fn collections_reject_missing_sources_duplicate_ids_and_incomplete_weeks() {
    let coverage = JudicialCalendarCoverage::new(date("2026-01-01"), date("2026-12-31")).unwrap();
    for sources in [vec![], vec![source(0), source(0)]] {
        assert!(JudicialCalendarValues::new(scope(), coverage, sources, weekly(), vec![]).is_err());
    }
    for week in [weekly()[..6].to_vec(), vec![weekly()[0].clone(); 7]] {
        assert!(
            JudicialCalendarValues::new(scope(), coverage, vec![source(0)], week, vec![]).is_err()
        );
    }
    assert!(
        JudicialCalendarWeekdayRule::new(0, rule(JudicialCalendarClassification::Countable))
            .is_err()
    );
    assert!(
        JudicialCalendarWeekdayRule::new(8, rule(JudicialCalendarClassification::Countable))
            .is_err()
    );
}
#[test]
fn exceptions_reject_overlaps_outside_coverage_and_duplicate_ids() {
    let make = |id, from, through| {
        JudicialCalendarException::new(
            Uuid::from_u128(id),
            date(from),
            date(through),
            rule(JudicialCalendarClassification::Countable),
        )
        .unwrap()
    };
    let coverage = JudicialCalendarCoverage::new(date("2026-01-01"), date("2026-12-31")).unwrap();
    for exceptions in [
        vec![make(1, "2025-12-31", "2026-01-01")],
        vec![
            make(1, "2026-01-01", "2026-01-02"),
            make(2, "2026-01-02", "2026-01-03"),
        ],
        vec![
            make(1, "2026-01-01", "2026-01-01"),
            make(1, "2026-01-03", "2026-01-03"),
        ],
    ] {
        assert!(JudicialCalendarValues::new(
            scope(),
            coverage,
            vec![source(0)],
            weekly(),
            exceptions
        )
        .is_err());
    }
    assert!(JudicialCalendarException::new(
        Uuid::nil(),
        date("2026-01-02"),
        date("2026-01-01"),
        rule(JudicialCalendarClassification::Countable)
    )
    .is_err());
}
#[test]
fn revisions_and_global_permissions_are_explicit() {
    assert_eq!(
        JudicialCalendarRevision::new(0),
        Err(DomainError::InvalidJudicialCalendarRevision)
    );
    assert_eq!(
        JudicialCalendarRevision::new(1)
            .unwrap()
            .next()
            .unwrap()
            .get(),
        2
    );
    assert_eq!(
        JudicialCalendarRevision::new(u32::MAX).unwrap().next(),
        Err(DomainError::JudicialCalendarRevisionExhausted)
    );
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        assert!(role.allows(Permission::ReadJudicialCalendar));
    }
    for role in [Role::Litigator, Role::Paralegal, Role::Client] {
        assert!(!role.allows(Permission::ManageJudicialCalendar));
    }
    assert!(!Role::Client.allows(Permission::ReadJudicialCalendar));
    assert!(Role::Owner.allows(Permission::ManageJudicialCalendar));
    assert_eq!(
        JudicialCalendarId::from_uuid(Uuid::nil()).as_uuid(),
        Uuid::nil()
    );
}

#[test]
fn reasons_preserve_normalized_multiline_text_and_scalar_limit() {
    assert_eq!(
        JudicialCalendarReason::new(" A\r\nB ").unwrap().as_str(),
        "A\nB"
    );
    assert!(JudicialCalendarReason::new(&"\u{1f642}".repeat(1000)).is_ok());
    assert!(JudicialCalendarReason::new(&"a".repeat(1001)).is_err());
    assert!(JudicialCalendarReason::new("\tbad").is_err());
    assert!(JudicialCalendarReason::new(" ").is_err());
}
