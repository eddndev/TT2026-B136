use super::*;
#[path = "../procedural_fact_base_support/mod.rs"]
mod base;

pub fn notification(revision: u32, withdrawn: bool, parent_revision: u32, at: &str) -> FactDetail {
    let parent = resolution(parent_revision, false, "2026-01-01");
    let ProceduralFactSnapshot::Resolution(parent_snapshot) = parent.snapshot else {
        unreachable!()
    };
    let mut input = base::notification_input(parent_snapshot.root.id());
    input.resolution.revision = parent_snapshot.metadata.revision;
    input.practiced_at = date(at);
    let values = NotificationValues::new(input).unwrap();
    let selected = FactSourceSelection::from_values(&ProceduralFactValues::Notification(Box::new(
        values.clone(),
    )));
    let projection = resolve_fact_resolution(
        hasher().as_ref(),
        case_id(),
        &selected,
        Some(&ResolutionSourceMaterial {
            snapshot: *parent_snapshot,
            hearing: None,
            admitted_support: None,
        }),
    )
    .unwrap()
    .unwrap();
    let mut sources = facts::empty();
    sources.resolved.resolution = Some(projection.snapshot);
    sources.views.resolution = Some(projection.view);
    let mut snapshot = base::notification_base(
        case_id(),
        NotificationId::from_uuid(Uuid::from_u128(11)),
        values.resolution().id,
        revision,
        if withdrawn {
            FactStatus::Withdrawn
        } else {
            FactStatus::Recorded
        },
    );
    let ProceduralFactSnapshot::Notification(s) = &mut snapshot else {
        unreachable!()
    };
    s.values = values;
    s.metadata.recorded_by.id = actor();
    let mut detail = FactDetail { snapshot, sources };
    resign_fact(&mut detail);
    fact_receipt_matches(hasher().as_ref(), &detail).unwrap();
    detail
}
fn change<V: Clone>(metadata: &FactRevisionMetadata, values: &V) -> FactChange<V> {
    match metadata.receipt.action {
        FactAction::Record => FactChange::record(values.clone()),
        FactAction::Correct => FactChange::correct(
            FactRevision::new(metadata.receipt.expected_revision).unwrap(),
            values.clone(),
            metadata.reason.clone().unwrap(),
        ),
        FactAction::Withdraw => FactChange::withdraw(
            FactRevision::new(metadata.receipt.expected_revision).unwrap(),
            metadata.reason.clone().unwrap(),
        ),
    }
}
/// Rebuilds existing public encodings so identity negatives stay self-consistent.
pub fn resign_fact(detail: &mut FactDetail) {
    let command = match &detail.snapshot {
        ProceduralFactSnapshot::Resolution(s) => {
            ProceduralFactCommand::Resolution(ResolutionCommand::new(
                s.metadata.receipt.operation_id,
                s.root.id(),
                change(&s.metadata, &s.values),
            ))
        }
        ProceduralFactSnapshot::Notification(s) => ProceduralFactCommand::Notification(
            NotificationCommand::new(
                s.metadata.receipt.operation_id,
                s.root.id(),
                s.root.resolution_id(),
                change(&s.metadata, &s.values),
            )
            .unwrap(),
        ),
    };
    let values_digest = fact_values_digest(hasher().as_ref(), &detail.snapshot.values());
    let sources_digest = fact_sources_digest(hasher().as_ref(), &detail.sources).unwrap();
    let submission_digest = fact_submission_digest(
        hasher().as_ref(),
        detail.snapshot.metadata().recorded_by.id,
        detail.snapshot.case_id(),
        &command,
        values_digest,
        sources_digest,
    )
    .unwrap();
    let metadata = metadata_mut(detail);
    metadata.values_digest = values_digest;
    metadata.receipt.sources_digest = sources_digest;
    metadata.receipt.submission_digest = submission_digest;
}

pub fn notification_input(v: &NotificationValues) -> NotificationValuesInput {
    NotificationValuesInput {
        resolution: v.resolution(),
        character: v.character().clone(),
        medium: v.medium().clone(),
        context: v.context().clone(),
        outcome: v.outcome().clone(),
        subtype: v.subtype().cloned(),
        practiced_at: v.practiced_at(),
        received_at: v.received_at(),
        stated_effect: v.stated_effect().cloned(),
        intended_recipient: v.intended_recipient().clone(),
        actual_receiver: v.actual_receiver().clone(),
        representation: v.representation().clone(),
        summary: v.summary().clone(),
        provenance: v.provenance().clone(),
    }
}
