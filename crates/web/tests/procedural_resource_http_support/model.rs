use application::{case_stages::*, cases::*, procedural_facts::*, procedural_resources::*};
use domain::{
    cases::{CaseId, CaseMetadata},
    clock::OffsetDateTime,
    crypto::*,
    identity::UserId,
    procedural_time::DeclaredProceduralTime,
};
use uuid::Uuid;
pub fn digest() -> Sha256Digest {
    Sha256Digest::from_array([7; 32])
}
fn text(v: &str) -> FactText {
    FactText::new(v).unwrap()
}
fn label(v: &str) -> FactLabel {
    FactLabel::new(v).unwrap()
}
pub fn values() -> ResourceValues {
    ResourceValues::new(ResourceValuesInput {
        kind: ResourceKind::Revocation,
        mode: FactDeclaration::Known(ResourceMode::Written),
        title: label("Written revocation record"),
        resolution: FactResolutionRef {
            id: ResolutionId::from_uuid(Uuid::from_u128(10)),
            revision: FactRevision::new(2).unwrap(),
        },
        resolution_evidence: FactEvidence::new(
            DocumentVersionRef {
                id: DocumentId::from_uuid(Uuid::from_u128(20)),
                version: DocumentVersion::new(3).unwrap(),
            },
            Sha256Digest::from_array([42; 32]),
            label("Order page 2"),
        ),
        resolution_reference: FactDeclaration::Known(label("Order reference")),
        issuing_authority: FactDeclaration::Unknown(text("Issuer not recorded")),
        receiving_authority: None,
        resolution_at: DeclaredProceduralTime::unknown(),
        notification_at: None,
        challenged_part: text("Declared challenged paragraph"),
        grounds: text("Declared reasons"),
        appellants: vec![ResourceAppellant::new(
            label("Captured name"),
            FactDeclaration::Known(label("Affected party")),
            None,
        )],
    })
    .unwrap()
}
pub fn supports(refs: Vec<FactSupportRef>) -> Vec<StageSupportSnapshot> {
    refs.into_iter()
        .map(|r| StageSupportSnapshot {
            reference: r.reference(),
            digest: r.digest(),
            name: "exact-support.pdf".into(),
            format: StageDocumentFormat::Pdf,
            policy: StageFormatPolicy::PdfDocxV1,
        })
        .collect()
}
pub fn sources(case: CaseId, values: &ResourceValues) -> ResourceSources {
    ResourceSources {
        resolution: FactResolutionProjection {
            snapshot: FactResolutionSourceSnapshot {
                case_id: case,
                reference: values.resolution(),
                values_digest: digest(),
                submission_digest: digest(),
                status: FactStatus::Withdrawn,
            },
            view: FactResolutionView {
                reference: values.resolution(),
                class: FactDeclaration::Known(ResolutionClass::Order),
                issuer: values.issuing_authority().clone(),
                issued_at: values.resolution_at(),
                summary: text("Historical order"),
            },
        },
        appellants: vec![],
        supports: supports(values.direct_supports()),
    }
}
pub fn detail(case: CaseId, command: &ResourceCommand) -> ResourceDetail {
    let values = match &command.change {
        ResourceChange::Register { values } | ResourceChange::Correct { values, .. } => {
            values.clone()
        }
        _ => values(),
    };
    let previous = (command.expected_revision() > 0).then(|| ResourceRevisionRef {
        revision: ResourceRevision::new(command.expected_revision()).unwrap(),
        capture_digest: digest(),
    });
    let act = match &command.change {
        ResourceChange::RecordAct { act_id, values, .. } => Some(ResourceActCapture {
            id: *act_id,
            revision: ResourceActRevision::initial(),
            values: values.clone(),
            supports: supports(values.direct_supports()),
            previous: None,
        }),
        ResourceChange::CorrectAct {
            act_id,
            expected_act_revision,
            values,
            ..
        } => Some(ResourceActCapture {
            id: *act_id,
            revision: expected_act_revision.next().unwrap(),
            values: values.clone(),
            supports: supports(values.direct_supports()),
            previous,
        }),
        _ => None,
    };
    ResourceDetail {
        case_id: case,
        id: command.resource_id,
        revision: command.result_revision().unwrap(),
        sources: sources(case, &values),
        values,
        status: if command.action() == ResourceAction::Archive {
            ResourceStatus::Archived
        } else {
            ResourceStatus::Active
        },
        act,
        reason: command.reason().cloned(),
        receipt: ResourceReceipt {
            operation_id: command.operation_id,
            action: command.action(),
            expected_revision: command.expected_revision(),
            previous,
            values_digest: digest(),
            sources_digest: digest(),
            submission_digest: digest(),
            capture_digest: digest(),
        },
        recorded_by: CaseActorSnapshot {
            id: UserId::from_uuid(Uuid::from_u128(50)),
            email: "actor@example.test".into(),
        },
        recorded_at: OffsetDateTime::UNIX_EPOCH,
        recorded_administration: CurrentCaseAdministration::Unrevised(
            CaseMetadata::new("Case title", "REF-1").unwrap(),
        ),
        recorded_stage: CurrentCaseStage::Unregistered,
    }
}
pub fn command(id: ResourceId, revision: u32) -> ResourceCommand {
    ResourceCommand {
        operation_id: ResourceOperationId::from_uuid(Uuid::from_u128(3)),
        resource_id: id,
        change: if revision == 1 {
            ResourceChange::Register { values: values() }
        } else {
            ResourceChange::Correct {
                expected_revision: ResourceRevision::new(revision - 1).unwrap(),
                values: values(),
                reason: text("Correction"),
            }
        },
    }
}
