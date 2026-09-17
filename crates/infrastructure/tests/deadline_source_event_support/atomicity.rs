use super::{calendars, facts, hearing_change, results, Fixture, Saved};
use application::{
    hearing_results::HearingResultWorkflow, judicial_calendars::JudicialCalendarWorkflow,
    procedural_facts::ProceduralFactWorkflow, ApplicationError,
};
use domain::identity::Role;
use serde_json::Value;

pub fn prepared_correction(
    db: &Fixture,
    saved: &Saved,
) -> Box<dyn FnOnce() -> Result<(), ApplicationError>> {
    let case = db.case;
    match saved {
        Saved::Fact(base) => {
            let service = facts::service(db, db.owner, Role::Owner);
            let command = facts::correct(base);
            let draft = service.prepare("session", case, command.clone()).unwrap();
            Box::new(move || {
                service
                    .submit("session", case, command, draft.submission_digest)
                    .map(|_| ())
            })
        }
        Saved::Hearing(base) => {
            let service = results::service(db, db.owner, Role::Owner);
            let command = hearing_change(base, false);
            let draft = service.prepare("session", case, command.clone()).unwrap();
            Box::new(move || {
                service
                    .submit("session", case, command, draft.submission_digest)
                    .map(|_| ())
            })
        }
        Saved::Calendar(base) => {
            let service = calendars::service(db, db.owner, Role::Owner);
            let command = calendars::replace(base);
            let draft = service.prepare("session", command.clone()).unwrap();
            Box::new(move || {
                service
                    .submit("session", command, draft.submission_digest)
                    .map(|_| ())
            })
        }
    }
}

pub fn snapshot(db: &mut Fixture) -> Value {
    db.admin.query_one("SELECT jsonb_build_object(
        'events',(SELECT jsonb_agg(to_jsonb(e) ORDER BY sequence) FROM deadline_source_events e),
        'facts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY family,id,revision) FROM case_procedural_fact_revisions r),
        'results',(SELECT jsonb_agg(to_jsonb(r) ORDER BY result_id,revision) FROM case_hearing_result_revisions r),
        'calendars',(SELECT jsonb_agg(to_jsonb(r) ORDER BY calendar_id,revision) FROM judicial_calendar_revisions r),
        'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
}

pub fn reject_audit(db: &mut Fixture) {
    db.admin.batch_execute("CREATE FUNCTION reject_source_event_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected audit failure'; END; $$;
        CREATE TRIGGER reject_source_event_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_source_event_audit()").unwrap();
}

pub fn allow_audit(db: &mut Fixture) {
    db.admin.batch_execute("DROP TRIGGER reject_source_event_audit ON audit_events; DROP FUNCTION reject_source_event_audit()").unwrap();
}
