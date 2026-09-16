use super::inconsistent;
use application::{procedural_facts::*, ApplicationError};
use uuid::Uuid;

pub(super) fn parts(target: FactTarget) -> (&'static str, Uuid, Option<Uuid>) {
    match target {
        FactTarget::Resolution(id) => ("resolution", id.as_uuid(), None),
        FactTarget::Notification { id, resolution_id } => {
            ("notification", id.as_uuid(), Some(resolution_id.as_uuid()))
        }
    }
}
pub(super) fn action(value: FactAction) -> &'static str {
    match value {
        FactAction::Record => "record",
        FactAction::Correct => "correct",
        FactAction::Withdraw => "withdraw",
    }
}
pub(super) fn status(value: FactStatus) -> &'static str {
    match value {
        FactStatus::Recorded => "recorded",
        FactStatus::Withdrawn => "withdrawn",
    }
}
pub(super) fn canonical(values: &ProceduralFactValues) -> Vec<u8> {
    match values {
        ProceduralFactValues::Resolution(v) => v.canonical_bytes(),
        ProceduralFactValues::Notification(v) => v.canonical_bytes(),
    }
}
pub(super) fn command(
    snapshot: &ProceduralFactSnapshot,
) -> Result<ProceduralFactCommand, ApplicationError> {
    fn change<V: Clone>(
        m: &FactRevisionMetadata,
        values: &V,
    ) -> Result<FactChange<V>, ApplicationError> {
        Ok(match m.receipt.action {
            FactAction::Record => FactChange::record(values.clone()),
            FactAction::Correct => FactChange::correct(
                FactRevision::new(m.receipt.expected_revision).map_err(inconsistent)?,
                values.clone(),
                m.reason
                    .clone()
                    .ok_or_else(|| inconsistent("correction reason absent"))?,
            ),
            FactAction::Withdraw => FactChange::withdraw(
                FactRevision::new(m.receipt.expected_revision).map_err(inconsistent)?,
                m.reason
                    .clone()
                    .ok_or_else(|| inconsistent("withdrawal reason absent"))?,
            ),
        })
    }
    Ok(match snapshot {
        ProceduralFactSnapshot::Resolution(v) => {
            ProceduralFactCommand::Resolution(ResolutionCommand::new(
                v.metadata.receipt.operation_id,
                v.root.id(),
                change(&v.metadata, &v.values)?,
            ))
        }
        ProceduralFactSnapshot::Notification(v) => ProceduralFactCommand::Notification(
            NotificationCommand::new(
                v.metadata.receipt.operation_id,
                v.root.id(),
                v.root.resolution_id(),
                change(&v.metadata, &v.values)?,
            )
            .map_err(inconsistent)?,
        ),
    })
}
pub(super) fn projection(snapshot: &ProceduralFactSnapshot) -> serde_json::Value {
    let m = snapshot.metadata();
    let (family, id, parent) = parts(snapshot.target());
    serde_json::json!({"family":family,"target_id":id,"parent_id":parent,"operation_id":m.receipt.operation_id.to_string(),
        "actor_id":m.recorded_by.id.to_string(),"case_id":snapshot.case_id().to_string(),"action":action(m.receipt.action),
        "expected_revision":m.receipt.expected_revision,"values_digest":m.values_digest.to_hex(),
        "sources_digest":m.receipt.sources_digest.to_hex(),"reason":m.reason.as_ref().map(FactText::as_str)})
}
