pub use crate::correction_support::*;
pub use crate::record_support::record_reference;
pub use application::{
    identity::Principal, precautionary_hearings::PrecautionaryContext,
    typed_participants::SubjectSnapshot, ApplicationError,
};
pub use domain::{cases::CaseId, clock::OffsetDateTime};

#[derive(Clone)]
pub struct ReplacementFixture {
    pub actor: Principal,
    pub case_id: CaseId,
    pub command: MeasureAdministrativeCommand,
    pub context: PrecautionaryContext,
    pub subject: SubjectSnapshot,
    pub history: MeasureDecisionRecordHistoryEvidence,
    pub recorded_at: OffsetDateTime,
}
impl ReplacementFixture {
    pub fn initial() -> Self {
        Self::from_correction(CorrectionFixture::initial())
    }
    pub fn from_correction(prior: CorrectionFixture) -> Self {
        let mut subject = crate::measure_source_support::Fixture::unknown(true)
            .sources
            .subject;
        subject.id =
            application::typed_participants::CaseSubjectId::from_uuid(uuid::Uuid::from_u128(901));
        let mut command = prior.command;
        command.reason = note(
            "Replace the incorrectly selected subject while preserving the original declaration",
        );
        command.action = MeasureAdministrativeAction::MarkEnteredInErrorAndReplace {
            replacement_id: id(10),
            subject: crate::measure_source_support::subject_ref(&subject),
        };
        Self {
            actor: prior.actor,
            case_id: prior.case_id,
            command,
            context: prior.context,
            subject,
            history: MeasureDecisionRecordHistoryEvidence {
                records: MeasureRecordHistoryEvidence {
                    judicial: prior.history,
                    administrative: vec![],
                },
                decisions: vec![],
            },
            recorded_at: prior.recorded_at,
        }
    }
    pub fn replacement_id(&self) -> MeasureId {
        match &self.command.action {
            MeasureAdministrativeAction::MarkEnteredInErrorAndReplace {
                replacement_id, ..
            } => *replacement_id,
            _ => panic!("expected a replacement action"),
        }
    }
    pub fn previous(&self) -> MeasureCapture {
        self.history
            .records
            .judicial
            .groups
            .iter()
            .flat_map(|g| &g.capture.measures)
            .find(|m| reference(m) == self.command.target)
            .unwrap()
            .clone()
    }
    pub fn prepare(&self) -> Result<CheckedMeasureAdministrativeReview, ApplicationError> {
        prepare_measure_administrative_replacement_with_decision_history(
            &Hasher,
            &self.actor,
            self.case_id,
            self.command.clone(),
            MeasureAdministrativeReplacementMaterial {
                context: self.context.clone(),
                subject: self.subject.clone(),
            },
            &self.history,
        )
    }
    pub fn capture(&self) -> MeasureAdministrativeCapture {
        self.prepare()
            .unwrap()
            .into_capture(&Hasher, self.recorded_at)
            .unwrap()
    }
}
pub fn row(
    capture: &MeasureAdministrativeCapture,
    id: MeasureId,
) -> &MeasureAdministrativeRecordCapture {
    capture
        .records
        .iter()
        .find(|row| row.result.id == id)
        .unwrap()
}
pub fn expected_values(prior: &MeasureValues, subject: &SubjectSnapshot) -> MeasureValues {
    MeasureValues::new(MeasureValuesInput {
        subject: crate::measure_source_support::subject_ref(subject),
        kind: prior.kind(),
        conditions: prior.conditions().clone(),
        validity: prior.validity().clone(),
        supervision: prior.supervision().clone(),
    })
}
pub fn append(
    fixture: &ReplacementFixture,
    capture: &MeasureAdministrativeCapture,
) -> MeasureDecisionRecordHistoryEvidence {
    let origin =
        measure_administrative_origin_with_decision_history(&Hasher, capture, &fixture.history)
            .unwrap();
    let mut history = fixture.history.clone();
    history
        .records
        .administrative
        .push(MeasureAdministrativeEvidence {
            origin,
            capture: capture.clone(),
        });
    history
}
