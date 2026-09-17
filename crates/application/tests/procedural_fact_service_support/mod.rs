use application::{
    cases::{CaseActorSnapshot, CurrentCaseAdministration},
    procedural_facts::*,
};
use domain::{
    cases::{CaseId, CaseMetadata},
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
    procedural_time::DeclaredProceduralTime,
};
use std::sync::Arc;

pub fn hasher() -> Arc<dyn DocumentHasher + Send + Sync> {
    crate::crypto::processor_ports().hasher.into()
}
pub fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
pub fn values() -> ResolutionValues {
    ResolutionValues::new(ResolutionValuesInput {
        class: FactDeclaration::Known(ResolutionClass::Order),
        subtype: None,
        issuer: FactDeclaration::Unknown(text("Unknown issuer")),
        issued_at: DeclaredProceduralTime::unknown(),
        summary: text("Declared resolution"),
        provenance: FactProvenance::OperatorNote {
            note: text("Declared by operator"),
        },
    })
}
pub fn empty() -> FactSources {
    FactSources {
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
    }
}
pub fn command() -> ProceduralFactCommand {
    ProceduralFactCommand::Resolution(ResolutionCommand::new(
        FactOperationId::new(),
        ResolutionId::new(),
        FactChange::record(values()),
    ))
}
pub fn preparation(case_id: CaseId) -> FactPreparation {
    FactPreparation {
        case_id,
        base: None,
        observed_administration: CurrentCaseAdministration::Unrevised(
            CaseMetadata::new("Case", "REF-1").unwrap(),
        ),
        source_material: FactSourceMaterial {
            resolution: None,
            participants: vec![],
            hearing_results: vec![],
        },
        records: vec![],
    }
}
pub fn detail(
    actor: UserId,
    case_id: CaseId,
    command: &ProceduralFactCommand,
    values: ResolutionValues,
    sources: FactSources,
) -> FactDetail {
    let FactTarget::Resolution(id) = command.target() else {
        panic!("resolution command expected")
    };
    let values_digest = fact_values_digest(
        hasher().as_ref(),
        &ProceduralFactValues::Resolution(Box::new(values.clone())),
    );
    let sources_digest = fact_sources_digest(hasher().as_ref(), &sources).unwrap();
    let submission_digest = fact_submission_digest(
        hasher().as_ref(),
        actor,
        case_id,
        command,
        values_digest,
        sources_digest,
    )
    .unwrap();
    FactDetail {
        snapshot: ProceduralFactSnapshot::Resolution(Box::new(ResolutionSnapshot {
            root: ResolutionRoot::new(id, case_id),
            values,
            metadata: FactRevisionMetadata {
                revision: command.result_revision().unwrap(),
                values_digest,
                status: command.action().resulting_status(),
                reason: command.reason().cloned(),
                receipt: FactReceipt {
                    operation_id: command.operation_id(),
                    action: command.action(),
                    expected_revision: command.expected_revision(),
                    sources_digest,
                    submission_digest,
                },
                recorded_administration: preparation(case_id).observed_administration,
                recorded_at: crate::case_support::instant(),
                recorded_by: CaseActorSnapshot {
                    id: actor,
                    email: "operator@example.com".into(),
                },
            },
        })),
        sources,
    }
}
pub fn snapshot_mut(detail: &mut FactDetail) -> &mut ResolutionSnapshot {
    let ProceduralFactSnapshot::Resolution(snapshot) = &mut detail.snapshot else {
        panic!("resolution expected")
    };
    snapshot
}
pub fn digest(byte: u8) -> Sha256Digest {
    Sha256Digest::from_array([byte; 32])
}
pub fn assert_inconsistent(result: Result<(), application::ApplicationError>) {
    assert!(matches!(
        result,
        Err(application::ApplicationError::ProceduralFact(
            ProceduralFactError::StoredInconsistent(_)
        ))
    ));
}
