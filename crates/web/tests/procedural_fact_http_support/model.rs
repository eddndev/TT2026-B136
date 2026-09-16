use super::*;
use application::cases::{CaseActorSnapshot, CurrentCaseAdministration};
use domain::{cases::CaseMetadata, clock::OffsetDateTime};

pub fn administration() -> CurrentCaseAdministration {
    CurrentCaseAdministration::Unrevised(CaseMetadata::new("Historical case", "BASE-1").unwrap())
}
pub fn sources(scope: CaseId, values: &ProceduralFactValues) -> FactSources {
    let mut out = FactSources {
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
    };
    if let ProceduralFactValues::Notification(v) = values {
        let parent = value_factory::resolution(&values_json("resolution"));
        out.resolved.resolution = Some(FactResolutionSourceSnapshot {
            case_id: scope,
            reference: v.resolution(),
            values_digest: digest(),
            submission_digest: digest(),
            status: FactStatus::Withdrawn,
        });
        out.views.resolution = Some(FactResolutionView {
            reference: v.resolution(),
            class: parent.class().clone(),
            issuer: parent.issuer().clone(),
            issued_at: parent.issued_at(),
            summary: parent.summary().clone(),
        });
    }
    out
}
pub fn command_values(command: &ProceduralFactCommand) -> ProceduralFactValues {
    match command {
        ProceduralFactCommand::Resolution(c) => ProceduralFactValues::Resolution(Box::new(
            c.change()
                .values()
                .cloned()
                .unwrap_or_else(|| value_factory::resolution(&values_json("resolution"))),
        )),
        ProceduralFactCommand::Notification(c) => {
            let mut value = values_json("notification");
            value["resolution"]["id"] = json!(c.resolution_id().to_string());
            ProceduralFactValues::Notification(Box::new(
                c.change()
                    .values()
                    .cloned()
                    .unwrap_or_else(|| value_factory::notification(&value)),
            ))
        }
    }
}
pub fn detail(scope: CaseId, command: &ProceduralFactCommand) -> FactDetail {
    let values = command_values(command);
    let metadata = FactRevisionMetadata {
        revision: command.result_revision().unwrap(),
        values_digest: digest(),
        status: command.action().resulting_status(),
        reason: command.reason().cloned(),
        receipt: FactReceipt {
            operation_id: command.operation_id(),
            action: command.action(),
            expected_revision: command.expected_revision(),
            sources_digest: digest(),
            submission_digest: digest(),
        },
        recorded_administration: administration(),
        recorded_at: OffsetDateTime::UNIX_EPOCH,
        recorded_by: CaseActorSnapshot {
            id: actor(),
            email: "historical@example.test".into(),
        },
    };
    let sources = sources(scope, &values);
    let snapshot = match (command.target(), values) {
        (FactTarget::Resolution(id), ProceduralFactValues::Resolution(values)) => {
            ProceduralFactSnapshot::Resolution(Box::new(ResolutionSnapshot {
                root: ResolutionRoot::new(id, scope),
                metadata,
                values: *values,
            }))
        }
        (
            FactTarget::Notification { id, resolution_id },
            ProceduralFactValues::Notification(values),
        ) => ProceduralFactSnapshot::Notification(Box::new(NotificationSnapshot {
            root: NotificationRoot::new(id, scope, resolution_id),
            metadata,
            values: *values,
        })),
        _ => panic!("fixture target family"),
    };
    FactDetail { snapshot, sources }
}
pub fn target_command(target: FactTarget) -> ProceduralFactCommand {
    let mut c = command_json(
        if matches!(target, FactTarget::Resolution(_)) {
            "resolution"
        } else {
            "notification"
        },
        "record",
    );
    let identity = target_json(target);
    c["id"] = identity["id"].clone();
    if let FactTarget::Notification { resolution_id, .. } = target {
        c["resolution_id"] = json!(resolution_id.to_string());
        c["change"]["values"]["resolution"]["id"] = c["resolution_id"].clone();
    }
    typed_command(&c)
}
pub fn set_revision(detail: &mut FactDetail, revision: FactRevision) {
    let metadata = match &mut detail.snapshot {
        ProceduralFactSnapshot::Resolution(s) => &mut s.metadata,
        ProceduralFactSnapshot::Notification(s) => &mut s.metadata,
    };
    metadata.revision = revision;
    metadata.receipt.expected_revision = revision.get() - 1;
    if revision.get() > 1 {
        metadata.receipt.action = FactAction::Correct;
        metadata.reason = Some(text("Historical correction"));
    }
}
