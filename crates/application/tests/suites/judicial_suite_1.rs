// Shared fixtures for related integration tests. See docs/adr/0046-cached-ci-test-suites.md.
#[allow(dead_code)]
#[path = "../case_support/mod.rs"]
mod case_support;
#[path = "../judicial_calendar_canonical.rs"]
mod judicial_calendar_canonical;
#[path = "../judicial_calendar_failures.rs"]
mod judicial_calendar_failures;
#[path = "../judicial_calendar_history_boundaries.rs"]
mod judicial_calendar_history_boundaries;
#[path = "../judicial_calendar_query.rs"]
mod judicial_calendar_query;
#[path = "../judicial_calendar_reads.rs"]
mod judicial_calendar_reads;
#[path = "../judicial_calendar_support/mod.rs"]
mod judicial_calendar_support;
#[path = "../judicial_calendar_workflow.rs"]
mod judicial_calendar_workflow;
