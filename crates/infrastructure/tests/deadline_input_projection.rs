mod case_administration_support;
mod deadline_input_projection_support;
use case_administration_support::Fixture;
use deadline_input_projection_support::*;
use serde_json::json;
use uuid::Uuid;

#[test]
fn projects_unknown_and_each_exact_family_including_nil_agreement_and_calendar() {
    let Some(mut db) = Fixture::new() else { return };
    let mut input = Input::default();
    assert_eq!(input.bytes().len(), 48);
    valid(&mut db, &input, expected(input.case));
    let id = Uuid::from_u128(7);
    let mut value = expected(input.case);
    input.source = known(0, &reference(id, u32::MAX));
    value["source_kind"] = json!("resolution");
    value["source_id"] = json!(id);
    value["source_revision"] = json!(u32::MAX);
    valid(&mut db, &input, value);
    let mut reference = reference(id, 2);
    reference.extend(deadline_input_projection_support::reference(Uuid::nil(), 3));
    input.source = known(1, &reference);
    let mut value = expected(input.case);
    value["source_kind"] = json!("notification");
    value["source_id"] = json!(id);
    value["source_revision"] = json!(2);
    value["source_parent_resolution_id"] = json!(Uuid::nil());
    value["source_parent_resolution_revision"] = json!(3);
    valid(&mut db, &input, value);
    for agreement in [false, true] {
        let mut reference = id.as_bytes().to_vec();
        reference.extend(deadline_input_projection_support::reference(Uuid::nil(), 1));
        reference.push(u8::from(agreement));
        if agreement {
            reference.extend(Uuid::nil().as_bytes());
        }
        input.source = known(2, &reference);
        input.calendar = vec![1];
        input
            .calendar
            .extend(deadline_input_projection_support::reference(
                Uuid::nil(),
                u32::MAX,
            ));
        let mut value = expected(input.case);
        value["source_kind"] = json!("hearing_result");
        value["source_id"] = json!(Uuid::nil());
        value["source_revision"] = json!(1);
        value["source_hearing_id"] = json!(id);
        if agreement {
            value["source_agreement_id"] = json!(Uuid::nil());
        }
        value["calendar_id"] = json!(Uuid::nil());
        value["calendar_revision"] = json!(u32::MAX);
        valid(&mut db, &input, value);
    }
}

#[test]
fn validates_discarded_qualification_time_and_tri_state_conditions_without_semantic_inference() {
    let Some(mut db) = Fixture::new() else { return };
    for time in [
        vec![0],
        vec![1, 7, 234, 1, 6, 0],
        vec![2, 7, 234, 1, 6, 12, 34, 0],
        vec![3, 7, 234, 1, 6, 12, 34, 0, 1, 0, 0, 0, 0],
    ] {
        let mut input = Input {
            qualification: qualification(&time),
            ordered: vec![1],
            ..Input::default()
        };
        input.ordered.extend(u32::MAX.to_be_bytes());
        input.scope = unknown("Scope is not established");
        input.incident = vec![1, 1];
        input.conditions = vec![(
            Uuid::nil(),
            unknown("Undetermined"),
            "Condition locator".into(),
        )];
        valid(&mut db, &input, expected(input.case));
    }
}

