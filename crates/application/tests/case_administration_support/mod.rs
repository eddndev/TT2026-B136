use application::cases::*;
use domain::cases::{CaseId, CaseMetadata};
use domain::crypto::Sha256Digest;
use domain::identity::UserId;

use crate::case_support::instant;

pub fn creation() -> PenalCaseCreation {
    PenalCaseCreation::new(
        CaseMetadata::new("Penal title", "REF-1").unwrap(),
        PenalCaseProfile::new(
            "NUC-1",
            "Fiscalia",
            "CJ-1",
            "Organo",
            &["A", "B"],
            Some("Info\nLine"),
            None,
        )
        .unwrap(),
    )
}
pub fn values() -> CaseEditableValues {
    creation().into_values().editable().clone()
}
pub fn list_query() -> CaseAdministrationQuery {
    CaseAdministrationQuery::new(
        2,
        Some(CaseId::from_uuid(uuid::Uuid::from_u128(7))),
        CaseStatusFilter::All,
        CaseProfileFilter::Pending,
        Some("T%_"),
        Some("NUC-1"),
        Some("CJ-1"),
    )
    .unwrap()
}
pub fn history_query() -> CaseAdministrationHistoryQuery {
    CaseAdministrationHistoryQuery::new(2, Some(9)).unwrap()
}
pub fn expected() -> CaseRevisionExpectation {
    CaseRevisionExpectation::new(7)
}
pub fn detail(id: CaseId, actor: UserId, revision: u32) -> CaseAdministrationDetail {
    CaseAdministrationDetail {
        origin: CaseOrigin {
            id,
            created_by: if revision == 1 {
                actor
            } else {
                UserId::from_uuid(uuid::Uuid::from_u128(1))
            },
            created_at: if revision == 1 {
                instant()
            } else {
                instant() - time::Duration::days(2)
            },
        },
        administration: CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
            case_id: id,
            revision: CaseRevision::new(revision).unwrap(),
            values: creation().into_values(),
            values_digest: Sha256Digest::from_bytes(&[8; 32]).unwrap(),
            changed_at: instant(),
            changed_by: CaseActorSnapshot {
                id: actor,
                email: "captured@example.com".into(),
            },
        })),
        initial_stage: Some(CaseInitialStageRegistration {
            case_id: id,
            stage_revision: CaseStageRevision::FIRST,
            administration_revision: CaseRevision::FIRST,
            stage: InitialCaseStage::Investigation,
            administration_digest: Sha256Digest::from_bytes(
                &[if revision == 1 { 8 } else { 1 }; 32],
            )
            .unwrap(),
            recorded_at: if revision == 1 {
                instant()
            } else {
                instant() - time::Duration::days(1)
            },
            recorded_by: CaseActorSnapshot {
                id: actor,
                email: if revision == 1 {
                    "captured@example.com"
                } else {
                    "initial@example.com"
                }
                .into(),
            },
        }),
    }
}
