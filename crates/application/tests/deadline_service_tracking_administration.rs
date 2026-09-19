#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_service_support;
mod deadline_support;

use application::{
    cases::*,
    deadline_tracking::{TrackingPolicies, TrackingPolicy},
    deadlines::*,
    ApplicationError,
};
use deadline_service_support::*;
use domain::{
    case_administration::{CaseAdministrationValues, CaseRevision},
    cases::CaseMetadata,
    identity::Role,
};

fn policies() -> TrackingPolicies {
    TrackingPolicies {
        profile: TrackingPolicy::Follow,
        source: TrackingPolicy::Follow,
        calendar: TrackingPolicy::Undetermined,
    }
}

fn administration(revision: u32, closed: bool) -> CurrentCaseAdministration {
    let values = CaseAdministrationValues::basic(
        CaseMetadata::new("Administrative capture", "REF-CAPTURE").unwrap(),
    )
    .with_status(if closed {
        CaseAdministrativeStatus::Closed
    } else {
        CaseAdministrativeStatus::Active
    });
    CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
        case_id: case_id(),
        revision: CaseRevision::new(revision).unwrap(),
        values_digest: case_administration_digest(hasher().as_ref(), &values),
        values,
        changed_at: case_support::instant(),
        changed_by: CaseActorSnapshot {
            id: owner(),
            email: "owner@example.com".into(),
        },
    }))
}

fn replace_administration(
    preparation: &mut DeadlinePreparation,
    administration: CurrentCaseAdministration,
) {
    preparation.administration = administration.clone();
    preparation
        .resolved
        .as_mut()
        .unwrap()
        .material
        .administration = administration;
}

fn prepare_v2(
    command: DeadlineCommand,
    preparation: DeadlinePreparation,
) -> PreparedDeadlineChange {
    prepare_tracked_deadline_change(
        hasher().as_ref(),
        DeadlineActorSnapshot::User {
            id: owner(),
            email: "owner@example.com".into(),
        },
        case_id(),
        command,
        preparation,
        Some(policies()),
        None,
    )
    .unwrap()
}

fn qualification(
    correction: bool,
    administration: CurrentCaseAdministration,
) -> (DeadlineCommand, DeadlinePreparation) {
    let (command, mut preparation) = fixture();
    replace_administration(&mut preparation, administration);
    if !correction {
        return (command, preparation);
    }
    let base = tracked_detail(&prepare_v2(command, preparation.clone()));
    let (command, mut next) = followup(
        &base,
        DeadlineChange::Correct {
            expected_revision: base.revision,
            definition: base.definition.clone(),
            reason: evaluation::text("Review the same explicit inputs"),
        },
    );
    next.resolved = preparation.resolved;
    next.responsible = preparation.responsible;
    (command, next)
}

fn recapture_only(returned: &mut DeadlineDetail) {
    returned.receipt.capture_digest =
        hasher().hash_bytes(&deadline_capture_bytes(hasher().as_ref(), returned).unwrap());
}

fn assert_capture_only(reviewed: &PreparedDeadlineChange, returned: &DeadlineDetail) {
    deadline_receipt_matches(hasher().as_ref(), returned).unwrap();
    let mut receipt = reviewed.receipt();
    assert_eq!(returned.receipt.review_digest, reviewed.review_digest());
    assert_eq!(
        returned.receipt.submission_digest,
        reviewed.submission_digest()
    );
    assert_ne!(returned.receipt.capture_digest, reviewed.capture_digest());
    receipt.capture_digest = returned.receipt.capture_digest;
    assert_eq!(returned.receipt, receipt);
    let mut tracking = reviewed.tracking().unwrap().clone();
    tracking.administration = returned.tracking.as_ref().unwrap().administration.clone();
    assert_eq!(returned.tracking.as_ref(), Some(&tracking));
    let mut calculation = reviewed.calculation().clone();
    calculation.material.administration = returned.calculation.material.administration.clone();
    assert_eq!(returned.calculation, calculation);
}

