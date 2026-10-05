pub use crate::alert_backend_support::{drive, operation, query, store, MutableClock};
pub use crate::case_administration_support::Fixture;
pub use application::{
    alerts::*, cases::CaseRepository, identity::Principal, precautionary_hearings::*,
};
pub use domain::{
    clock::OffsetDateTime,
    hearings::{HearingNote, HearingTime, HearingVenue},
    identity::UserId,
    precautionary_hearings::*,
};
pub use infrastructure::{PostgresAlertStore, RingSha256Hasher};
pub use std::sync::Arc;
pub use time::Duration;

pub fn set_time(command: &mut PrecautionaryHearingCommand, due: OffsetDateTime) {
    let values = match &mut command.change {
        PrecautionaryHearingChange::Schedule { values, .. }
        | PrecautionaryHearingChange::Replace { values, .. } => values,
        PrecautionaryHearingChange::Cancel { .. } => panic!("cancellation retains its time"),
    };
    let mut input = crate::hearing_fixture::input(values);
    input.scheduled_at = HearingTime::new(due).unwrap();
    *values = PrecautionaryHearingValues::new(input).unwrap();
}
pub fn simple(
    db: &mut Fixture,
    due: OffsetDateTime,
) -> (Principal, PrecautionaryHearingRecordStoredOperation) {
    let (actor, mut command) = crate::hearing_fixture::setup(db);
    set_time(&mut command, due);
    let result = crate::record_fixture::persist(db, actor.clone(), command);
    (actor, result)
}
pub fn subject(value: &PrecautionaryHearingRecordStoredOperation) -> AlertSubject {
    AlertSubject::PrecautionaryHearing {
        case_id: value.capture.review.case_id,
        id: value.capture.review.command.hearing_id,
    }
}
pub fn origin(value: &PrecautionaryHearingRecordStoredOperation) -> AlertOrigin {
    AlertOrigin {
        revision: value.capture.review.result_revision.get(),
        evidence_digest: value.capture.capture_digest,
    }
}
pub fn own_alerts(store: &PostgresAlertStore, user: UserId) -> Vec<AlertRecord> {
    store
        .list(user, query(20))
        .unwrap()
        .alerts
        .into_iter()
        .filter(|row| matches!(row.subject, AlertSubject::PrecautionaryHearing { .. }))
        .collect()
}
pub fn replacement(
    prior: &PrecautionaryHearingRecordStoredOperation,
    due: OffsetDateTime,
) -> PrecautionaryHearingCommand {
    let capture = &prior.capture;
    let mut values = crate::hearing_fixture::input(&capture.review.resolved_values);
    values.venue = HearingVenue::new("Updated court for the declared appointment").unwrap();
    values.scheduled_at = HearingTime::new(due).unwrap();
    PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: capture.review.command.hearing_id,
        change: PrecautionaryHearingChange::Replace {
            expected_revision: capture.review.result_revision,
            expected_capture_digest: capture.capture_digest,
            context: crate::hearing_fixture::expectation(&capture.review.observed_context),
            values: PrecautionaryHearingValues::new(values).unwrap(),
            reason: HearingNote::new("Updated communicated appointment").unwrap(),
        },
    }
}
pub fn cancellation(
    prior: &PrecautionaryHearingRecordStoredOperation,
) -> PrecautionaryHearingCommand {
    PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: prior.capture.review.command.hearing_id,
        change: PrecautionaryHearingChange::Cancel {
            expected_revision: prior.capture.review.result_revision,
            expected_capture_digest: prior.capture.capture_digest,
            reason: HearingNote::new("Declared appointment cancellation").unwrap(),
        },
    }
}
pub fn open(
    db: &Fixture,
    clock: Arc<MutableClock>,
) -> Result<PostgresAlertStore, application::ApplicationError> {
    PostgresAlertStore::open(&db.runtime_url, Arc::new(RingSha256Hasher), clock, None)
}
pub fn generation(db: &mut Fixture, id: PrecautionaryHearingId) -> i64 {
    db.admin
        .query_one(
            "SELECT generation FROM alert_subject_state WHERE kind=3 AND id=$1",
            &[&id.as_uuid()],
        )
        .unwrap()
        .get(0)
}
pub fn initial_decision(
    db: &mut Fixture,
    due: OffsetDateTime,
    id: uuid::Uuid,
) -> domain::cases::CaseId {
    use application::{hearings::HearingChange, precautionary_measures::MeasureDecisionAnchorRef};
    let seed = crate::measure_fixture::setup(db);
    let mut command = crate::hearing_database_support::schedule();
    command.hearing_id = domain::hearings::HearingId::from_uuid(id);
    let HearingChange::Schedule { values, .. } = &mut command.change else {
        unreachable!()
    };
    *values = crate::hearing_database_support::values(
        &due.format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
    );
    let hearing = crate::hearing_database_support::persist(
        &crate::hearing_database_support::service(db, db.owner, domain::identity::Role::Owner),
        db.case,
        command,
    );
    let mut command = seed.command;
    command.anchor = Some(MeasureDecisionAnchorRef::Initial {
        hearing_id: hearing.snapshot.id,
        revision: hearing.snapshot.revision,
        values_digest: hearing.snapshot.values_digest,
        submission_digest: hearing.snapshot.receipt.submission_digest,
    });
    let created = crate::measure_fixture::persist(db, seed.actor, command);
    assert_eq!(created.group.measures.len(), 2);
    db.case
}
