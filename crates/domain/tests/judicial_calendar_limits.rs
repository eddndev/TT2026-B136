mod judicial_calendar_support;
use domain::judicial_calendars::*;
use judicial_calendar_support::*;
use time::{Date, Month};
use uuid::Uuid;

#[test]
fn scope_limits_count_unicode_scalars_and_all_entities_are_explicit() {
    let maximum = "\u{1f642}".repeat(200);
    let description = "\u{1f642}".repeat(1000);
    let codes: Vec<_> = (1..=32).map(|i| format!("{i:02}")).collect();
    let refs: Vec<_> = codes.iter().map(String::as_str).collect();
    let value = JudicialCalendarScope::new(JudicialCalendarScopeInput {
        title: &maximum,
        jurisdiction: JudicialCalendarJurisdiction::Local,
        entity_codes: &refs,
        authority: &maximum,
        organ: &maximum,
        territory: &maximum,
        use_description: &description,
    })
    .unwrap();
    assert_eq!(value.entity_codes().len(), 32);
    assert_eq!(value.title().len(), 800);
    for field in [
        "title",
        "authority",
        "organ",
        "territory",
        "use_description",
    ] {
        let oversized = "\u{1f642}".repeat(if field == "use_description" {
            1001
        } else {
            201
        });
        assert!(
            JudicialCalendarScope::new(JudicialCalendarScopeInput {
                title: if field == "title" { &oversized } else { "T" },
                jurisdiction: JudicialCalendarJurisdiction::Federal,
                entity_codes: &["01"],
                authority: if field == "authority" {
                    &oversized
                } else {
                    "A"
                },
                organ: if field == "organ" { &oversized } else { "O" },
                territory: if field == "territory" {
                    &oversized
                } else {
                    "T"
                },
                use_description: if field == "use_description" {
                    &oversized
                } else {
                    "D"
                },
            })
            .is_err(),
            "{field}"
        );
    }
}
#[test]
fn source_text_and_url_have_distinct_scalar_and_ascii_byte_limits() {
    let maximum = "\u{1f642}".repeat(200);
    let locator = "\u{1f642}".repeat(512);
    let prefix = "https://example.org/";
    let url = format!("{prefix}{}", "a".repeat(2048 - prefix.len()));
    assert!(JudicialCalendarSource::new(JudicialCalendarSourceInput {
        id: Uuid::nil(),
        title: &maximum,
        issuer: &maximum,
        official_url: &url,
        published_on: Some(date("9999-12-31")),
        consulted_on: date("9999-12-31"),
        locator: &locator,
    })
    .is_ok());
    for field in ["title", "issuer", "locator", "url"] {
        let too_long = "a".repeat(if field == "locator" { 513 } else { 201 });
        let too_long_url = format!("{url}a");
        assert!(
            JudicialCalendarSource::new(JudicialCalendarSourceInput {
                id: Uuid::nil(),
                title: if field == "title" { &too_long } else { "T" },
                issuer: if field == "issuer" { &too_long } else { "I" },
                official_url: if field == "url" {
                    &too_long_url
                } else {
                    prefix
                },
                published_on: None,
                consulted_on: date("2026-01-01"),
                locator: if field == "locator" { &too_long } else { "L" },
            })
            .is_err(),
            "{field}"
        );
    }
    for invalid in [
        "",
        " ",
        "line\nline",
        "line\r\nline",
        "\tline",
        "line\u{7f}",
    ] {
        assert!(JudicialCalendarSource::new(JudicialCalendarSourceInput {
            id: Uuid::nil(),
            title: invalid,
            issuer: "I",
            official_url: prefix,
            published_on: None,
            consulted_on: date("2026-01-01"),
            locator: "L",
        })
        .is_err());
    }
}
#[test]
fn all_collection_cardinalities_and_rule_limits_are_enforced() {
    let coverage = JudicialCalendarCoverage::new(date("2026-01-01"), date("2026-12-31")).unwrap();
    let sources: Vec<_> = (0..16).map(source).collect();
    assert!(
        JudicialCalendarValues::new(scope(), coverage, sources.clone(), weekly(), vec![]).is_ok()
    );
    let mut excess = sources;
    excess.push(source(16));
    assert!(JudicialCalendarValues::new(scope(), coverage, excess, weekly(), vec![]).is_err());
    let ids: Vec<_> = (0..16).map(Uuid::from_u128).collect();
    assert!(JudicialCalendarRule::new(
        JudicialCalendarClassification::Countable,
        ids.clone(),
        &"\u{1f642}".repeat(256)
    )
    .is_ok());
    assert!(JudicialCalendarRule::new(
        JudicialCalendarClassification::Countable,
        ids,
        &"a".repeat(257)
    )
    .is_err());
    assert!(JudicialCalendarRule::new(
        JudicialCalendarClassification::Unresolved,
        (0..17).map(Uuid::from_u128).collect(),
        "D"
    )
    .is_err());
    let exceptions: Vec<_> = (0..65)
        .map(|i| {
            let day = CivilDate::from_days_since_epoch(date("2026-01-01").days_since_epoch() + i)
                .unwrap();
            JudicialCalendarException::new(
                Uuid::from_u128(i as u128),
                day,
                day,
                rule(JudicialCalendarClassification::Countable),
            )
            .unwrap()
        })
        .collect();
    assert!(JudicialCalendarValues::new(
        scope(),
        coverage,
        vec![source(0)],
        weekly(),
        exceptions[..64].to_vec()
    )
    .is_ok());
    assert!(
        JudicialCalendarValues::new(scope(), coverage, vec![source(0)], weekly(), exceptions)
            .is_err()
    );
}
#[test]
fn civil_epoch_conversion_and_catalogs_reject_out_of_range_values() {
    for value in [i32::MIN, -719163, 2932897, i32::MAX] {
        assert!(CivilDate::from_days_since_epoch(value).is_err());
    }
    assert!(CivilDate::from_date(Date::from_calendar_date(0, Month::January, 1).unwrap()).is_err());
    for value in [
        JudicialCalendarClassification::Countable,
        JudicialCalendarClassification::Excluded,
        JudicialCalendarClassification::Unresolved,
    ] {
        assert_eq!(
            value
                .as_str()
                .parse::<JudicialCalendarClassification>()
                .unwrap(),
            value
        );
    }
    assert!("outside_coverage"
        .parse::<JudicialCalendarClassification>()
        .is_err());
    assert!("Federal".parse::<JudicialCalendarJurisdiction>().is_err());
    assert!("active".parse::<JudicialCalendarStatus>().is_err());
    assert_eq!(
        "published".parse::<JudicialCalendarStatus>().unwrap(),
        JudicialCalendarStatus::Published
    );
    assert_eq!(
        "retired".parse::<JudicialCalendarStatus>().unwrap(),
        JudicialCalendarStatus::Retired
    );
}
