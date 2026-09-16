mod judicial_calendar_support;
use domain::judicial_calendars::*;
use serde_json::Value;
use uuid::Uuid;
fn string(value: &Value) -> &str {
    value.as_str().unwrap()
}
fn date(value: &Value) -> CivilDate {
    string(value).parse().unwrap()
}
fn uuid(value: &Value) -> Uuid {
    string(value).parse().unwrap()
}
fn rule(value: &Value) -> JudicialCalendarRule {
    JudicialCalendarRule::new(
        string(&value["classification"]).parse().unwrap(),
        value["source_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(uuid)
            .collect(),
        string(&value["explanation"]),
    )
    .unwrap()
}
fn values(value: &Value) -> JudicialCalendarValues {
    let scope = &value["scope"];
    let codes: Vec<_> = scope["entity_codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(string)
        .collect();
    let scope = JudicialCalendarScope::new(JudicialCalendarScopeInput {
        title: string(&scope["title"]),
        jurisdiction: string(&scope["jurisdiction"]).parse().unwrap(),
        entity_codes: &codes,
        authority: string(&scope["authority"]),
        organ: string(&scope["organ"]),
        territory: string(&scope["territory"]),
        use_description: string(&scope["use_description"]),
    })
    .unwrap();
    let coverage = JudicialCalendarCoverage::new(
        date(&value["coverage"]["from"]),
        date(&value["coverage"]["through"]),
    )
    .unwrap();
    let sources = value["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            JudicialCalendarSource::new(JudicialCalendarSourceInput {
                id: uuid(&s["id"]),
                title: string(&s["title"]),
                issuer: string(&s["issuer"]),
                official_url: string(&s["official_url"]),
                published_on: (!s["published_on"].is_null()).then(|| date(&s["published_on"])),
                consulted_on: date(&s["consulted_on"]),
                locator: string(&s["locator"]),
            })
            .unwrap()
        })
        .collect();
    let weekly = value["weekly_pattern"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| {
            JudicialCalendarWeekdayRule::new(w["weekday"].as_u64().unwrap() as u8, rule(w)).unwrap()
        })
        .collect();
    let exceptions = value["exceptions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            JudicialCalendarException::new(
                uuid(&e["id"]),
                date(&e["from"]),
                date(&e["through"]),
                rule(e),
            )
            .unwrap()
        })
        .collect();
    JudicialCalendarValues::new(scope, coverage, sources, weekly, exceptions).unwrap()
}
fn vectors() -> Vec<Value> {
    serde_json::from_str(include_str!("fixtures/judicial_calendar_vectors.json")).unwrap()
}
fn bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn independent_vectors_bind_normalization_civil_days_and_maximum_size() {
    for vector in vectors() {
        let value = values(&vector["input"]);
        let expected = bytes(string(&vector["hex"]));
        assert_eq!(value.canonical_bytes(), expected, "{}", vector["name"]);
        assert_eq!(value, values(&vector["normalized"]));
        assert_eq!(
            JudicialCalendarValues::from_canonical_bytes(&expected).unwrap(),
            value
        );
        assert_eq!(expected.len(), vector["bytes"].as_u64().unwrap() as usize);
        for sample in vector["classifications"].as_array().unwrap() {
            let actual = value.classify(date(&sample["date"]));
            assert_eq!(
                actual
                    .classification()
                    .map(|c| c.as_str())
                    .unwrap_or("outside_coverage"),
                string(&sample["state"])
            );
            let origin = match string(&sample["state"]) {
                "outside_coverage" => None,
                _ if sample["origin"] == "weekly_pattern" => {
                    Some(JudicialCalendarDayOrigin::WeeklyPattern(
                        sample["weekday"].as_u64().unwrap() as u8,
                    ))
                }
                _ => Some(JudicialCalendarDayOrigin::Exception(uuid(
                    &sample["exception_id"],
                ))),
            };
            assert_eq!(actual.origin(), origin);
            assert_eq!(actual.explanation(), sample["explanation"].as_str());
            assert_eq!(
                actual.source_ids(),
                sample["source_ids"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(uuid)
                    .collect::<Vec<_>>()
            );
        }
    }
    assert_eq!(MIN_JUDICIAL_CALENDAR_CANONICAL_BYTES, 99);
    assert_eq!(MAX_JUDICIAL_CALENDAR_CANONICAL_BYTES, 191910);
}
#[test]
fn canonical_reader_rejects_truncation_trailing_data_bad_tags_and_noncanonical_order() {
    let original = bytes(string(&vectors()[0]["hex"]));
    for length in 0..original.len() {
        assert!(JudicialCalendarValues::from_canonical_bytes(&original[..length]).is_err());
    }
    for (offset, value) in [
        (0, b'X'),
        (10, 2),
        (11, 0),
        (11, 33),
        (12, 0),
        (12, 33),
        (41, 17),
        (42, 0),
        (43, 3),
        (44, 17),
        (50, 1),
        (98, 65),
    ] {
        let mut malformed = original.clone();
        malformed[offset] = value;
        assert!(
            JudicialCalendarValues::from_canonical_bytes(&malformed).is_err(),
            "offset {offset}"
        );
    }
    let mut trailing = original.clone();
    trailing.push(0);
    assert!(JudicialCalendarValues::from_canonical_bytes(&trailing).is_err());
    let mut date_overflow = original.clone();
    date_overflow[33..37].copy_from_slice(&i32::MAX.to_be_bytes());
    assert!(JudicialCalendarValues::from_canonical_bytes(&date_overflow).is_err());
    let mut length_overflow = original.clone();
    length_overflow[5..9].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(JudicialCalendarValues::from_canonical_bytes(&length_overflow).is_err());
    let mut invalid_utf8 = original.clone();
    invalid_utf8[9] = 0xff;
    assert!(JudicialCalendarValues::from_canonical_bytes(&invalid_utf8).is_err());
    let mut weekly_order = original.clone();
    weekly_order[42] = 2;
    weekly_order[50] = 1;
    assert!(JudicialCalendarValues::from_canonical_bytes(&weekly_order).is_err());
    assert!(JudicialCalendarValues::from_canonical_bytes(&vec![
        0;
        MAX_JUDICIAL_CALENDAR_CANONICAL_BYTES
            + 1
    ])
    .is_err());
}
