use super::*;
use domain::crypto::DocumentHasher;
use postgres::error::SqlState;

fn reject(
    db: &mut Fixture,
    capture: &MeasureAdministrativeCapture,
    payload: Option<&Value>,
    member: Option<&Value>,
    root: bool,
) {
    let before = snapshot(db);
    assert!(direct::insert(db, capture, payload, member, root).is_err());
    assert_eq!(snapshot(db), before);
}

#[test]
fn direct_administrative_append_requires_exact_payload_shape_and_retained_member_identity() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, command) = setup(&mut db);
    let capture = capture(&db, &seed.actor, &judicial, command);
    let payload = direct::payload(&capture);
    let member = direct::member(&capture);
    let before = snapshot(&mut db);
    direct::insert(&db, &capture, Some(&payload), Some(&member), false)
        .expect("the real checked administrative capture is the direct-append control");
    assert_eq!(snapshot(&mut db), before);
    for field in [
        "missing", "mark", "target", "support", "family", "action", "validity", "subject",
    ] {
        let mut changed_payload = payload.clone();
        let mut changed_member = member.clone();
        match field {
            "missing" => changed_payload["correction_view"] = Value::Null,
            "mark" => changed_payload["action"] = json!("entered_in_error"),
            "target" => {
                changed_payload["target_capture_digest"] = json!(format!("\\x{}", "11".repeat(32)))
            }
            "support" => changed_payload["support_format"] = json!("docx"),
            "family" => changed_member["family"] = json!("m1"),
            "action" => changed_member["action"] = json!("revoke"),
            "validity" => changed_member["validity"] = json!("entered_in_error"),
            "subject" => changed_member["subject_id"] = json!(uuid::Uuid::new_v4()),
            _ => unreachable!(),
        }
        reject(
            &mut db,
            &capture,
            Some(&changed_payload),
            Some(&changed_member),
            false,
        );
    }
}

#[test]
fn direct_administrative_owner_requires_one_payload_one_record_and_no_root() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, command) = setup(&mut db);
    let capture = capture(&db, &seed.actor, &judicial, command);
    let payload = direct::payload(&capture);
    let member = direct::member(&capture);
    direct::insert(&db, &capture, Some(&payload), Some(&member), false).unwrap();

    reject(&mut db, &capture, None, None, false);
    reject(&mut db, &capture, None, Some(&member), false);
    reject(&mut db, &capture, Some(&payload), None, false);
    reject(&mut db, &capture, Some(&payload), Some(&member), true);
}

#[test]
fn direct_administrative_admission_requires_the_current_valid_target() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, command) = setup(&mut db);
    let mut candidate = command.clone();
    candidate.operation_id = MeasureCorrectionOperationId::new();
    let capture = capture(&db, &seed.actor, &judicial, candidate);
    let payload = direct::payload(&capture);
    let member = direct::member(&capture);
    direct::insert(&db, &capture, Some(&payload), Some(&member), false).unwrap();
    let corrected = persist(&db, seed.actor.clone(), command);

    reject(&mut db, &capture, Some(&payload), Some(&member), false);
    reopened(&db, &seed.actor, &corrected);
}

#[test]
fn direct_administrative_capture_cannot_predate_its_actual_target_owner() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = crate::measure_fixture::setup(&mut db);
    let first_time = db.at;
    db.at += time::Duration::seconds(10);
    let judicial = crate::measure_fixture::persist(&db, seed.actor.clone(), seed.command.clone());
    let previous = &judicial.group.measures[0];
    let command = correction(
        reference(previous),
        seed.command.context,
        &previous.result.values,
        "Corrected later record",
    );
    let mut capture = capture(&db, &seed.actor, &judicial, command);
    direct::insert(
        &db,
        &capture,
        Some(&direct::payload(&capture)),
        Some(&direct::member(&capture)),
        false,
    )
    .unwrap();
    capture.recorded_at = first_time + time::Duration::seconds(9);
    capture.records[0].recorded_at = capture.recorded_at;
    capture.records[0].capture_digest = RingSha256Hasher
        .hash_bytes(&measure_administrative_record_bytes(&capture.records[0]).unwrap());
    capture.capture_digest =
        RingSha256Hasher.hash_bytes(&measure_administrative_capture_bytes(&capture).unwrap());
    let history = MeasureHistoryEvidence {
        groups: vec![MeasureGroupEvidence {
            origin: judicial.origin.clone(),
            capture: judicial.group.clone(),
        }],
    };
    assert!(measure_administrative_capture_matches(&RingSha256Hasher, &capture, &history).is_err());
    let before = snapshot(&mut db);

    let error = direct::insert(
        &db,
        &capture,
        Some(&direct::payload(&capture)),
        Some(&direct::member(&capture)),
        false,
    )
    .expect_err("accepted an administrative capture before its target");

    assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn direct_administrative_admission_retains_older_review_dependencies_after_imposition_replacement()
{
    use application::precautionary_hearings::{
        PrecautionaryHearingChange, PrecautionaryHearingCommand,
    };
    use domain::hearings::{HearingModality, HearingTime, HearingVenue};
    use domain::precautionary_hearings::*;
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, command) = setup(&mut db);
    let capture = capture(&db, &seed.actor, &judicial, command);
    let payload = direct::payload(&capture);
    let member = direct::member(&capture);
    direct::insert(&db, &capture, Some(&payload), Some(&member), false).unwrap();
    let hearing = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        PrecautionaryHearingCommand {
            operation_id: PrecautionaryHearingOperationId::new(),
            hearing_id: PrecautionaryHearingId::new(),
            change: PrecautionaryHearingChange::Schedule {
                context: seed.command.context,
                values: PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
                    purpose: PrecautionaryHearingPurpose::Review,
                    scheduled_at: HearingTime::new(
                        (db.at + time::Duration::days(2))
                            .replace_nanosecond(0)
                            .unwrap(),
                    )
                    .unwrap(),
                    modality: HearingModality::InPerson,
                    venue: HearingVenue::new("Court for the declared review").unwrap(),
                    note: None,
                    participants: vec![],
                    scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
                        note("Declared review"),
                        seed.command.values.support(),
                        note("Page 1"),
                    ),
                    review_targets: vec![capture.review.command.target],
                })
                .unwrap(),
            },
        },
    );
    let mut replacement = crate::hearing_fixture::replacement(&hearing);
    let PrecautionaryHearingChange::Replace { values, .. } = &mut replacement.change else {
        unreachable!()
    };
    let mut input = crate::hearing_fixture::input(values);
    input.purpose = PrecautionaryHearingPurpose::Imposition;
    input.review_targets.clear();
    *values = PrecautionaryHearingValues::new(input).unwrap();
    let latest = crate::hearing_fixture::persist(&db, seed.actor, replacement);
    assert!(latest
        .capture
        .review
        .resolved_values
        .review_targets()
        .is_empty());

    reject(&mut db, &capture, Some(&payload), Some(&member), false);
}