fn submit_reply(
    command: DeadlineCommand,
    preparation: DeadlinePreparation,
    reviewed: &PreparedDeadlineChange,
    returned: DeadlineDetail,
) -> Result<DeadlineDetail, ApplicationError> {
    let expected = reviewed.submission_digest();
    let expected_receipt = reviewed.receipt();
    let expected_tracking = reviewed.tracking().cloned();
    let expected_base = preparation.base.clone();
    let requested = command.clone();
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |actor, case, actual| {
            assert_eq!(actor, owner());
            assert_eq!(case, case_id());
            assert_eq!(actual, &requested);
            Ok(preparation)
        });
    store
        .expect_commit()
        .times(1)
        .return_once(move |actor, submitted| {
            assert_eq!(actor, owner());
            assert_eq!(submitted.receipt(), expected_receipt);
            assert_eq!(submitted.tracking(), expected_tracking.as_ref());
            assert_eq!(submitted.preparation().base, expected_base);
            Ok(returned)
        });
    let (workflow, _) = service(store, identity(Role::Owner, 2));
    workflow.submit(
        "session",
        case_id(),
        DeadlineHumanCommand::new(command, Some(policies())).unwrap(),
        expected,
    )
}

#[test]
fn register_and_correct_reject_invalid_tracking_administration_with_valid_receipts() {
    for correction in [false, true] {
        for mutation in 0..5 {
            let (command, preparation) = qualification(correction, administration(2, false));
            let reviewed = prepare_v2(command.clone(), preparation.clone());
            let mut returned = tracked_detail(&reviewed);
            let tracking = returned.tracking.as_mut().unwrap();
            match mutation {
                0 => tracking.administration = administration(1, false),
                1 => tracking.administration = fixture().1.administration,
                2 => {
                    let CurrentCaseAdministration::Recorded(snapshot) =
                        &mut tracking.administration
                    else {
                        unreachable!()
                    };
                    snapshot.changed_by.email = "another@example.com".into();
                }
                3 => {
                    let CurrentCaseAdministration::Recorded(snapshot) =
                        &mut tracking.administration
                    else {
                        unreachable!()
                    };
                    let original = snapshot.changed_at;
                    snapshot.changed_at =
                        original.to_offset(time::UtcOffset::from_hms(3, 0, 0).unwrap());
                    assert_eq!(snapshot.changed_at, original);
                    assert_ne!(snapshot.changed_at.offset(), original.offset());
                }
                _ => tracking.administration = administration(3, true),
            }
            assert_eq!(&returned.calculation, reviewed.calculation());
            recapture_only(&mut returned);
            assert_capture_only(&reviewed, &returned);
            let result = submit_reply(command, preparation, &reviewed, returned);
            assert!(
                matches!(
                    result,
                    Err(ApplicationError::Deadline(
                        DeadlineError::StoredInconsistent(_)
                    ))
                ),
                "correction={correction}, mutation={mutation}: {result:?}"
            );
        }
    }
}

#[test]
fn register_and_correct_reject_mixed_administration_captures_with_valid_receipts() {
    for correction in [false, true] {
        for material_newer in [false, true] {
            let (command, preparation) = qualification(correction, administration(2, false));
            let reviewed = prepare_v2(command.clone(), preparation.clone());
            let mut returned = tracked_detail(&reviewed);
            if material_newer {
                returned.calculation.material.administration = administration(3, false);
            } else {
                returned.tracking.as_mut().unwrap().administration = administration(3, false);
            }
            recapture_only(&mut returned);
            assert_capture_only(&reviewed, &returned);
            let result = submit_reply(command, preparation, &reviewed, returned);
            assert!(
                matches!(
                    result,
                    Err(ApplicationError::Deadline(
                        DeadlineError::StoredInconsistent(_)
                    ))
                ),
                "correction={correction}, material_newer={material_newer}: {result:?}"
            );
        }
    }
}

#[test]
fn register_and_correct_accept_active_administration_advances_in_both_captures() {
    for correction in [false, true] {
        for already_recorded in [false, true] {
            let before = if already_recorded {
                administration(1, false)
            } else {
                fixture().1.administration
            };
            let after = administration(if already_recorded { 2 } else { 1 }, false);
            let (command, preparation) = qualification(correction, before);
            let reviewed = prepare_v2(command.clone(), preparation.clone());
            let mut returned = tracked_detail(&reviewed);
            returned.calculation.material.administration = after.clone();
            returned.tracking.as_mut().unwrap().administration = after.clone();
            recapture_only(&mut returned);
            assert_capture_only(&reviewed, &returned);

            let mut refreshed = preparation.clone();
            replace_administration(&mut refreshed, after);
            assert_eq!(refreshed.base, preparation.base);
            let honest = prepare_v2(command.clone(), refreshed);
            assert_eq!(honest.receipt().version, reviewed.receipt().version);
            assert_eq!(returned, tracked_detail(&honest));
            let expected = returned.clone();
            assert_eq!(
                submit_reply(command, preparation, &reviewed, returned).unwrap(),
                expected
            );
        }
    }
}
