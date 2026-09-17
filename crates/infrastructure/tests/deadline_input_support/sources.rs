use super::Fixture;
use crate::{
    hearing_database_support as hearings, hearing_result_database_support as results,
    judicial_calendar_database_support as calendars, procedural_fact_backend_support as facts,
};
use application::{hearing_results::*, judicial_calendars::*, procedural_facts::*};
use domain::{identity::Role, procedural_time::DeclaredProceduralTime};

pub fn resolution_at(values: &ResolutionValues, date: &str) -> ResolutionValues {
    ResolutionValues::new(ResolutionValuesInput {
        class: values.class().clone(),
        subtype: values.subtype().cloned(),
        issuer: values.issuer().clone(),
        issued_at: DeclaredProceduralTime::date(date.parse().unwrap(), None).unwrap(),
        summary: values.summary().clone(),
        provenance: values.provenance().clone(),
    })
}
pub fn correct_resolution(base: &FactDetail, date: &str) -> ProceduralFactCommand {
    let ProceduralFactSnapshot::Resolution(snapshot) = &base.snapshot else {
        panic!("resolution expected")
    };
    ProceduralFactCommand::Resolution(ResolutionCommand::new(
        FactOperationId::new(),
        snapshot.root.id(),
        FactChange::correct(
            snapshot.metadata.revision,
            resolution_at(&snapshot.values, date),
            facts::text("Correct declared date"),
        ),
    ))
}
pub fn notification_at(parent: FactResolutionRef, date: &str) -> NotificationValues {
    let value = facts::notification_values(parent, "Dated notification");
    NotificationValues::new(NotificationValuesInput {
        resolution: parent,
        character: value.character().clone(),
        medium: value.medium().clone(),
        context: value.context().clone(),
        outcome: value.outcome().clone(),
        subtype: None,
        practiced_at: DeclaredProceduralTime::date(date.parse().unwrap(), None).unwrap(),
        received_at: None,
        stated_effect: None,
        intended_recipient: value.intended_recipient().clone(),
        actual_receiver: value.actual_receiver().clone(),
        representation: value.representation().clone(),
        summary: value.summary().clone(),
        provenance: value.provenance().clone(),
    })
    .unwrap()
}
pub fn calendar_values() -> JudicialCalendarValues {
    let base = calendars::values("Calendar", "Declared classification");
    let source = JudicialCalendarSource::new(JudicialCalendarSourceInput {
        id: uuid::Uuid::nil(),
        title: "Declared reference",
        issuer: "Declared authority",
        official_url: "https://example.test/calendar",
        published_on: None,
        consulted_on: "2026-01-01".parse().unwrap(),
        locator: "Weekly classification",
    })
    .unwrap();
    let weekly = (1..=7)
        .map(|day| {
            JudicialCalendarWeekdayRule::new(
                day,
                JudicialCalendarRule::new(
                    JudicialCalendarClassification::Countable,
                    vec![source.id()],
                    "Declared countable day",
                )
                .unwrap(),
            )
            .unwrap()
        })
        .collect();
    JudicialCalendarValues::new(
        base.scope().clone(),
        base.coverage(),
        vec![source],
        weekly,
        vec![],
    )
    .unwrap()
}
pub fn calendar(db: &Fixture) -> JudicialCalendarDetail {
    let mut command = calendars::publish();
    command.change = JudicialCalendarChange::Publish {
        values: calendar_values(),
    };
    calendars::persist(&calendars::service(db, db.owner, Role::Owner), command)
}
pub fn hearing_values(agreement: bool, instant: bool) -> HearingResultValues {
    let base = results::values("Declared session");
    let event_time = if instant {
        DeclaredHearingResultTime::instant(
            time::Date::from_calendar_date(2024, time::Month::December, 30)
                .unwrap()
                .with_hms(14, 15, 16)
                .unwrap()
                .assume_offset(time::UtcOffset::from_hms(-6, 0, 0).unwrap()),
        )
        .unwrap()
    } else {
        base.event_time()
    };
    HearingResultValues::new(HearingResultValuesInput {
        occurrence: base.occurrence(),
        extent: base.extent(),
        event_time,
        summary: base.summary().clone(),
        attendees: vec![],
        agreements: if agreement {
            vec![HearingResultAgreement::new(
                HearingResultAgreementId::from_uuid(uuid::Uuid::nil()),
                HearingResultText::new("Exact historical agreement").unwrap(),
            )]
        } else {
            vec![]
        },
        provenance: base.provenance().clone(),
    })
    .unwrap()
}
pub fn hearing_result(db: &mut Fixture, instant: bool) -> HearingResultDetail {
    hearings::complete(db);
    let hearing = hearings::persist(
        &hearings::service(db, db.owner, Role::Owner),
        db.case,
        hearings::schedule(),
    );
    let mut command = results::record(hearing.snapshot.id);
    let HearingResultChange::Record { values, .. } = &mut command.change else {
        unreachable!()
    };
    *values = hearing_values(true, instant);
    results::persist(
        &results::service(db, db.owner, Role::Owner),
        db.case,
        command,
    )
}
pub fn corrected_result(db: &Fixture, first: &HearingResultDetail) -> HearingResultDetail {
    results::persist(
        &results::service(db, db.owner, Role::Owner),
        db.case,
        HearingResultCommand {
            operation_id: HearingResultOperationId::new(),
            hearing_id: first.snapshot.hearing_id,
            result_id: first.snapshot.id,
            change: HearingResultChange::Correct {
                expected_revision: first.snapshot.revision,
                values: hearing_values(false, false),
                reason: HearingResultText::new("Correct session and remove agreement").unwrap(),
            },
        },
    )
}
pub fn retired_result(db: &Fixture, base: &HearingResultDetail) -> HearingResultDetail {
    results::persist(
        &results::service(db, db.owner, Role::Owner),
        db.case,
        HearingResultCommand {
            operation_id: HearingResultOperationId::new(),
            hearing_id: base.snapshot.hearing_id,
            result_id: base.snapshot.id,
            change: HearingResultChange::Withdraw {
                expected_revision: base.snapshot.revision,
                reason: HearingResultText::new("Withdraw declaration").unwrap(),
            },
        },
    )
}
