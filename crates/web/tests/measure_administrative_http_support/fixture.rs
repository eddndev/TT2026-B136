use super::*;
use crate::{correction_support::CorrectionFixture, record_support::RecordFixture};
use application::precautionary_measures::MeasureDecisionRecordHistoryEvidence;
use domain::{precautionary_measures::MeasureTime, procedural_time::DeclaredProceduralPrecision};

pub fn stored(fixture: &RecordFixture) -> MeasureAdministrativeStoredOperation {
    let history = MeasureDecisionRecordHistoryEvidence {
        records: fixture.history.clone(),
        decisions: vec![],
    };
    let capture = prepare_measure_administrative_record_with_decision_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command.clone(),
        fixture.context.clone(),
        &history,
    )
    .unwrap()
    .into_capture(&Hasher, fixture.recorded_at)
    .unwrap();
    operation(capture, history)
}
pub fn operation(
    capture: MeasureAdministrativeCapture,
    record_history: MeasureDecisionRecordHistoryEvidence,
) -> MeasureAdministrativeStoredOperation {
    let origin =
        measure_administrative_origin_with_decision_history(&Hasher, &capture, &record_history)
            .unwrap();
    MeasureAdministrativeStoredOperation {
        capture,
        origin,
        record_history,
    }
}
pub fn correct() -> MeasureAdministrativeStoredOperation {
    stored(&RecordFixture::from_first(CorrectionFixture::initial()))
}
pub fn confirm(value: &MeasureAdministrativeStoredOperation) -> MeasureAdministrativeConfirmation {
    MeasureAdministrativeConfirmation {
        submission_digest: value.capture.review.submission_digest,
        review_digest: value.capture.review.review_digest,
    }
}
pub fn reference(value: PrecautionaryMeasureRef) -> Value {
    json!({"id":value.id().to_string(),"revision":value.revision().get(),"capture_digest":value.digest().to_hex()})
}
pub fn command_json(value: &MeasureAdministrativeReview) -> Value {
    let command = &value.command;
    let action = match &command.action {
        MeasureAdministrativeAction::Correct(values) => json!({"kind":"correct","values":{
            "conditions":values.conditions().as_str(),"validity":{"start":time_json(values.validity().start()),
            "statement":values.validity().statement().as_str(),"end":values.validity().end().map(time_json)},
            "supervision_text":values.supervision_text().as_str()}}),
        MeasureAdministrativeAction::MarkEnteredInError => json!({"kind":"entered_in_error"}),
        MeasureAdministrativeAction::MarkEnteredInErrorAndReplace {
            replacement_id,
            subject,
        } => json!({
            "kind":"replace_entered_in_error","replacement_id":replacement_id.to_string(),"subject":{
                "id":subject.id.to_string(),"revision":subject.revision.get(),"values_digest":subject.values_digest.to_hex()}}),
    };
    json!({"case_id":value.case_id.to_string(),"operation_id":command.operation_id.to_string(),"target":reference(command.target),
        "context":{"administration_revision":command.context.administration_revision.get(),"stage_revision":command.context.stage_revision.get(),"context_digest":command.context.context_digest.to_hex()},
        "reason":command.reason.as_str(),"action":action})
}
pub fn time_json(value: &MeasureTime) -> Value {
    let time = value.declared();
    let precision = match time.precision() {
        DeclaredProceduralPrecision::Unknown => {
            return json!({"precision":"unknown","reason":value.unknown_reason().unwrap().as_str()})
        }
        DeclaredProceduralPrecision::Date => "date",
        DeclaredProceduralPrecision::Minute => "minute",
        DeclaredProceduralPrecision::Second => "second",
    };
    let date = time.local_date().unwrap().date();
    let mut value = json!({"precision":precision,"year":date.year(),"month":date.month() as u8,"day":date.day(),"offset_seconds":time.offset().map(|v|v.whole_seconds())});
    if let Some(hour) = time.local_hour() {
        value["hour"] = json!(hour);
    }
    if let Some(minute) = time.local_minute() {
        value["minute"] = json!(minute);
    }
    if let Some(second) = time.local_second() {
        value["second"] = json!(second);
    }
    value
}
pub fn submit_json(value: &MeasureAdministrativeStoredOperation) -> Value {
    json!({"command":command_json(&value.capture.review),"expected_submission_digest":value.capture.review.submission_digest.to_hex(),"expected_review_digest":value.capture.review.review_digest.to_hex()})
}
pub fn assert_stored(body: &Value, value: &MeasureAdministrativeStoredOperation) {
    assert_eq!(body["capture"]["family"], "a1");
    assert_eq!(
        body["capture"]["capture_digest"],
        value.capture.capture_digest.to_hex()
    );
    assert_eq!(
        body["capture"]["review"]["command"],
        command_json(&value.capture.review)
    );
    assert_eq!(
        body["origin"],
        json!({"case_id":value.origin.case_id.to_string(),"operation_id":value.origin.operation_id.to_string(),
        "submission_digest":value.origin.submission_digest.to_hex(),"review_digest":value.origin.review_digest.to_hex(),"capture_digest":value.origin.capture_digest.to_hex()})
    );
    assert_eq!(
        body["capture"]["records"].as_array().unwrap().len(),
        value.capture.records.len()
    );
    assert_eq!(
        body["record_history"]["records"]["judicial"]["groups"]
            .as_array()
            .unwrap()
            .len(),
        value.record_history.records.judicial.groups.len()
    );
    assert_eq!(
        body["record_history"]["records"]["administrative"]
            .as_array()
            .unwrap()
            .len(),
        value.record_history.records.administrative.len()
    );
    assert_eq!(
        body["record_history"]["decisions"]
            .as_array()
            .unwrap()
            .len(),
        value.record_history.decisions.len()
    );
}
pub fn marked() -> MeasureAdministrativeStoredOperation {
    let mut fixture = RecordFixture::initial();
    fixture.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    stored(&fixture)
}
pub fn replacement() -> MeasureAdministrativeStoredOperation {
    let fixture = crate::replacement_support::ReplacementFixture::initial();
    operation(fixture.capture(), fixture.history)
}
pub fn after_replacement_g2() -> MeasureAdministrativeStoredOperation {
    use crate::correction_support::{note, MeasureCorrectionValues};
    let prior = replacement();
    let next = crate::record_decision_support::FixtureV2::confirm(
        &prior.capture,
        &prior.record_history.records,
    );
    let group = next.capture();
    let history = crate::record_decision_support::append_v2(&next.history, &group);
    let member = &group.measures[0];
    let mut command = prior.capture.review.command.clone();
    command.operation_id = MeasureCorrectionOperationId::from_uuid(uuid::Uuid::from_u128(800));
    command.target = crate::record_decision_support::reference_v2(member);
    command.action = MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
        note("Correct conditions following the actual later decision"),
        member.result.values.validity().clone(),
        note("Retained supervision clarified"),
    ));
    let capture = prepare_measure_administrative_record_with_decision_history(
        &Hasher,
        &next.actor,
        next.case_id,
        command,
        next.material.context.clone(),
        &history,
    )
    .unwrap()
    .into_capture(&Hasher, group.recorded_at + time::Duration::seconds(1))
    .unwrap();
    operation(capture, history)
}
pub fn utc(value: time::OffsetDateTime) -> String {
    value
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}
pub async fn get_json(value: MeasureAdministrativeStoredOperation) -> (u16, Value) {
    let path = operation_path(&value);
    let mut read = MockRead::new();
    read.expect_get_operation()
        .times(1)
        .return_once(move |_, _, _| Ok(value));
    request(MockWrite::new(), read, "GET", &path, None).await
}
