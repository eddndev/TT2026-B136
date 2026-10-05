use super::*;
use crate::correction_support::{
    note, CorrectionFixture, MeasureCorrectionValues, MeasureTime, MeasureValidity, MeasureValues,
};
use domain::{judicial_calendars::CivilDate, procedural_time::DeclaredProceduralTime};

fn with_validity(validity: MeasureValidity) -> crate::record_support::RecordFixture {
    let mut original = crate::measure_decision_fixtures::Fixture::single();
    let values = original.command.outcome.changes().unwrap()[0].clone();
    let crate::correction_support::MeasureEffect::Impose(proposal) = values else {
        panic!("expected imposition")
    };
    let mut input = crate::effect_support::values_input(&proposal.values);
    input.validity = validity;
    original.replace_values(proposal.id, MeasureValues::new(input));
    let group = original.capture();
    crate::record_support::RecordFixture::from_first(CorrectionFixture::from_group(
        &group,
        &crate::effect_support::empty_history(),
        proposal.id,
    ))
}
fn unknown(text: &str) -> MeasureTime {
    MeasureTime::new(DeclaredProceduralTime::unknown(), Some(note(text))).unwrap()
}
fn date() -> CivilDate {
    CivilDate::from_date(time::Date::from_calendar_date(2028, time::Month::February, 29).unwrap())
        .unwrap()
}

#[tokio::test]
async fn known_precision_retains_explicit_missing_offsets_and_never_adds_time_components() {
    let offset = Some(time::UtcOffset::from_whole_seconds(-21600).unwrap());
    let declarations = [
        DeclaredProceduralTime::date(date(), None).unwrap(),
        DeclaredProceduralTime::minute(date(), 8, 9, offset).unwrap(),
        DeclaredProceduralTime::second(date(), 8, 9, 10, offset).unwrap(),
    ];
    for declared in declarations {
        let start = MeasureTime::new(declared, None).unwrap();
        let fixture =
            with_validity(MeasureValidity::new(start, note("Declared terms"), None).unwrap());
        let operation = stored(&fixture);
        let review = operation.capture.review;
        let input = command_json(&review);
        let expected_time = input["action"]["values"]["validity"]["start"].clone();
        let mut write = MockWrite::new();
        write
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, command| {
                assert_eq!(command, review.command);
                Ok(review)
            });
        let (status, body) =
            request(write, MockRead::new(), "POST", &prepare_path(), Some(input)).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(
            body["command"]["action"]["values"]["validity"]["start"],
            expected_time
        );
        assert!(body["command"]["action"]["values"]["validity"]["end"].is_null());
    }
}

#[tokio::test]
async fn maximum_six_unicode_notes_fit_when_all_scalars_are_escaped_surrogate_pairs() {
    let text = "\u{1f642}".repeat(1000);
    let validity = MeasureValidity::new(unknown(&text), note(&text), Some(unknown(&text))).unwrap();
    let mut fixture = with_validity(validity.clone());
    fixture.command.reason = note(&text);
    fixture.command.action = MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
        note(&text),
        validity,
        note(&text),
    ));
    let operation = stored(&fixture);
    let encoded = submit_json(&operation)
        .to_string()
        .replace('\u{1f642}', "\\ud83d\\ude42");
    assert!(encoded.is_ascii());
    assert_eq!(encoded.matches("\\ud83d\\ude42").count(), 6000);
    assert!(encoded.len() > 72000 && encoded.len() < 128 * 1024);
    let mut write = MockWrite::new();
    let expected = operation.clone();
    write
        .expect_submit()
        .times(1)
        .return_once(move |_, _, command, confirmation| {
            assert_eq!(command, expected.capture.review.command);
            assert_eq!(confirmation, confirm(&expected));
            Ok(expected)
        });
    let (status, body) = raw(
        (write, MockRead::new(), MockRecords::new()),
        "POST",
        &submit_path(),
        "staff-token",
        encoded,
        "application/json",
    )
    .await;
    assert_eq!(status, 201, "{body}");
    assert_stored(&body, &operation);
    assert_eq!(body["capture"]["review"]["command"]["reason"], text);
}

#[tokio::test]
async fn invalid_or_noncanonical_declared_time_never_reaches_the_port() {
    let original = command_json(&correct().capture.review);
    let valid = json!({"precision":"second","year":2028,"month":2,"day":29,"hour":8,"minute":9,"second":10,"offset_seconds":-21600});
    for fault in 0..18 {
        let mut body = original.clone();
        body["action"]["values"]["validity"]["start"] = valid.clone();
        let start = &mut body["action"]["values"]["validity"]["start"];
        match fault {
            0 => {
                start.as_object_mut().unwrap().remove("offset_seconds");
            }
            1 => start["year"] = json!(0),
            2 => start["year"] = json!(10000),
            3 => start["year"] = json!(2027),
            4 => start["hour"] = json!(24),
            5 => start["minute"] = json!(60),
            6 => start["second"] = json!(60),
            7 => start["offset_seconds"] = json!(1),
            8 => start["offset_seconds"] = json!(50460),
            9 => start["reason"] = json!("Not an unknown time"),
            10 => start["precision"] = json!("millisecond"),
            11 => start["second"] = json!(10.0),
            12 => start["offset_seconds"] = json!("-21600"),
            13 => *start = json!({"precision":"unknown"}),
            14 => *start = json!({"precision":"unknown","reason":"Missing","offset_seconds":null}),
            15 => {
                *start =
                    json!({"precision":"date","year":1,"month":1,"day":1,"offset_seconds":50400})
            }
            16 => {
                body["action"]["values"]["validity"]
                    .as_object_mut()
                    .unwrap()
                    .remove("end");
            }
            _ => {
                body["action"]["values"]["validity"]["end"] = json!({"precision":"second","year":2028,"month":2,"day":28,"hour":8,"minute":9,"second":10,"offset_seconds":-21600})
            }
        }
        let (status, response) = request(
            MockWrite::new(),
            MockRead::new(),
            "POST",
            &prepare_path(),
            Some(body),
        )
        .await;
        assert_eq!(status, 400, "{fault}: {response}");
    }
}
