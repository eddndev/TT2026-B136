use application::identity::Principal;
use application::precautionary_hearings::{
    precautionary_hearing_submission_bytes, PrecautionaryContextExpectation,
    PrecautionaryHearingChange, PrecautionaryHearingCommand,
};
use domain::case_administration::{CaseRevision, CaseStageRevision};
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest};
use domain::hearings::{
    HearingModality, HearingNote, HearingSupportRef, HearingTime, HearingVenue,
};
use domain::identity::{Role, UserId};
use domain::precautionary_hearings::{
    PrecautionaryHearingId, PrecautionaryHearingOperationId, PrecautionaryHearingPurpose,
    PrecautionaryHearingRevision, PrecautionaryHearingSchedulingBasis, PrecautionaryHearingValues,
    PrecautionaryHearingValuesInput,
};
use time::macros::datetime;
use uuid::Uuid;

#[derive(Clone)]
pub struct Fixture {
    pub actor: Principal,
    pub case_id: CaseId,
    pub command: PrecautionaryHearingCommand,
    pub resolved_values: PrecautionaryHearingValues,
}

impl Fixture {
    pub fn schedule() -> Self {
        let values = PrecautionaryHearingValues::new(values_input()).unwrap();
        Self {
            actor: Principal {
                id: UserId::from_uuid(Uuid::from_u128(1)),
                email: "a@b".to_owned(),
                role: Role::Owner,
            },
            case_id: CaseId::from_uuid(Uuid::from_u128(2)),
            command: PrecautionaryHearingCommand {
                operation_id: PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(3)),
                hearing_id: PrecautionaryHearingId::from_uuid(Uuid::from_u128(4)),
                change: PrecautionaryHearingChange::Schedule {
                    context: context(),
                    values: values.clone(),
                },
            },
            resolved_values: values,
        }
    }

    pub fn replace() -> Self {
        let mut value = Self::schedule();
        value.command.change = PrecautionaryHearingChange::Replace {
            expected_revision: PrecautionaryHearingRevision::new(7).unwrap(),
            expected_capture_digest: Sha256Digest::from_array([0x44; 32]),
            context: context(),
            values: value.resolved_values.clone(),
            reason: note("Correct the declared venue"),
        };
        value
    }

    pub fn cancel() -> Self {
        let mut value = Self::schedule();
        value.command.change = PrecautionaryHearingChange::Cancel {
            expected_revision: PrecautionaryHearingRevision::new(7).unwrap(),
            expected_capture_digest: Sha256Digest::from_array([0x44; 32]),
            reason: note("Remove duplicate appointment"),
        };
        value
    }

    pub fn bytes(&self) -> Vec<u8> {
        precautionary_hearing_submission_bytes(
            &self.actor,
            self.case_id,
            &self.command,
            &self.resolved_values,
        )
        .unwrap()
    }

    pub fn set_values(&mut self, values: PrecautionaryHearingValues) {
        match &mut self.command.change {
            PrecautionaryHearingChange::Schedule { values: target, .. }
            | PrecautionaryHearingChange::Replace { values: target, .. } => {
                *target = values.clone();
            }
            PrecautionaryHearingChange::Cancel { .. } => {}
        }
        self.resolved_values = values;
    }
}

pub fn context() -> PrecautionaryContextExpectation {
    PrecautionaryContextExpectation {
        administration_revision: CaseRevision::new(5).unwrap(),
        stage_revision: CaseStageRevision::new(6).unwrap(),
        context_digest: Sha256Digest::from_array([0x33; 32]),
    }
}

pub fn note(value: &str) -> HearingNote {
    HearingNote::new(value).unwrap()
}

pub fn basis(
    statement: &str,
    document: u128,
    version: u32,
    digest: u8,
    locator: &str,
) -> PrecautionaryHearingSchedulingBasis {
    PrecautionaryHearingSchedulingBasis::new(
        note(statement),
        HearingSupportRef::new(
            DocumentVersionRef {
                id: DocumentId::from_uuid(Uuid::from_u128(document)),
                version: DocumentVersion::new(version).unwrap(),
            },
            Sha256Digest::from_array([digest; 32]),
        ),
        note(locator),
    )
}

pub fn values_input() -> PrecautionaryHearingValuesInput {
    PrecautionaryHearingValuesInput {
        purpose: PrecautionaryHearingPurpose::Imposition,
        scheduled_at: HearingTime::new(datetime!(1970-01-01 00:00 UTC)).unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Court A").unwrap(),
        note: None,
        participants: Vec::new(),
        scheduling_basis: basis("Set by order", 77, 4, 0x42, "Page 2"),
        review_targets: Vec::new(),
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
