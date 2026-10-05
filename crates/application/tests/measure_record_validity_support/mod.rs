pub use crate::record_support::*;
use application::ApplicationError;

pub fn marking(mut fixture: RecordFixture) -> RecordFixture {
    fixture.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    fixture.command.reason = note("The source was associated with the wrong subject");
    fixture
}

pub fn prepare(
    fixture: &RecordFixture,
) -> Result<CheckedMeasureAdministrativeReview, ApplicationError> {
    prepare_measure_administrative_record_with_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command.clone(),
        fixture.context.clone(),
        &fixture.history,
    )
}

pub fn capture(fixture: &RecordFixture) -> MeasureAdministrativeCapture {
    prepare(fixture)
        .unwrap()
        .into_capture(&Hasher, fixture.recorded_at)
        .unwrap()
}
