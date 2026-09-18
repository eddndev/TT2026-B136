#![allow(dead_code)]
use super::*;
use application::{
    cases::{CaseActorSnapshot, CurrentCaseAdministration},
    deadline_evaluations::*,
    deadline_inputs::*,
    deadline_profiles::*,
    deadlines::*,
    procedural_facts::*,
};
use domain::{
    cases::CaseMetadata,
    crypto::{DocumentHasher, Sha256Digest},
    deadline_triggers::*,
    identity::{Role, UserId},
    procedural_time::DeclaredProceduralTime,
};
use time::OffsetDateTime;
use uuid::Uuid;
#[path = "../../../application/tests/deadline_profile_catalog_support/values.rs"]
mod profiles;
pub struct Hasher;
impl DocumentHasher for Hasher {
    fn hash_bytes(&self, _: &[u8]) -> Sha256Digest {
        digest()
    }
    fn hash_stream(&self, _: &mut dyn std::io::Read) -> Result<Sha256Digest, domain::DomainError> {
        Ok(digest())
    }
}
pub fn actor() -> UserId {
    UserId::from_uuid(Uuid::parse_str(ACTOR).unwrap())
}
pub fn instant() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1767744000).unwrap()
}
pub fn fixture() -> DeadlineDetail {
    let mut p = profiles::input(Some(case()));
    p.conditions[0].id = Uuid::nil();
    let profile = DeadlineProfileDetail {
        id: DeadlineProfileId::from_uuid(Uuid::parse_str(PROFILE).unwrap()),
        revision: DeadlineProfileRevision::initial(),
        definition: DeadlineProfileDefinition::new(p).unwrap(),
        algorithm: DeadlineProfileAlgorithm::V1,
        definition_digest: digest(),
        status: DeadlineProfileStatus::Published,
        reason: None,
        receipt: DeadlineProfileReceipt {
            operation_id: DeadlineProfileOperationId::new(),
            action: DeadlineProfileAction::Publish,
            expected_revision: 0,
            submission_digest: digest(),
        },
        recorded_at: instant(),
        recorded_by: DeadlineProfileActorSnapshot {
            id: actor(),
            email: "owner@example.com".into(),
        },
    };
    let administration =
        CurrentCaseAdministration::Unrevised(CaseMetadata::new("Case", "REF-1").unwrap());
    let source = source(administration.clone());
    let input = DeadlineEvaluationInput {
        selection: TriggerSelection {
            case_id: case(),
            source: FactDeclaration::Known(TriggerSourceRef::Resolution(FactResolutionRef {
                id: ResolutionId::from_uuid(Uuid::parse_str(SOURCE).unwrap()),
                revision: FactRevision::initial(),
            })),
            qualification: None,
        },
        calendar: None,
        ordered_quantity: None,
        qualification: DeadlineApplicability {
            statement: text("Declared applicability"),
            locator: label("Resolution page 1"),
            scope_applies: FactDeclaration::Known(true),
            unresolved_incident: FactDeclaration::Known(false),
            conditions: vec![DeadlineConditionAnswer {
                id: Uuid::nil(),
                applies: FactDeclaration::Known(true),
                locator: label("Condition record"),
            }],
        },
    };
    let material = DeadlineInputMaterial {
        case_id: case(),
        administration,
        source: Some(source.clone()),
        source_head: Some(source),
        calendar: None,
        calendar_head: None,
    };
    let result = DeadlineEvaluationRecord::capture(
        &evaluate_profiled_deadline(&Hasher, &profile.definition, &input, &material).unwrap(),
    );
    DeadlineDetail {
        id: DeadlineId::from_uuid(Uuid::nil()),
        case_id: case(),
        revision: DeadlineRevision::new(1).unwrap(),
        definition: DeadlineDefinition {
            title: label("Declared period"),
            profile: DeadlineProfileRef {
                id: profile.id,
                revision: profile.revision,
            },
            input,
            responsible: actor(),
        },
        calculation: DeadlineCalculation {
            profile,
            material,
            result,
        },
        responsible: DeadlineResponsibleSnapshot {
            id: actor(),
            email: "owner@example.com".into(),
            role: Role::Owner,
        },
        attention: DeadlineAttention::Pending,
        status: DeadlineStatus::Active,
        reason: None,
        receipt: DeadlineReceipt {
            version: DeadlineReceiptVersion::Legacy,
            operation_id: DeadlineOperationId::from_uuid(Uuid::nil()),
            action: DeadlineAction::Register,
            expected_revision: 0,
            review_digest: digest(),
            capture_digest: digest(),
            submission_digest: digest(),
        },
        recorded_at: instant(),
        recorded_by: DeadlineActorSnapshot::User {
            id: actor(),
            email: "owner@example.com".into(),
        },
        tracking: None,
    }
}
fn text(v: &str) -> FactText {
    FactText::new(v).unwrap()
}
fn label(v: &str) -> FactLabel {
    FactLabel::new(v).unwrap()
}
fn source(administration: CurrentCaseAdministration) -> DeadlineSourceDetail {
    let values = ResolutionValues::new(ResolutionValuesInput {
        class: FactDeclaration::Known(ResolutionClass::Order),
        subtype: None,
        issuer: FactDeclaration::Unknown(text("Unknown issuer")),
        issued_at: DeclaredProceduralTime::date("2026-01-06".parse().unwrap(), None).unwrap(),
        summary: text("Declared resolution"),
        provenance: FactProvenance::OperatorNote {
            note: text("Declared by operator"),
        },
    });
    DeadlineSourceDetail::Fact(Box::new(FactDetail {
        snapshot: ProceduralFactSnapshot::Resolution(Box::new(ResolutionSnapshot {
            root: ResolutionRoot::new(
                ResolutionId::from_uuid(Uuid::parse_str(SOURCE).unwrap()),
                case(),
            ),
            values,
            metadata: FactRevisionMetadata {
                revision: FactRevision::initial(),
                values_digest: digest(),
                status: FactStatus::Recorded,
                reason: None,
                receipt: FactReceipt {
                    operation_id: FactOperationId::new(),
                    action: FactAction::Record,
                    expected_revision: 0,
                    sources_digest: digest(),
                    submission_digest: digest(),
                },
                recorded_administration: administration,
                recorded_at: instant(),
                recorded_by: CaseActorSnapshot {
                    id: actor(),
                    email: "owner@example.com".into(),
                },
            },
        })),
        sources: FactSources {
            resolved: FactResolvedSources {
                resolution: None,
                participants: vec![],
                hearing_results: vec![],
            },
            views: FactSourceViews {
                resolution: None,
                participants: vec![],
                hearing_results: vec![],
            },
            direct_supports: vec![],
        },
    }))
}
pub fn draft(command: DeadlineCommand, detail: &DeadlineDetail) -> DeadlineDraft {
    DeadlineDraft {
        case_id: detail.case_id,
        actor: actor(),
        result_revision: command.result_revision().unwrap(),
        command,
        definition: detail.definition.clone(),
        calculation: detail.calculation.clone(),
        responsible: detail.responsible.clone(),
        attention: detail.attention.clone(),
        status: detail.status,
        review_digest: digest(),
        capture_digest: digest(),
        submission_digest: digest(),
    }
}