#[test]
fn accepts_the_independently_constructed_maximum_and_rejects_larger_or_duplicate_conditions() {
    let Some(mut db) = Fixture::new() else { return };
    let large = "\u{1f600}".repeat(1000);
    let label = "\u{1f600}".repeat(200);
    let mut input = Input {
        source: unknown(&large),
        statement: large.clone(),
        locator: label.clone(),
        scope: unknown(&large),
        incident: unknown(&large),
        ..Input::default()
    };
    input.qualification = vec![1, 1, 3, 7, 234, 1, 6, 12, 34, 56, 1];
    input.qualification.extend((-50_400_i32).to_be_bytes());
    input.qualification.extend(text(&large));
    input.qualification.extend(text(&label));
    input.calendar = vec![1];
    input.calendar.extend(reference(Uuid::nil(), u32::MAX));
    input.ordered = vec![1];
    input.ordered.extend(u32::MAX.to_be_bytes());
    input.conditions = (0..16)
        .map(|id| (Uuid::from_u128(id), unknown(&large), label.clone()))
        .collect();
    assert_eq!(input.bytes().len(), 98_897);
    let mut value = expected(input.case);
    value["calendar_id"] = json!(Uuid::nil());
    value["calendar_revision"] = json!(u32::MAX);
    valid(&mut db, &input, value);
    let mut oversized = input.bytes();
    oversized.push(0);
    rejected(&mut db, &oversized);
    input.conditions[1].0 = input.conditions[0].0;
    rejected(&mut db, &input.bytes());
    let too_many = Input {
        conditions: (0..17)
            .map(|id| (Uuid::from_u128(id), vec![1, 1], "x".into()))
            .collect(),
        ..Input::default()
    };
    rejected(&mut db, &too_many.bytes());
}

#[test]
fn rejects_invalid_prefix_trailing_bytes_lengths_and_every_truncated_prefix() {
    let Some(mut db) = Fixture::new() else { return };
    let input = Input {
        source: known(0, &reference(Uuid::nil(), 1)),
        qualification: qualification(&[3, 7, 234, 1, 6, 12, 34, 56, 0]),
        ..Input::default()
    };
    let bytes = input.bytes();
    for end in 0..bytes.len() {
        rejected(&mut db, &bytes[..end]);
    }
    let mut changed = bytes.clone();
    changed[0] = b'X';
    rejected(&mut db, &changed);
    let mut changed = bytes;
    changed.push(0);
    rejected(&mut db, &changed);
    let mut changed = Input::default().bytes();
    changed[22..26].copy_from_slice(&u32::MAX.to_be_bytes());
    rejected(&mut db, &changed);
}

#[test]
fn rejects_invalid_flags_revisions_quantities_text_and_discarded_tail_values() {
    let Some(mut db) = Fixture::new() else { return };
    for mutation in 0..13 {
        let mut input = Input::default();
        match mutation {
            0 => input.source = vec![2],
            1 => input.source = known(3, &reference(Uuid::nil(), 1)),
            2 => input.source = known(0, &reference(Uuid::nil(), 0)),
            3 => input.qualification = vec![2],
            4 => input.qualification = vec![1, 2, 0],
            5 => input.calendar = vec![2],
            6 => {
                input.calendar = vec![1];
                input.calendar.extend(reference(Uuid::nil(), 0));
            }
            7 => input.ordered = vec![1, 0, 0, 0, 0],
            8 => input.ordered = vec![2],
            9 => input.scope = vec![1, 2],
            10 => input.incident = vec![2],
            11 => input.conditions = vec![(Uuid::nil(), vec![1, 2], "x".into())],
            _ => input.locator = "line\nbreak".into(),
        }
        rejected(&mut db, &input.bytes());
    }
    for text in ["", " padded", "x\r\ny", "\u{85}"] {
        let input = Input {
            statement: text.into(),
            ..Input::default()
        };
        rejected(&mut db, &input.bytes());
    }
    let mut invalid_utf8 = Input::default().bytes();
    invalid_utf8[26] = 255;
    rejected(&mut db, &invalid_utf8);
    for time in [
        vec![4],
        vec![1, 0, 0, 1, 6, 0],
        vec![1, 7, 233, 2, 29, 0],
        vec![2, 7, 234, 1, 6, 24, 0, 0],
        vec![3, 7, 234, 1, 6, 12, 34, 60, 0],
        vec![1, 7, 234, 1, 6, 2],
        vec![1, 7, 234, 1, 6, 1, 0, 0, 0, 1],
    ] {
        let input = Input {
            qualification: qualification(&time),
            ..Input::default()
        };
        rejected(&mut db, &input.bytes());
    }
}
