use application::{
    cases::{CaseActorSnapshot, CurrentCaseAdministration},
    procedural_facts::*,
    ApplicationError,
};
use domain::{
    cases::{CaseId, CaseMetadata},
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    identity::UserId,
    procedural_time::DeclaredProceduralTime,
};
use uuid::Uuid;

pub fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
pub fn revision(value: u32) -> FactRevision {
    FactRevision::new(value).unwrap()
}
pub fn case_id() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(1))
}
pub fn parent() -> ResolutionId {
    ResolutionId::from_uuid(Uuid::from_u128(2))
}
pub fn other_parent() -> ResolutionId {
    ResolutionId::from_uuid(Uuid::from_u128(3))
}
pub fn notification_id() -> NotificationId {
    NotificationId::from_uuid(Uuid::from_u128(4))
}

pub fn resolution_values() -> ResolutionValues {
    ResolutionValues::new(ResolutionValuesInput {
        class: FactDeclaration::Known(ResolutionClass::Order),
        subtype: None,
        issuer: FactDeclaration::Unknown(text("Issuer not stated")),
        issued_at: DeclaredProceduralTime::unknown(),
        summary: text("Declared resolution"),
        provenance: FactProvenance::OperatorNote {
            note: text("Operator declaration"),
        },
    })
}
pub fn notification_input(parent: ResolutionId) -> NotificationValuesInput {
    NotificationValuesInput {
        resolution: FactResolutionRef {
            id: parent,
            revision: revision(1),
        },
        character: FactDeclaration::Known(NotificationCharacter::Personal),
        medium: FactDeclaration::Known(NotificationMedium::Electronic),
        context: FactDeclaration::Known(NotificationContext::OutsideHearing),
        outcome: FactDeclaration::Known(NotificationOutcome::Attempted),
        subtype: None,
        practiced_at: DeclaredProceduralTime::unknown(),
        received_at: None,
        stated_effect: None,
        intended_recipient: FactDeclaration::Unknown(text("Recipient not stated")),
        actual_receiver: FactDeclaration::Unknown(text("Receiver not stated")),
        representation: FactRepresentation::NotRecorded(text("No relationship captured")),
        summary: text("Declared practice"),
        provenance: FactProvenance::OperatorNote {
            note: text("Operator declaration"),
        },
    }
}
pub fn notification_values(parent: ResolutionId) -> NotificationValues {
    NotificationValues::new(notification_input(parent)).unwrap()
}
pub fn resolution_command(
    id: ResolutionId,
    change: FactChange<ResolutionValues>,
) -> ProceduralFactCommand {
    ProceduralFactCommand::Resolution(ResolutionCommand::new(FactOperationId::new(), id, change))
}
pub fn notification_command(
    id: NotificationId,
    parent: ResolutionId,
    change: FactChange<NotificationValues>,
) -> ProceduralFactCommand {
    ProceduralFactCommand::Notification(
        NotificationCommand::new(FactOperationId::new(), id, parent, change).unwrap(),
    )
}
fn metadata(value: u32, status: FactStatus) -> FactRevisionMetadata {
    let action = match (value, status) {
        (_, FactStatus::Withdrawn) => FactAction::Withdraw,
        (1, _) => FactAction::Record,
        _ => FactAction::Correct,
    };
    FactRevisionMetadata {
        revision: revision(value),
        values_digest: Sha256Digest::from_array([0; 32]),
        status,
        reason: (action != FactAction::Record).then(|| text("Captured reason")),
        receipt: FactReceipt {
            operation_id: FactOperationId::new(),
            action,
            expected_revision: value - 1,
            sources_digest: Sha256Digest::from_array([0; 32]),
            submission_digest: Sha256Digest::from_array([0; 32]),
        },
        recorded_administration: CurrentCaseAdministration::Unrevised(
            CaseMetadata::new("Case", "Reference").unwrap(),
        ),
        recorded_at: OffsetDateTime::UNIX_EPOCH,
        recorded_by: CaseActorSnapshot {
            id: UserId::from_uuid(Uuid::from_u128(5)),
            email: "actor@example.com".into(),
        },
    }
}
pub fn resolution_base(
    case: CaseId,
    id: ResolutionId,
    revision: u32,
    status: FactStatus,
) -> ProceduralFactSnapshot {
    ProceduralFactSnapshot::Resolution(Box::new(ResolutionSnapshot {
        root: ResolutionRoot::new(id, case),
        metadata: metadata(revision, status),
        values: resolution_values(),
    }))
}
pub fn notification_base(
    case: CaseId,
    id: NotificationId,
    parent: ResolutionId,
    revision: u32,
    status: FactStatus,
) -> ProceduralFactSnapshot {
    ProceduralFactSnapshot::Notification(Box::new(NotificationSnapshot {
        root: NotificationRoot::new(id, case, parent),
        metadata: metadata(revision, status),
        values: notification_values(parent),
    }))
}
pub fn assert_error(result: Result<(), ApplicationError>, expected: ProceduralFactError) {
    let Err(ApplicationError::ProceduralFact(error)) = result else {
        panic!("expected procedural fact error, got {result:?}");
    };
    assert_eq!(error, expected);
}
pub fn assert_inconsistent(result: Result<(), ApplicationError>) {
    let Err(ApplicationError::ProceduralFact(ProceduralFactError::StoredInconsistent(message))) =
        result
    else {
        panic!("expected stored inconsistency, got {result:?}");
    };
    assert!(!message.trim().is_empty());
}
