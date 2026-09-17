use super::*;
#[path = "../judicial_calendar_support/mod.rs"]
mod calendars;
#[path = "../procedural_fact_hearing_support/mod.rs"]
mod sessions;
use application::{hearing_results::*, hearings::HearingSchedulingContext, judicial_calendars::*};
use domain::{
    case_stages::CaseStageRevision,
    hearings::{HearingKind, HearingStatus, HearingTime},
};
use time::UtcOffset;

pub fn hearing(
    revision: u32,
    withdrawn: bool,
    at: &str,
    agreements: &[u128],
) -> HearingResultDetail {
    let mut snapshot = sessions::source(20, 30, revision, withdrawn, agreements);
    let mut input = sessions::values_input(&snapshot.values);
    input.event_time =
        DeclaredHearingResultTime::date(at.parse::<CivilDate>().unwrap().date(), UtcOffset::UTC)
            .unwrap();
    input.provenance =
        HearingResultProvenance::new(HearingResultProvenanceKind::OperatorNote, None, None)
            .unwrap();
    snapshot.values = HearingResultValues::new(input).unwrap();
    let anchor = HearingResultAnchorSnapshot {
        reference: snapshot.anchor,
        status: HearingStatus::Scheduled,
        kind: HearingKind::Initial,
        scheduled_at: HearingTime::new(snapshot.recorded_at).unwrap(),
        scheduling_context: HearingSchedulingContext {
            administration_revision: snapshot.recorded_administration_revision,
            administration_digest: snapshot.recorded_administration_digest,
            stage_revision: CaseStageRevision::FIRST,
            stage: HearingKind::Initial.required_stage(),
            stage_digest: None,
        },
    };
    let mut detail = HearingResultDetail {
        snapshot,
        anchor,
        continuation: None,
        attendees: vec![],
        support: None,
    };
    resign_hearing(&mut detail);
    hearing_result_receipt_matches(hasher().as_ref(), &detail).unwrap();
    detail
}
pub fn resign_hearing(detail: &mut HearingResultDetail) {
    detail.snapshot.values_digest =
        hearing_result_values_digest(hasher().as_ref(), &detail.snapshot.values);
    detail.snapshot.receipt.submission_digest =
        hasher().hash_bytes(&sessions::receipt_bytes(&detail.snapshot));
}
pub fn calendar(
    revision: u32,
    retired: bool,
    classification: JudicialCalendarClassification,
) -> JudicialCalendarDetail {
    let original = calendars::values("Declared calendar");
    let source_id = Uuid::from_u128(88);
    let sources = vec![JudicialCalendarSource::new(JudicialCalendarSourceInput {
        id: source_id,
        title: "Declared public reference",
        issuer: "Declared authority",
        official_url: "https://example.com/calendar",
        published_on: None,
        consulted_on: "2026-01-01".parse().unwrap(),
        locator: "Declared section",
    })
    .unwrap()];
    let pattern = (1..=7)
        .map(|day| {
            JudicialCalendarWeekdayRule::new(
                day,
                JudicialCalendarRule::new(
                    classification,
                    vec![source_id],
                    "Declared classification",
                )
                .unwrap(),
            )
            .unwrap()
        })
        .collect();
    let values = JudicialCalendarValues::new(
        original.scope().clone(),
        original.coverage(),
        sources,
        pattern,
        vec![],
    )
    .unwrap();
    let change = if retired {
        JudicialCalendarChange::Retire {
            expected_revision: JudicialCalendarRevision::new(revision - 1).unwrap(),
            reason: JudicialCalendarReason::new("Retired").unwrap(),
        }
    } else if revision == 1 {
        JudicialCalendarChange::Publish {
            values: values.clone(),
        }
    } else {
        JudicialCalendarChange::Replace {
            expected_revision: JudicialCalendarRevision::new(revision - 1).unwrap(),
            values: values.clone(),
            reason: JudicialCalendarReason::new("Changed").unwrap(),
        }
    };
    let command = JudicialCalendarCommand {
        operation_id: JudicialCalendarOperationId::from_uuid(Uuid::from_u128(
            300 + u128::from(revision),
        )),
        calendar_id: JudicialCalendarId::from_uuid(Uuid::from_u128(40)),
        change,
    };
    let mut detail = calendars::detail(actor(), &command, values);
    resign_calendar(&mut detail);
    judicial_calendar_receipt_matches(hasher().as_ref(), &detail).unwrap();
    detail
}
pub fn resign_calendar(detail: &mut JudicialCalendarDetail) {
    let change = match detail.receipt.action {
        JudicialCalendarAction::Publish => JudicialCalendarChange::Publish {
            values: detail.values.clone(),
        },
        JudicialCalendarAction::Replace => JudicialCalendarChange::Replace {
            expected_revision: JudicialCalendarRevision::new(detail.receipt.expected_revision)
                .unwrap(),
            values: detail.values.clone(),
            reason: detail.reason.clone().unwrap(),
        },
        JudicialCalendarAction::Retire => JudicialCalendarChange::Retire {
            expected_revision: JudicialCalendarRevision::new(detail.receipt.expected_revision)
                .unwrap(),
            reason: detail.reason.clone().unwrap(),
        },
    };
    let command = JudicialCalendarCommand {
        operation_id: detail.receipt.operation_id,
        calendar_id: detail.id,
        change,
    };
    detail.values_digest = judicial_calendar_values_digest(hasher().as_ref(), &detail.values);
    detail.receipt.submission_digest = judicial_calendar_submission_digest(
        hasher().as_ref(),
        detail.recorded_by.id,
        &command,
        detail.values_digest,
    );
}
