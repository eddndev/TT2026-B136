use super::tracking_context_tests::{metadata, pending, technical};
use super::tracking_response_test_support::{case, records};
use application::{
    deadline_evaluations::{evaluate_profiled_deadline, DeadlineEvaluationRecord},
    deadline_inputs::DeadlineSourceDetail,
    deadline_profiles::{DeadlineProfileDefinition, DeadlineProfileDefinitionInput},
    deadline_reevaluation::*,
    deadline_tracking::*,
    deadlines::*,
    procedural_facts::{
        FactDetail, FactResolutionSourceSnapshot, FactResolutionView, NotificationSnapshot,
        ProceduralFactSnapshot,
    },
};
use domain::{
    deadline_triggers::{TriggerField, TriggerRequirement, TriggerSourceRef},
    procedural_facts::{
        FactDeclaration, FactProvenance, FactRepresentation, FactResolutionRef, FactText,
        NotificationId, NotificationRoot, NotificationValues, NotificationValuesInput,
    },
};
use uuid::Uuid;

pub(super) fn notification_parent_event() -> DeadlineDetail {
    let mut value = technical();
    let Some(DeadlineSourceDetail::Fact(original)) = value.calculation.material.source.clone()
    else {
        unreachable!()
    };
    let ProceduralFactSnapshot::Resolution(parent) = original.snapshot else {
        unreachable!()
    };
    let parent_ref = FactResolutionRef {
        id: parent.root.id(),
        revision: parent.metadata.revision,
    };
    let notice_id = NotificationId::from_uuid(Uuid::from_u128(9));
    let unknown = || FactText::new("Not declared").unwrap();
    let values = NotificationValues::new(NotificationValuesInput {
        resolution: parent_ref,
        character: FactDeclaration::Unknown(unknown()),
        medium: FactDeclaration::Unknown(unknown()),
        context: FactDeclaration::Unknown(unknown()),
        outcome: FactDeclaration::Unknown(unknown()),
        subtype: None,
        practiced_at: parent.values.issued_at(),
        received_at: None,
        stated_effect: None,
        intended_recipient: FactDeclaration::Unknown(unknown()),
        actual_receiver: FactDeclaration::Unknown(unknown()),
        representation: FactRepresentation::NotRecorded(unknown()),
        summary: unknown(),
        provenance: FactProvenance::OperatorNote { note: unknown() },
    })
    .unwrap();
    let mut sources = original.sources;
    sources.resolved.resolution = Some(FactResolutionSourceSnapshot {
        case_id: case(),
        reference: parent_ref,
        values_digest: parent.metadata.values_digest,
        submission_digest: parent.metadata.receipt.submission_digest,
        status: parent.metadata.status,
    });
    sources.views.resolution = Some(FactResolutionView {
        reference: parent_ref,
        class: parent.values.class().clone(),
        issuer: parent.values.issuer().clone(),
        issued_at: parent.values.issued_at(),
        summary: parent.values.summary().clone(),
    });
    let notice = DeadlineSourceDetail::Fact(Box::new(FactDetail {
        snapshot: ProceduralFactSnapshot::Notification(Box::new(NotificationSnapshot {
            root: NotificationRoot::new(notice_id, case(), parent_ref.id),
            metadata: parent.metadata.clone(),
            values,
        })),
        sources,
    }));
    value.definition.input.selection.source =
        FactDeclaration::Known(TriggerSourceRef::Notification {
            id: notice_id,
            revision: parent.metadata.revision,
            resolution: parent_ref,
        });
    value.calculation.material.source = Some(notice.clone());
    value.calculation.material.source_head = Some(notice);
    let definition = &value.calculation.profile.definition;
    value.calculation.profile.definition =
        DeadlineProfileDefinition::new(DeadlineProfileDefinitionInput {
            title: definition.title().clone(),
            description: definition.description().clone(),
            scope: definition.scope().clone(),
            references: definition.references().to_vec(),
            trigger: TriggerRequirement::SourceField(TriggerField::NotificationPracticedAt),
            template: definition.template(),
            completion: definition.completion().clone(),
            conditions: definition.conditions().to_vec(),
            examples: definition.examples().to_vec(),
        })
        .unwrap();
    value.calculation.result = DeadlineEvaluationRecord::capture(
        &evaluate_profiled_deadline(
            &records::Hasher,
            &value.calculation.profile.definition,
            &value.definition.input,
            &value.calculation.material,
        )
        .unwrap(),
    );
    let tracking = value.tracking.as_mut().unwrap();
    tracking.observations.entries[0].revision = 1;
    let mut observed_parent = tracking.observations.entries[1].clone();
    observed_parent.role = ObservationRole::NotificationParent;
    observed_parent.revision = 4;
    let source = &mut tracking.observations.entries[1];
    source.family = DependencyFamily::Notification;
    source.id = notice_id.as_uuid();
    source.parent_resolution = Some(ResolutionReference {
        id: parent_ref.id.as_uuid(),
        revision: parent_ref.revision.get(),
    });
    tracking.observations.entries.push(observed_parent);
    tracking.review = pending(
        TrackingDependency::Source,
        TrackingReviewReason::SourceChanged,
    );
    metadata(&mut value).cause = Some(TechnicalCause::SourceEvent {
        job_id: Uuid::nil(),
        event: SourceEventReference {
            sequence: 17,
            family: DependencyFamily::Resolution,
            source_id: parent_ref.id.as_uuid(),
            revision: 3,
            case_id: Some(case()),
            hearing_id: None,
            operation_id: Uuid::from_u128(8),
        },
    });
    value
}
