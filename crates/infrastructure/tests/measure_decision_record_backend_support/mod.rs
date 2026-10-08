#![allow(dead_code, unused_imports)]

pub use crate::administrative_fixture::{
    corrected_reference, effect_command, MeasureAdministrativeStoredOperation,
};
pub use crate::measure_fixture::{
    note, processor, FixedClock, Fixture, FormatCheck, Principal, TestIdentity,
};
pub use application::measure_corrections::OwnedMeasureRecord;
pub use application::precautionary_measures::*;
pub use domain::precautionary_hearings::PrecautionaryMeasureRef;
pub use domain::precautionary_measures::*;
pub use infrastructure::RingSha256Hasher;
pub use std::sync::Arc;

mod sql_negative;
mod supplemental;

pub struct RecordSeed {
    pub actor: Principal,
    pub judicial: MeasureDecisionStoredOperation,
    pub corrected: MeasureAdministrativeStoredOperation,
    pub command: MeasureDecisionCommand,
}

pub fn setup(db: &mut Fixture) -> RecordSeed {
    let (seed, judicial, correction) = crate::administrative_fixture::setup(db);
    let corrected = crate::administrative_fixture::persist(db, seed.actor.clone(), correction);
    let command = effect_command(
        &seed.command,
        vec![MeasureEffect::Confirm {
            previous: corrected_reference(&corrected.capture),
        }],
    );
    RecordSeed {
        actor: seed.actor,
        judicial,
        corrected,
        command,
    }
}

pub fn service(db: &Fixture, actor: Principal) -> MeasureDecisionRecordService {
    service_with_format(db, actor, FormatCheck(None))
}

pub fn service_with_format(
    db: &Fixture,
    actor: Principal,
    format: FormatCheck,
) -> MeasureDecisionRecordService {
    MeasureDecisionRecordService::new(
        crate::measure_fixture::store(db),
        Arc::new(TestIdentity(actor)),
        processor(),
        Arc::new(format),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}

pub fn confirmation(review: &MeasureDecisionReviewV2) -> MeasureDecisionConfirmation {
    MeasureDecisionConfirmation {
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
    }
}

pub fn persist(
    db: &Fixture,
    actor: Principal,
    command: MeasureDecisionCommand,
) -> MeasureDecisionRecordStoredOperation {
    let workflow = service(db, actor);
    let MeasureDecisionRecordReview::V2(review) = workflow
        .prepare("session", db.case, command.clone())
        .unwrap()
    else {
        panic!("new record command must prepare a V2 review")
    };
    let MeasureDecisionRecordReceipt::V2(stored) = workflow
        .submit("session", db.case, command, confirmation(&review))
        .unwrap()
    else {
        panic!("new record command must return a V2 receipt")
    };
    *stored
}

pub fn reference(capture: &MeasureCaptureV2) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        capture.result.id,
        capture.result.revision,
        capture.capture_digest,
    )
}

mod reads;
