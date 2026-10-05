pub use crate::hearing_fixture::{confirmation, Fixture, FormatCheck};
use crate::measure_fixture::{processor, FixedClock, TestIdentity};
pub use application::{
    identity::Principal, measure_corrections::MeasureAdministrativeStoredOperation,
    precautionary_hearings::*, precautionary_measures::MeasureDecisionStoredOperation,
};
use domain::crypto::DocumentVersionRef;
use domain::hearings::{
    HearingModality, HearingNote, HearingSupportRef, HearingTime, HearingVenue,
};
pub use domain::precautionary_hearings::*;
pub use infrastructure::{PostgresPrecautionaryHearingStore, RingSha256Hasher};
use std::sync::Arc;
use time::Duration;

pub struct Seed {
    pub actor: Principal,
    pub judicial: MeasureDecisionStoredOperation,
    pub corrected: MeasureAdministrativeStoredOperation,
    pub command: PrecautionaryHearingCommand,
}

pub fn store(db: &Fixture) -> Arc<PostgresPrecautionaryHearingStore> {
    Arc::new(
        PostgresPrecautionaryHearingStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}

pub fn service_with_format(
    db: &Fixture,
    actor: Principal,
    format: FormatCheck,
) -> PrecautionaryHearingRecordService {
    PrecautionaryHearingRecordService::new(
        store(db),
        Arc::new(TestIdentity(actor)),
        processor(),
        Arc::new(format),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

pub fn service(db: &Fixture, actor: Principal) -> PrecautionaryHearingRecordService {
    service_with_format(db, actor, FormatCheck(None))
}

pub fn setup(db: &mut Fixture) -> Seed {
    let (seed, judicial, correction) = crate::administrative_fixture::setup(db);
    let corrected = crate::administrative_fixture::persist(db, seed.actor.clone(), correction);
    let values = PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
        purpose: PrecautionaryHearingPurpose::Review,
        scheduled_at: HearingTime::new((db.at + Duration::days(2)).replace_nanosecond(0).unwrap())
            .unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Court for the corrected measure review").unwrap(),
        note: None,
        participants: vec![],
        scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
            HearingNote::new("Declared appointment reviewing the corrected capture").unwrap(),
            HearingSupportRef::new(
                DocumentVersionRef {
                    id: seed.record.id,
                    version: seed.record.version,
                },
                seed.record.digest,
            ),
            HearingNote::new("Page 1").unwrap(),
        ),
        review_targets: vec![crate::administrative_fixture::corrected_reference(
            &corrected.capture,
        )],
    })
    .unwrap();
    Seed {
        actor: seed.actor,
        judicial,
        corrected,
        command: PrecautionaryHearingCommand {
            operation_id: PrecautionaryHearingOperationId::new(),
            hearing_id: PrecautionaryHearingId::new(),
            change: PrecautionaryHearingChange::Schedule {
                context: seed.command.context,
                values,
            },
        },
    }
}

pub fn persist(
    db: &Fixture,
    actor: Principal,
    command: PrecautionaryHearingCommand,
) -> PrecautionaryHearingRecordStoredOperation {
    let workflow = service(db, actor);
    let review = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    workflow
        .submit("session", db.case, command, confirmation(&review))
        .unwrap()
}

pub fn same_operation(
    actual: &PrecautionaryHearingRecordStoredOperation,
    expected: &PrecautionaryHearingRecordStoredOperation,
) {
    let mut actual = actual.clone();
    let mut expected = expected.clone();
    for result in [&mut actual, &mut expected] {
        let history = &mut result.history.record_history;
        history
            .records
            .judicial
            .groups
            .sort_by_key(|g| g.origin.operation_id.as_uuid());
        history
            .records
            .administrative
            .sort_by_key(|a| a.origin.operation_id.as_uuid());
        history
            .decisions
            .sort_by_key(|g| g.origin.operation_id.as_uuid());
    }
    assert_eq!(actual, expected);
}

pub fn reopened(
    db: &Fixture,
    actor: &Principal,
    expected: &PrecautionaryHearingRecordStoredOperation,
) {
    let bytes = precautionary_hearing_capture_bytes(&expected.capture).unwrap();
    let workflow = service_with_format(
        db,
        actor.clone(),
        FormatCheck(Some(Box::new(|| {
            panic!("exact mixed hearing replay must skip document admission")
        }))),
    );
    let command = expected.capture.review.command.clone();
    assert_eq!(
        workflow
            .prepare("session", db.case, command.clone())
            .unwrap(),
        expected.capture.review
    );
    let result = workflow
        .submit(
            "session",
            db.case,
            command,
            confirmation(&expected.capture.review),
        )
        .unwrap();
    same_operation(&result, expected);
    assert_eq!(
        precautionary_hearing_capture_bytes(&result.capture).unwrap(),
        bytes
    );
}
