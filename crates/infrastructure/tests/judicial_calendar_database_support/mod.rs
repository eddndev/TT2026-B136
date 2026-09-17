#![allow(dead_code)]
pub use super::case_administration_support::Fixture;
use super::case_stage_database_support::{FixedClock, TestIdentity};
use application::{identity::Principal, judicial_calendars::*};
use domain::identity::{Role, UserId};
use infrastructure::{PostgresJudicialCalendarStore, RingSha256Hasher};
use std::sync::Arc;

pub fn store(db: &Fixture) -> Arc<PostgresJudicialCalendarStore> {
    Arc::new(
        PostgresJudicialCalendarStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}
pub fn service(db: &Fixture, actor: UserId, role: Role) -> JudicialCalendarService {
    JudicialCalendarService::new(
        store(db),
        Arc::new(TestIdentity(Principal {
            id: actor,
            email: "session@example.test".into(),
            role,
        })),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}
pub fn values(title: &str, explanation: &str) -> JudicialCalendarValues {
    let scope = JudicialCalendarScope::new(JudicialCalendarScopeInput {
        title,
        jurisdiction: JudicialCalendarJurisdiction::Federal,
        entity_codes: &["09"],
        authority: "Authority",
        organ: "Court",
        territory: "Territory",
        use_description: "Declared purpose",
    })
    .unwrap();
    let weekly = (1..=7)
        .map(|day| {
            JudicialCalendarWeekdayRule::new(
                day,
                JudicialCalendarRule::new(
                    JudicialCalendarClassification::Unresolved,
                    vec![],
                    explanation,
                )
                .unwrap(),
            )
            .unwrap()
        })
        .collect();
    JudicialCalendarValues::new(
        scope,
        JudicialCalendarCoverage::new("2026-01-01".parse().unwrap(), "2026-12-31".parse().unwrap())
            .unwrap(),
        vec![],
        weekly,
        vec![],
    )
    .unwrap()
}
pub fn publish() -> JudicialCalendarCommand {
    JudicialCalendarCommand {
        operation_id: JudicialCalendarOperationId::new(),
        calendar_id: JudicialCalendarId::new(),
        change: JudicialCalendarChange::Publish {
            values: values("Calendar", "Review source"),
        },
    }
}
pub fn replace(base: &JudicialCalendarDetail) -> JudicialCalendarCommand {
    JudicialCalendarCommand {
        operation_id: JudicialCalendarOperationId::new(),
        calendar_id: base.id,
        change: JudicialCalendarChange::Replace {
            expected_revision: base.revision,
            values: values("Calendar", "Still unresolved after review"),
            reason: JudicialCalendarReason::new("Rechecked declared sources").unwrap(),
        },
    }
}
pub fn retire(base: &JudicialCalendarDetail) -> JudicialCalendarCommand {
    JudicialCalendarCommand {
        operation_id: JudicialCalendarOperationId::new(),
        calendar_id: base.id,
        change: JudicialCalendarChange::Retire {
            expected_revision: base.revision,
            reason: JudicialCalendarReason::new("Retired reference").unwrap(),
        },
    }
}
pub fn persist(
    service: &JudicialCalendarService,
    command: JudicialCalendarCommand,
) -> JudicialCalendarDetail {
    let draft = service.prepare("session", command.clone()).unwrap();
    service
        .submit("session", command, draft.submission_digest)
        .unwrap()
}
pub fn counts(db: &mut Fixture) -> (i64, i64, i64) {
    let row = db
        .admin
        .query_one(
            "SELECT (SELECT count(*) FROM judicial_calendars),
        (SELECT count(*) FROM judicial_calendar_revisions),(SELECT count(*) FROM audit_events)",
            &[],
        )
        .unwrap();
    (row.get(0), row.get(1), row.get(2))
}
pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object(
        'roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM judicial_calendars r),
        'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY calendar_id,revision) FROM judicial_calendar_revisions r),
        'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
}
