#![allow(dead_code)]

use domain::{
    cases::CaseId, crypto::Sha256Digest, deadline_triggers::*, hearing_results::*,
    hearings::HearingId, procedural_facts::*, procedural_time::DeclaredProceduralTime,
};
use time::macros::datetime;
use uuid::Uuid;

pub fn case_id() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(1))
}

pub fn qualification(
    purpose: QualifiedTriggerPurpose,
    at: DeclaredProceduralTime,
) -> QualifiedTriggerTime {
    QualifiedTriggerTime {
        purpose,
        at,
        statement: FactText::new("Expressly declared start\nKeep this statement").unwrap(),
        locator: FactLabel::new("Record page 4, paragraph 2").unwrap(),
    }
}

pub struct ResolutionFixture {
    pub values: ResolutionValues,
}
impl ResolutionFixture {
    pub fn new(at: DeclaredProceduralTime) -> Self {
        let mut input = super::procedural_fact_support::resolution_input();
        input.issued_at = at;
        Self {
            values: ResolutionValues::new(input),
        }
    }
    pub fn reference(&self) -> FactResolutionRef {
        FactResolutionRef {
            id: ResolutionId::from_uuid(Uuid::from_u128(10)),
            revision: FactRevision::new(3).unwrap(),
        }
    }
    pub fn selection(&self, qualification: Option<QualifiedTriggerTime>) -> TriggerSelection {
        TriggerSelection {
            case_id: case_id(),
            source: FactDeclaration::Known(TriggerSourceRef::Resolution(self.reference())),
            qualification,
        }
    }
    pub fn material(&self) -> TriggerMaterial<'_> {
        TriggerMaterial::Resolution {
            root: ResolutionRoot::new(self.reference().id, case_id()),
            revision: self.reference().revision,
            values: &self.values,
            digests: FactTriggerDigests {
                values: Sha256Digest::from_array([3; 32]),
                sources: Sha256Digest::from_array([4; 32]),
                submission: Sha256Digest::from_array([5; 32]),
            },
        }
    }
}

pub struct HearingFixture {
    pub values: HearingResultValues,
}
impl HearingFixture {
    pub fn concluded() -> Self {
        Self {
            values: HearingResultValues::new(HearingResultValuesInput {
                occurrence: HearingResultOccurrence::Occurred,
                extent: HearingResultExtent::Concluded,
                event_time: DeclaredHearingResultTime::instant(
                    datetime!(2026-03-02 10:20:30 -06:00),
                )
                .unwrap(),
                summary: HearingResultText::new("Session concluded; end not separately timed")
                    .unwrap(),
                attendees: vec![],
                agreements: vec![],
                provenance: HearingResultProvenance::new(
                    HearingResultProvenanceKind::OperatorNote,
                    None,
                    None,
                )
                .unwrap(),
            })
            .unwrap(),
        }
    }
    pub fn reference(&self) -> FactHearingRef {
        FactHearingRef {
            hearing_id: HearingId::from_uuid(Uuid::from_u128(20)),
            result_id: HearingResultId::from_uuid(Uuid::from_u128(30)),
            revision: HearingResultRevision::new(7).unwrap(),
            agreement_id: None,
        }
    }
    pub fn selection(&self, qualification: Option<QualifiedTriggerTime>) -> TriggerSelection {
        TriggerSelection {
            case_id: case_id(),
            source: FactDeclaration::Known(TriggerSourceRef::HearingResult(self.reference())),
            qualification,
        }
    }
    pub fn material(&self) -> TriggerMaterial<'_> {
        let reference = self.reference();
        TriggerMaterial::HearingResult {
            case_id: case_id(),
            hearing_id: reference.hearing_id,
            result_id: reference.result_id,
            revision: reference.revision,
            values: &self.values,
            digests: HearingTriggerDigests {
                values: Sha256Digest::from_array([6; 32]),
                submission: Sha256Digest::from_array([7; 32]),
            },
        }
    }
}
