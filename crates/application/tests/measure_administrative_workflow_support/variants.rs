use super::*;
use application::case_stages::{StageDocumentFormat, StageFormatPolicy, StageSupportSnapshot};

impl Fixture {
    pub fn mark() -> Self {
        let mut fixture = Self::single();
        fixture.command.action = MeasureAdministrativeAction::MarkEnteredInError;
        fixture.command.reason = note("Declare the captured record entered in error");
        fixture
    }

    pub fn after_c() -> Self {
        let previous = Self::single();
        let operation = previous.operation(at() + Duration::seconds(1));
        Self::from_record(
            RecordFixture::next(&operation.capture, &previous.history.records, 1),
            previous.material.support_record,
        )
    }

    pub fn after_m2() -> Self {
        let previous = Self::single();
        let correction = previous.operation(at() + Duration::seconds(1));
        let mut judicial = FixtureV2::confirm(&correction.capture, &previous.history.records);
        let old = &previous.history.records.judicial.groups[0].capture;
        judicial.command.values = old.review.command.values.clone();
        judicial.material.support = old.decision.support.clone();
        let group = judicial.capture();
        let next = AdministrativeFixture::after(&group, &judicial.history, 1);
        Self::from_administrative(next, previous.material.support_record)
    }

    pub fn from_administrative(
        fixture: AdministrativeFixture,
        support_record: DocumentRecord,
    ) -> Self {
        Self {
            actor: fixture.actor,
            case_id: fixture.case_id,
            command: fixture.command.clone(),
            material: MeasureAdministrativeReady {
                context: fixture.context,
                support_record,
                target_head: fixture.command.target,
                dependency_inventory: MeasureAdministrativeDependencyInventory {
                    records: fixture.history.clone(),
                    hearings: vec![],
                },
            },
            history: fixture.history,
        }
    }

    pub fn after_corrected_m2() -> Self {
        let previous = Self::after_m2();
        let operation = previous.operation(at() + Duration::seconds(3));
        let prior = &operation.capture.records[0];
        let mut history = previous.history.clone();
        history
            .records
            .administrative
            .push(MeasureAdministrativeEvidence {
                origin: operation.origin,
                capture: operation.capture.clone(),
            });
        let supervision = match prior.result.values.supervision() {
            MeasureSupervision::Known { statement, .. } => statement,
            MeasureSupervision::Unknown { reason } => reason,
        };
        let command = MeasureAdministrativeCommand {
            operation_id: MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(4501)),
            target: record_reference(prior),
            context: expectation(&previous.material.context),
            reason: note("Correct another recorded condition"),
            action: MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
                note("Another corrected condition after M2"),
                prior.result.values.validity().clone(),
                supervision.clone(),
            )),
        };
        Self {
            actor: previous.actor,
            case_id: previous.case_id,
            material: MeasureAdministrativeReady {
                target_head: command.target,
                dependency_inventory: MeasureAdministrativeDependencyInventory {
                    records: history.clone(),
                    hearings: vec![],
                },
                ..previous.material
            },
            command,
            history,
        }
    }

    pub fn after_modification() -> Self {
        let first = Self::single();
        let base = &first.history.records.judicial.groups[0].capture;
        let mut later = crate::effect_support::LaterFixture::confirm(base);
        let record = crate::crypto::processor()
            .prepare_version(
                DocumentId::from_uuid(Uuid::from_u128(92)),
                DocumentVersion::initial(),
                "modified-resolution.pdf",
                b"Later judicial modification",
            )
            .unwrap();
        let selected = DocumentVersionRef {
            id: record.id,
            version: record.version,
        };
        let mut decision =
            crate::measure_decision_fixtures::decision_input(&later.request.command.values);
        decision.support = HearingSupportRef::new(selected, record.digest);
        later.request.command.values = MeasureDecisionValues::new(decision);
        later.request.material.support = StageSupportSnapshot {
            reference: selected,
            digest: record.digest,
            name: record.name.clone(),
            format: StageDocumentFormat::Pdf,
            policy: StageFormatPolicy::PdfDocxV1,
        };
        let mut values = crate::effect_support::values_input(&base.measures[0].result.values);
        values.conditions = note("Judicially modified conditions");
        later.effects(vec![MeasureEffect::Modify {
            previous: reference(&base.measures[0]),
            values: MeasureValues::new(values),
        }]);
        let group = later.clone().capture();
        let next =
            CorrectionFixture::from_group(&group, &later.evidence, group.measures[0].result.id);
        Self::from_record(RecordFixture::from_first(next), record)
    }
}
