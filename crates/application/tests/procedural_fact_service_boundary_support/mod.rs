mod retained_hearing;
use crate::procedural_fact_service_support::{detail, empty, hasher, preparation, text, values};
use application::{
    case_stages::StageSupportSnapshot,
    cases::{
        case_administration_digest, CaseActorSnapshot, CaseAdministrationSnapshot,
        CurrentCaseAdministration,
    },
    documents::{DocumentRecord, StageDocumentFormat, StageFormatPolicy},
    procedural_facts::*,
    ApplicationError,
};
use domain::{
    case_administration::{CaseAdministrationValues, CaseAdministrativeStatus, CaseRevision},
    cases::{CaseId, CaseMetadata},
    crypto::{DocumentVersion, DocumentVersionRef},
    identity::UserId,
    procedural_time::DeclaredProceduralTime,
};

pub fn support(record: &DocumentRecord) -> StageSupportSnapshot {
    StageSupportSnapshot {
        reference: DocumentVersionRef {
            id: record.id,
            version: record.version,
        },
        digest: record.digest,
        name: record.name.clone(),
        format: StageDocumentFormat::Pdf,
        policy: StageFormatPolicy::PdfDocxV1,
    }
}
pub fn parent(actor: UserId, case: CaseId) -> ResolutionSourceMaterial {
    let historical = crate::store::record();
    let values = crate::store::with_support(&historical);
    let command = crate::store::record_command(values.clone());
    let mut sources = empty();
    let admitted = support(&historical);
    sources.direct_supports.push(admitted.clone());
    let detail = detail(actor, case, &command, values, sources);
    let ProceduralFactSnapshot::Resolution(snapshot) = detail.snapshot else {
        unreachable!()
    };
    ResolutionSourceMaterial {
        snapshot: *snapshot,
        hearing: None,
        admitted_support: Some(admitted),
    }
}
pub fn parent_ref(parent: &ResolutionSourceMaterial) -> FactResolutionRef {
    FactResolutionRef {
        id: parent.snapshot.root.id(),
        revision: parent.snapshot.metadata.revision,
    }
}
fn provenance(record: Option<&DocumentRecord>) -> FactProvenance {
    FactProvenance::ExternalReference {
        reference: text("Declared external evidence"),
        support: record.map(|record| {
            FactEvidence::new(
                DocumentVersionRef {
                    id: record.id,
                    version: record.version,
                },
                record.digest,
                FactLabel::new("Declared locator").unwrap(),
            )
        }),
    }
}
pub fn notice(parent: &ResolutionSourceMaterial, records: &[DocumentRecord]) -> NotificationValues {
    let unlinked = || FactPerson::Unlinked {
        label: FactLabel::new("Declared person").unwrap(),
        description: text("No directory identity selected"),
    };
    NotificationValues::new(NotificationValuesInput {
        resolution: parent_ref(parent),
        character: FactDeclaration::Known(NotificationCharacter::Personal),
        medium: FactDeclaration::Known(NotificationMedium::InPerson),
        context: FactDeclaration::Known(NotificationContext::OutsideHearing),
        outcome: FactDeclaration::Known(NotificationOutcome::Attempted),
        subtype: None,
        practiced_at: DeclaredProceduralTime::unknown(),
        received_at: None,
        stated_effect: None,
        intended_recipient: FactDeclaration::Unknown(text("Not stated")),
        actual_receiver: FactDeclaration::Unknown(text("Not stated")),
        representation: match records.get(1) {
            Some(record) => FactRepresentation::Declared {
                represented: unlinked(),
                representative: unlinked(),
                scope: text("Declared scope"),
                provenance: Box::new(provenance(Some(record))),
            },
            None => FactRepresentation::NotRecorded(text("Not stated")),
        },
        summary: text("Declared practice"),
        provenance: provenance(records.first()),
    })
    .unwrap()
}
pub fn notice_command(values: NotificationValues) -> ProceduralFactCommand {
    ProceduralFactCommand::Notification(
        NotificationCommand::new(
            FactOperationId::new(),
            NotificationId::new(),
            values.resolution().id,
            FactChange::record(values),
        )
        .unwrap(),
    )
}
pub fn notice_sources(
    parent: &ResolutionSourceMaterial,
    values: &NotificationValues,
) -> FactSources {
    let selected = FactSourceSelection::from_values(&ProceduralFactValues::Notification(Box::new(
        values.clone(),
    )));
    let projection = resolve_fact_resolution(
        hasher().as_ref(),
        parent.snapshot.root.case_id(),
        &selected,
        Some(parent),
    )
    .unwrap()
    .unwrap();
    let mut sources = empty();
    sources.resolved.resolution = Some(projection.snapshot);
    sources.views.resolution = Some(projection.view);
    sources
}
pub fn notice_detail(
    actor: UserId,
    case: CaseId,
    command: &ProceduralFactCommand,
    values: NotificationValues,
    sources: FactSources,
) -> FactDetail {
    let FactTarget::Notification { id, resolution_id } = command.target() else {
        unreachable!()
    };
    let values_digest = fact_values_digest(
        hasher().as_ref(),
        &ProceduralFactValues::Notification(Box::new(values.clone())),
    );
    let sources_digest = fact_sources_digest(hasher().as_ref(), &sources).unwrap();
    let submission_digest = fact_submission_digest(
        hasher().as_ref(),
        actor,
        case,
        command,
        values_digest,
        sources_digest,
    )
    .unwrap();
    FactDetail {
        snapshot: ProceduralFactSnapshot::Notification(Box::new(NotificationSnapshot {
            root: NotificationRoot::new(id, case, resolution_id),
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
                recorded_administration: preparation(case).observed_administration,
                recorded_at: crate::case_support::instant(),
                recorded_by: CaseActorSnapshot {
                    id: actor,
                    email: "captured@example.test".into(),
                },
            },
        })),
        sources,
    }
}
pub fn notice_correction(base: &FactDetail, values: NotificationValues) -> ProceduralFactCommand {
    let FactTarget::Notification { id, resolution_id } = base.snapshot.target() else {
        unreachable!()
    };
    ProceduralFactCommand::Notification(
        NotificationCommand::new(
            FactOperationId::new(),
            id,
            resolution_id,
            FactChange::correct(
                base.snapshot.metadata().revision,
                values,
                text("Correct declaration"),
            ),
        )
        .unwrap(),
    )
}
pub fn correction_base(
    actor: UserId,
    case: CaseId,
) -> (ResolutionSourceMaterial, NotificationValues, FactDetail) {
    let parent = parent(actor, case);
    let values = notice(&parent, &[]);
    let base = notice_detail(
        actor,
        case,
        &notice_command(values.clone()),
        values.clone(),
        notice_sources(&parent, &values),
    );
    fact_receipt_matches(hasher().as_ref(), &base).unwrap();
    (parent, values, base)
}
pub fn replace_parent_summary(parent: &mut ResolutionSourceMaterial, actor: UserId, case: CaseId) {
    let value = &parent.snapshot.values;
    let values = ResolutionValues::new(ResolutionValuesInput {
        class: value.class().clone(),
        subtype: value.subtype().cloned(),
        issuer: value.issuer().clone(),
        issued_at: value.issued_at(),
        summary: text("Substituted historical summary"),
        provenance: value.provenance().clone(),
    });
    let command = ProceduralFactCommand::Resolution(ResolutionCommand::new(
        parent.snapshot.metadata.receipt.operation_id,
        parent.snapshot.root.id(),
        FactChange::record(values.clone()),
    ));
    let mut sources = empty();
    sources.direct_supports = parent.admitted_support.iter().cloned().collect();
    let replacement = detail(actor, case, &command, values, sources);
    fact_receipt_matches(hasher().as_ref(), &replacement).unwrap();
    let ProceduralFactSnapshot::Resolution(snapshot) = replacement.snapshot else {
        unreachable!()
    };
    parent.snapshot = *snapshot;
}
pub fn closed(case: CaseId, actor: UserId) -> CurrentCaseAdministration {
    let values = CaseAdministrationValues::basic(CaseMetadata::new("Case", "REF-1").unwrap())
        .with_status(CaseAdministrativeStatus::Closed);
    CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
        case_id: case,
        revision: CaseRevision::FIRST,
        values_digest: case_administration_digest(hasher().as_ref(), &values),
        values,
        changed_at: crate::case_support::instant(),
        changed_by: CaseActorSnapshot {
            id: actor,
            email: "actor@example.test".into(),
        },
    }))
}
pub fn second_version(first: &DocumentRecord) -> DocumentRecord {
    crate::crypto::processor()
        .prepare_version(
            first.id,
            DocumentVersion::new(2).unwrap(),
            "next.pdf",
            b"%PDF-another exact version",
        )
        .unwrap()
}
pub fn future_values() -> ResolutionValues {
    let original = values();
    ResolutionValues::new(ResolutionValuesInput {
        class: original.class().clone(),
        subtype: None,
        issuer: original.issuer().clone(),
        issued_at: DeclaredProceduralTime::date("9999-12-31".parse().unwrap(), None).unwrap(),
        summary: original.summary().clone(),
        provenance: original.provenance().clone(),
    })
}
pub fn inconsistent<T>(result: Result<T, ApplicationError>) {
    assert!(matches!(
        result,
        Err(ApplicationError::ProceduralFact(
            ProceduralFactError::StoredInconsistent(_)
        ))
    ));
}

pub fn withdrawal(base: &FactDetail) -> ProceduralFactCommand {
    let FactTarget::Resolution(id) = base.snapshot.target() else {
        unreachable!()
    };
    ProceduralFactCommand::Resolution(ResolutionCommand::new(
        FactOperationId::new(),
        id,
        FactChange::withdraw(
            base.snapshot.metadata().revision,
            text("Withdraw declaration"),
        ),
    ))
}
