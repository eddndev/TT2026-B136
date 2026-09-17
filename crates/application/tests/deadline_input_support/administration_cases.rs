use super::*;
use application::cases::{
    case_administration_digest, CaseActorSnapshot, CaseAdministrationSnapshot,
    CurrentCaseAdministration,
};
use domain::{
    case_administration::{CaseAdministrationValues, CaseAdministrativeStatus, CaseRevision},
    cases::CaseMetadata,
};

fn recorded(case: CaseId, status: CaseAdministrativeStatus) -> CurrentCaseAdministration {
    let values = CaseAdministrationValues::basic(CaseMetadata::new("Case", "REF-1").unwrap())
        .with_status(status);
    CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
        case_id: case,
        revision: CaseRevision::FIRST,
        values_digest: case_administration_digest(hasher().as_ref(), &values),
        values,
        changed_at: crate::case_support::instant(),
        changed_by: CaseActorSnapshot {
            id: actor(),
            email: "owner@example.test".into(),
        },
    }))
}
#[test]
fn closed_case_administration_remains_readable_without_a_complete_penal_profile() {
    let source = DeadlineSourceDetail::Fact(Box::new(resolution(1, false, "2026-01-06")));
    let request = request(&source);
    let mut material = material(source);
    material.administration = recorded(case_id(), CaseAdministrativeStatus::Closed);
    let before = material.clone();
    let result = check_deadline_inputs(hasher().as_ref(), &request, &material).unwrap();
    assert_eq!(
        result.arithmetic().unwrap().outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: "2026-01-06".parse().unwrap()
        }
    );
    assert_eq!(material, before);
}
#[test]
fn corrupt_or_cross_case_administration_cannot_hide_behind_unknown_source() {
    for wrong_case in [false, true] {
        let (request, mut material) = unknown();
        material.administration = recorded(
            if wrong_case { CaseId::new() } else { case_id() },
            CaseAdministrativeStatus::Closed,
        );
        if !wrong_case {
            let CurrentCaseAdministration::Recorded(snapshot) = &mut material.administration else {
                unreachable!()
            };
            snapshot.values_digest = Sha256Digest::from_array([255; 32]);
        }
        inconsistent(check_deadline_inputs(
            hasher().as_ref(),
            &request,
            &material,
        ));
    }
}
