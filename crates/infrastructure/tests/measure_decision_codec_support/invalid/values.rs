use super::*;

#[test]
fn every_value_and_effect_object_requires_exact_keys() {
    stored_decision(unknown_decision()).exact_keys(&["", "/declared_at", "/support"]);
    stored_measure(unknown_measure()).exact_keys(&[
        "",
        "/subject",
        "/validity",
        "/validity/start",
        "/supervision",
    ]);
    stored_measure(known_measure()).exact_keys(&[
        "/validity/start",
        "/validity/end",
        "/supervision",
        "/supervision/participant",
    ]);
    stored_outcome(mixed()).exact_keys(&[
        "",
        "/effects/0",
        "/effects/0/proposal",
        "/effects/0/proposal/values",
        "/effects/1",
        "/effects/1/previous",
        "/effects/2",
        "/effects/2/values",
        "/effects/3",
        "/effects/3/previous",
        "/effects/4",
        "/effects/4/previous",
        "/effects/5",
        "/effects/5/predecessors/0",
        "/effects/5/successors/0",
    ]);
    stored_outcome(
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::NoMeasureChange(note(
            "Unchanged",
        )))
        .unwrap(),
    )
    .exact_keys(&[""]);
}

#[test]
fn persisted_notes_cannot_be_trimmed_normalized_or_truncated() {
    for (stored, paths) in [
        (
            stored_decision(unknown_decision()),
            vec![
                "/authority",
                "/justification",
                "/locator",
                "/declared_at/reason",
            ],
        ),
        (
            stored_measure(unknown_measure()),
            vec![
                "/conditions",
                "/validity/statement",
                "/validity/start/reason",
                "/supervision/reason",
            ],
        ),
        (
            stored_measure(known_measure()),
            vec!["/supervision/statement"],
        ),
    ] {
        for path in paths {
            stored.rejects(|view| {
                *view.pointer_mut(path).unwrap() = json!(format!(
                    " {} ",
                    view.pointer(path).unwrap().as_str().unwrap()
                ));
            });
            for bad in [
                "".to_owned(),
                "A\tB".into(),
                "x".repeat(1001),
                "x".repeat(4001),
            ] {
                stored.rejects(|view| *view.pointer_mut(path).unwrap() = json!(bad));
            }
        }
    }
    let mut input = decision_input();
    input.authority = note("First\nSecond");
    stored_decision(MeasureDecisionValues::new(input))
        .rejects(|view| view["authority"] = json!("First\r\nSecond"));
}

#[test]
fn identifier_and_digest_spellings_must_be_canonical() {
    let mut input = measure_input();
    input.subject.id = CaseSubjectId::from_uuid(Uuid::from_u128(0xab));
    input.subject.values_digest = Sha256Digest::from_array([0xab; 32]);
    let stored = stored_measure(MeasureValues::new(input));
    for path in ["/subject/id", "/subject/digest"] {
        stored.rejects(|view| {
            *view.pointer_mut(path).unwrap() =
                json!(view.pointer(path).unwrap().as_str().unwrap().to_uppercase());
        });
    }
    for (stored, paths) in [
        (
            stored_decision(unknown_decision()),
            vec!["/support/document_id"],
        ),
        (
            stored_measure(known_measure()),
            vec!["/subject/id", "/supervision/participant/id"],
        ),
        (
            stored_outcome(mixed()),
            vec![
                "/effects/0/proposal/id",
                "/effects/1/previous/id",
                "/effects/5/successors/0/id",
            ],
        ),
    ] {
        for path in paths {
            stored.rejects(|view| {
                *view.pointer_mut(path).unwrap() = json!(view
                    .pointer(path)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .replace('-', ""));
            });
        }
    }
    for bad in ["ab".repeat(31), "ab".repeat(33), "gg".repeat(32)] {
        stored.rejects(|view| view["subject"]["digest"] = json!(bad));
    }
}

#[test]
fn positive_revision_fields_and_discriminators_reject_aliases_and_wrong_types() {
    for (stored, paths) in [
        (
            stored_decision(unknown_decision()),
            vec!["/support/version"],
        ),
        (
            stored_measure(known_measure()),
            vec!["/subject/revision", "/supervision/participant/revision"],
        ),
        (
            stored_outcome(mixed()),
            vec![
                "/effects/1/previous/revision",
                "/effects/5/predecessors/0/revision",
            ],
        ),
    ] {
        for path in paths {
            for bad in [
                json!(0),
                json!(-1),
                json!(4_294_967_296u64),
                json!(1.5),
                json!("1"),
                Value::Null,
            ] {
                stored.rejects(|view| *view.pointer_mut(path).unwrap() = bad);
            }
        }
    }
    stored_measure(unknown_measure()).rejects(|view| view["kind"] = json!("PeriodicAppearance"));
    stored_measure(unknown_measure())
        .rejects(|view| view["supervision"]["kind"] = json!("Unknown"));
    stored_outcome(mixed())
        .rejects(|view| view["effects"][0]["action"] = json!("administrative_correction"));
    stored_outcome(mixed()).rejects(|view| view["kind"] = json!("Changes"));
}
