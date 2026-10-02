// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[path = "../judicial_calendar_canonical.rs"]
mod judicial_calendar_canonical;
#[path = "../judicial_calendar_dates.rs"]
mod judicial_calendar_dates;
#[path = "../judicial_calendar_limits.rs"]
mod judicial_calendar_limits;
#[path = "../judicial_calendar_support/mod.rs"]
mod judicial_calendar_support;
#[path = "../judicial_calendar_urls.rs"]
mod judicial_calendar_urls;
#[path = "../judicial_calendar_values.rs"]
mod judicial_calendar_values;
