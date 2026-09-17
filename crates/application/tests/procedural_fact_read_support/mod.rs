use crate::{
    case_support::{CountingClock, MockIdentity},
    procedural_fact_service_support::*,
};
use application::{documents::*, procedural_facts::*, ApplicationError};
use domain::{cases::CaseId, identity::UserId};
use std::sync::Arc;

#[allow(dead_code)]
#[path = "../procedural_fact_service_support/store.rs"]
mod store;
pub use store::MockStore as MockReads;
struct NoAdmission;
impl DocumentFormatBatchValidator for NoAdmission {
    fn validate_batch(
        &self,
        _: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        panic!("reads must not admit document content")
    }
}
pub fn service(
    store: MockReads,
    identity: MockIdentity,
) -> (ProceduralFactService, Arc<CountingClock>) {
    let clock = Arc::new(CountingClock::default());
    (
        ProceduralFactService::new(
            Arc::new(store),
            Arc::new(identity),
            Arc::new(crate::crypto::processor()),
            Arc::new(NoAdmission),
            hasher(),
            clock.clone(),
        ),
        clock,
    )
}
pub fn id(value: u128) -> ResolutionId {
    ResolutionId::from_uuid(uuid::Uuid::from_u128(value))
}
pub fn nid(value: u128) -> NotificationId {
    NotificationId::from_uuid(uuid::Uuid::from_u128(value))
}
pub fn resolution(case_id: CaseId, value: u128) -> ResolutionOverview {
    ResolutionOverview {
        root: ResolutionRoot::new(id(value), case_id),
        revision: FactRevision::initial(),
        status: FactStatus::Recorded,
        class: values().class().clone(),
        issued_at: values().issued_at(),
    }
}
pub fn notification(case_id: CaseId, parent: ResolutionId, value: u128) -> NotificationOverview {
    NotificationOverview {
        root: NotificationRoot::new(nid(value), case_id, parent),
        revision: FactRevision::initial(),
        status: FactStatus::Recorded,
        resolution: FactResolutionRef {
            id: parent,
            revision: FactRevision::initial(),
        },
        outcome: FactDeclaration::Known(NotificationOutcome::Practiced),
        practiced_at: domain::procedural_time::DeclaredProceduralTime::unknown(),
    }
}
pub fn resolution_detail(case_id: CaseId, actor: UserId, revision: u32) -> FactDetail {
    let change = if revision == 1 {
        FactChange::record(values())
    } else {
        FactChange::correct(
            FactRevision::new(revision - 1).unwrap(),
            values(),
            text("Correction"),
        )
    };
    let command = ProceduralFactCommand::Resolution(ResolutionCommand::new(
        FactOperationId::new(),
        id(10),
        change,
    ));
    detail(actor, case_id, &command, values(), empty())
}
pub fn notification_detail(case_id: CaseId, actor: UserId) -> FactDetail {
    let parent = FactResolutionRef {
        id: id(10),
        revision: FactRevision::initial(),
    };
    let values = NotificationValues::new(NotificationValuesInput {
        resolution: parent,
        character: FactDeclaration::Known(NotificationCharacter::Personal),
        medium: FactDeclaration::Known(NotificationMedium::InPerson),
        context: FactDeclaration::Known(NotificationContext::OutsideHearing),
        outcome: FactDeclaration::Known(NotificationOutcome::Practiced),
        subtype: None,
        practiced_at: domain::procedural_time::DeclaredProceduralTime::unknown(),
        received_at: None,
        stated_effect: None,
        intended_recipient: FactDeclaration::Unknown(text("Unknown recipient")),
        actual_receiver: FactDeclaration::Unknown(text("Unknown receiver")),
        representation: FactRepresentation::NotRecorded(text("Not recorded")),
        summary: text("Declared notification"),
        provenance: FactProvenance::OperatorNote {
            note: text("Operator declaration"),
        },
    })
    .unwrap();
    let command = ProceduralFactCommand::Notification(
        NotificationCommand::new(
            FactOperationId::new(),
            nid(20),
            parent.id,
            FactChange::record(values.clone()),
        )
        .unwrap(),
    );
    let mut sources = empty();
    sources.resolved.resolution = Some(FactResolutionSourceSnapshot {
        case_id,
        reference: parent,
        values_digest: digest(1),
        submission_digest: digest(2),
        status: FactStatus::Withdrawn,
    });
    sources.views.resolution = Some(FactResolutionView {
        reference: parent,
        class: FactDeclaration::Known(ResolutionClass::Order),
        issuer: FactDeclaration::Unknown(text("Unknown issuer")),
        issued_at: domain::procedural_time::DeclaredProceduralTime::unknown(),
        summary: text("Original resolution"),
    });
    let value_digest = fact_values_digest(
        hasher().as_ref(),
        &ProceduralFactValues::Notification(Box::new(values.clone())),
    );
    let source_digest = fact_sources_digest(hasher().as_ref(), &sources).unwrap();
    let mut metadata = resolution_detail(case_id, actor, 1)
        .snapshot
        .metadata()
        .clone();
    metadata.values_digest = value_digest;
    metadata.receipt = FactReceipt {
        operation_id: command.operation_id(),
        action: FactAction::Record,
        expected_revision: 0,
        sources_digest: source_digest,
        submission_digest: fact_submission_digest(
            hasher().as_ref(),
            actor,
            case_id,
            &command,
            value_digest,
            source_digest,
        )
        .unwrap(),
    };
    FactDetail {
        snapshot: ProceduralFactSnapshot::Notification(Box::new(NotificationSnapshot {
            root: NotificationRoot::new(nid(20), case_id, parent.id),
            metadata,
            values,
        })),
        sources,
    }
}
pub fn assert_bad<T: std::fmt::Debug>(value: Result<T, ApplicationError>) {
    assert!(
        matches!(
            value,
            Err(ApplicationError::ProceduralFact(
                ProceduralFactError::StoredInconsistent(_)
            ))
        ),
        "{value:?}"
    );
}
