#[allow(unused_imports)]
pub use crate::hearing_support::{
    hasher, identity, instant, CountingClock, MockIdentity, Validator,
};
use application::documents::StageSupportReadLimits;
use application::{hearing_results::*, hearings::HearingDetail, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime, hearings::HearingId, identity::UserId};
use mockall::mock;
use std::sync::Arc;
mock! {
    pub Store {}
    impl HearingResultStore for Store {
        fn list(&self,actor:UserId,case_id:CaseId,hearing_id:HearingId,query:HearingResultQuery,at:OffsetDateTime)->Result<HearingResultPage,ApplicationError>;
        fn get(&self,actor:UserId,case_id:CaseId,hearing_id:HearingId,id:HearingResultId,revision:Option<HearingResultRevision>,at:OffsetDateTime)->Result<HearingResultDetail,ApplicationError>;
        fn history(&self,actor:UserId,case_id:CaseId,hearing_id:HearingId,id:HearingResultId,query:HearingResultHistoryQuery,at:OffsetDateTime)->Result<HearingResultHistoryPage,ApplicationError>;
        fn prepare(&self,actor:UserId,case_id:CaseId,command:&HearingResultCommand,limits:&StageSupportReadLimits)->Result<HearingResultPreparation,ApplicationError>;
        fn commit(&self,actor:UserId,case_id:CaseId,prepared:PreparedHearingResultChange)->Result<HearingResultDetail,ApplicationError>;
    }
}
pub fn values_input(values: &HearingResultValues) -> HearingResultValuesInput {
    HearingResultValuesInput {
        occurrence: values.occurrence(),
        extent: values.extent(),
        event_time: values.event_time(),
        summary: values.summary().clone(),
        attendees: values.attendees().to_vec(),
        agreements: values.agreements().to_vec(),
        provenance: values.provenance().clone(),
    }
}
pub fn values() -> HearingResultValues {
    HearingResultValues::new(HearingResultValuesInput {
        occurrence: HearingResultOccurrence::Occurred,
        extent: HearingResultExtent::Partial,
        event_time: DeclaredHearingResultTime::date(instant().date(), instant().offset()).unwrap(),
        summary: HearingResultText::new("Declared session").unwrap(),
        attendees: vec![],
        agreements: vec![],
        provenance: HearingResultProvenance::new(
            HearingResultProvenanceKind::OperatorNote,
            None,
            None,
        )
        .unwrap(),
    })
    .unwrap()
}
pub fn preparation(case_id: CaseId, actor: UserId) -> HearingResultPreparation {
    let context = crate::hearing_support::context(case_id, actor);
    let command = crate::hearing_support::command();
    let anchor = crate::hearing_support::detail(
        case_id,
        actor,
        &command,
        crate::hearing_support::values(),
        &context,
    );
    HearingResultPreparation {
        case_id,
        base: None,
        administration: context.administration,
        anchor,
        continuation: None,
        attendees: vec![],
        records: vec![],
    }
}
pub fn command(prep: &HearingResultPreparation) -> HearingResultCommand {
    HearingResultCommand {
        operation_id: HearingResultOperationId::new(),
        hearing_id: prep.anchor.snapshot.id,
        result_id: HearingResultId::new(),
        change: HearingResultChange::Record {
            anchor_revision: prep.anchor.snapshot.revision,
            continuation: None,
            values: values(),
        },
    }
}
pub fn anchor(detail: &HearingDetail) -> HearingResultAnchorSnapshot {
    let s = &detail.snapshot;
    HearingResultAnchorSnapshot {
        reference: HearingResultAnchor {
            hearing_id: s.id,
            revision: s.revision,
            values_digest: s.values_digest,
            submission_digest: s.receipt.submission_digest,
        },
        status: s.status,
        kind: s.values.kind(),
        scheduled_at: s.values.scheduled_at(),
        scheduling_context: s.scheduling_context,
    }
}
pub fn previous(snapshot: &HearingResultSnapshot) -> HearingResultContinuationSnapshot {
    HearingResultContinuationSnapshot {
        reference: HearingResultContinuation {
            hearing_id: snapshot.hearing_id,
            result_id: snapshot.id,
            revision: snapshot.revision,
            values_digest: snapshot.values_digest,
            submission_digest: snapshot.receipt.submission_digest,
        },
        status: snapshot.status,
    }
}
pub fn detail(
    actor: UserId,
    command: &HearingResultCommand,
    prep: &HearingResultPreparation,
) -> HearingResultDetail {
    let values = match &command.change {
        HearingResultChange::Record { values, .. }
        | HearingResultChange::Correct { values, .. } => values.clone(),
        HearingResultChange::Withdraw { .. } => prep.base.as_ref().unwrap().snapshot.values.clone(),
    };
    let anchor = anchor(&prep.anchor);
    let continuation = prep.continuation.as_ref().map(previous);
    let digest = hearing_result_values_digest(hasher().as_ref(), &values);
    let admin = prep.administration.snapshot().unwrap();
    HearingResultDetail {
        snapshot: HearingResultSnapshot {
            case_id: prep.case_id,
            hearing_id: command.hearing_id,
            id: command.result_id,
            revision: command.result_revision().unwrap(),
            values,
            values_digest: digest,
            status: if command.action() == HearingResultAction::Withdraw {
                HearingResultStatus::Withdrawn
            } else {
                HearingResultStatus::Recorded
            },
            reason: command.reason().cloned(),
            receipt: HearingResultReceipt {
                operation_id: command.operation_id,
                action: command.action(),
                expected_revision: command.expected_revision(),
                submission_digest: hearing_result_submission_digest(
                    hasher().as_ref(),
                    actor,
                    prep.case_id,
                    command,
                    &anchor.reference,
                    continuation.as_ref().map(|v| &v.reference),
                    digest,
                ),
            },
            anchor: anchor.reference,
            continuation: continuation.map(|v| v.reference),
            recorded_administration_revision: admin.revision,
            recorded_administration_digest: admin.values_digest,
            recorded_at: instant(),
            recorded_by: application::cases::CaseActorSnapshot {
                id: actor,
                email: "actor@example.com".into(),
            },
        },
        anchor,
        continuation,
        attendees: prep.attendees.clone(),
        support: None,
    }
}
pub fn correction(base: &HearingResultDetail) -> HearingResultCommand {
    HearingResultCommand {
        operation_id: HearingResultOperationId::new(),
        hearing_id: base.snapshot.hearing_id,
        result_id: base.snapshot.id,
        change: HearingResultChange::Correct {
            expected_revision: base.snapshot.revision,
            values: base.snapshot.values.clone(),
            reason: HearingResultText::new("Corrected declaration").unwrap(),
        },
    }
}
pub fn withdrawal(base: &HearingResultDetail) -> HearingResultCommand {
    HearingResultCommand {
        operation_id: HearingResultOperationId::new(),
        hearing_id: base.snapshot.hearing_id,
        result_id: base.snapshot.id,
        change: HearingResultChange::Withdraw {
            expected_revision: base.snapshot.revision,
            reason: HearingResultText::new("Administrative withdrawal").unwrap(),
        },
    }
}
pub fn service(
    store: MockStore,
    identity: MockIdentity,
    validator: Arc<Validator>,
) -> (HearingResultService, Arc<CountingClock>) {
    let clock = Arc::new(CountingClock::default());
    (
        HearingResultService::new(
            Arc::new(store),
            Arc::new(identity),
            Arc::new(crate::crypto::processor()),
            validator,
            hasher(),
            clock.clone(),
        ),
        clock,
    )
}
