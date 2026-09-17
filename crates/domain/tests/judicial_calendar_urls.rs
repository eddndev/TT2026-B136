mod judicial_calendar_support;
use domain::judicial_calendars::*;
use judicial_calendar_support::date;
use uuid::Uuid;
fn parse(url: &str) -> Result<JudicialCalendarSource, domain::DomainError> {
    JudicialCalendarSource::new(JudicialCalendarSourceInput {
        id: Uuid::nil(),
        title: "T",
        issuer: "I",
        official_url: url,
        published_on: None,
        consulted_on: date("2026-01-01"),
        locator: "L",
    })
}
#[test]
fn reference_url_profile_rejects_general_url_features_and_malformed_escapes() {
    for value in [
        "HTTPS://example.org",
        "https://example.org:443/",
        "https://localhost/",
        "https://127.0.0.1/",
        "https://[::1]/",
        "https://xn--bcher-kva.example/",
        "https://Xn--site.example/",
        "https://a.xn--p1ai/",
        "https://example.9/",
        "https://example.c/",
        "https://-bad.example/",
        "https://bad-.example/",
        "https://a..example/",
        "https://example.org./",
        "https://example.org/%",
        "https://example.org/%2",
        "https://example.org/%xz",
        "https://example.org/a#b#c",
        "https://example.org/{}",
        "https://example.org/\"",
        "https://example.org/[x]",
        "https://example.org/<x>",
    ] {
        assert!(parse(value).is_err(), "{value}");
    }
}
#[test]
fn reference_url_profile_preserves_accepted_bytes() {
    for value in [
        "https://example.org",
        "https://ExAmPle.org/a?b=%20#c",
        "https://1a.b-example.org?x=1",
        "https://example.org#part",
        "https://example.org/-._~!$&'()*+,;=:@/?#",
    ] {
        assert_eq!(parse(value).unwrap().official_url(), value);
    }
    assert!(parse(&format!("https://{}.org", "a".repeat(64))).is_err());
    assert!(parse(&format!("https://{}.org", "a".repeat(63))).is_ok());
    assert!(parse(&format!(
        "https://{}.{}.{}.{}.org",
        "a".repeat(63),
        "a".repeat(63),
        "a".repeat(63),
        "a".repeat(63)
    ))
    .is_err());
}
