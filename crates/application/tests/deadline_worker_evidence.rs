#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;

use application::{
    cases::CurrentCaseAdministration, deadline_worker::administration_evidence_digest,
};
use deadline_observation_support::{administration, inputs};
use domain::{case_administration::CaseRevision, cases::CaseMetadata};

#[test]
fn worker_administration_commitment_binds_history_beyond_unchanged_values() {
    let base = administration(inputs::case_id(), false);
    let digest = administration_evidence_digest(inputs::hasher().as_ref(), &base);
    for mutation in 0..4 {
        let mut changed = base.clone();
        let CurrentCaseAdministration::Recorded(snapshot) = &mut changed else {
            unreachable!();
        };
        match mutation {
            0 => snapshot.revision = CaseRevision::new(2).unwrap(),
            1 => snapshot.changed_by.email = "different@example.test".into(),
            2 => {
                snapshot.changed_at = snapshot
                    .changed_at
                    .to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap())
            }
            _ => snapshot.case_id = domain::cases::CaseId::from_uuid(uuid::Uuid::from_u128(991)),
        }
        assert_eq!(base.values(), changed.values());
        assert_ne!(
            digest,
            administration_evidence_digest(inputs::hasher().as_ref(), &changed),
            "historical metadata mutation {mutation} escaped the commitment"
        );
    }
}

#[test]
fn worker_administration_commitment_distinguishes_original_and_recorded_metadata() {
    let recorded = administration(inputs::case_id(), false);
    let original = CurrentCaseAdministration::Unrevised(
        CaseMetadata::new("Observed case", "REF-OBS").unwrap(),
    );
    assert_eq!(original.values(), recorded.values());
    let digest = administration_evidence_digest(inputs::hasher().as_ref(), &original);
    assert_ne!(
        digest,
        administration_evidence_digest(inputs::hasher().as_ref(), &recorded)
    );
    let changed = CurrentCaseAdministration::Unrevised(
        CaseMetadata::new("Different original", "REF-OBS").unwrap(),
    );
    assert_ne!(
        digest,
        administration_evidence_digest(inputs::hasher().as_ref(), &changed)
    );
    assert_eq!(
        digest,
        administration_evidence_digest(inputs::hasher().as_ref(), &original)
    );
}
