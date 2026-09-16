use application::{cases::*, procedural_facts::*};
use domain::{
    cases::{CaseId, CaseMetadata},
    crypto::Sha256Digest,
    identity::UserId,
    procedural_time::DeclaredProceduralTime,
};

pub(super) fn text(s: &str) -> FactText {
    FactText::new(s).unwrap()
}
pub(super) fn digest(n: u8) -> Sha256Digest {
    Sha256Digest::from_array([n; 32])
}
pub(super) fn values() -> ResolutionValues {
    ResolutionValues::new(ResolutionValuesInput {
        class: FactDeclaration::Known(ResolutionClass::Order),
        subtype: None,
        issuer: FactDeclaration::Unknown(text("Not identified")),
        issued_at: DeclaredProceduralTime::unknown(),
        summary: text("Declared order"),
        provenance: FactProvenance::OperatorNote {
            note: text("Operator statement"),
        },
    })
}
pub(super) fn empty_sources() -> FactSources {
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
pub(super) fn administration() -> CurrentCaseAdministration {
    CurrentCaseAdministration::Unrevised(CaseMetadata::new("Case title", "REF-1").unwrap())
}
pub(super) fn metadata() -> FactRevisionMetadata {
    FactRevisionMetadata {
        revision: FactRevision::initial(),
        values_digest: digest(1),
        status: FactStatus::Recorded,
        reason: None,
        receipt: FactReceipt {
            operation_id: FactOperationId::new(),
            action: FactAction::Record,
            expected_revision: 0,
            sources_digest: digest(2),
            submission_digest: digest(3),
        },
        recorded_administration: administration(),
        recorded_at: time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
        recorded_by: CaseActorSnapshot {
            id: UserId::new(),
            email: "staff@example.com".into(),
        },
    }
}
pub(super) fn detail() -> FactDetail {
    FactDetail {
        snapshot: ProceduralFactSnapshot::Resolution(Box::new(ResolutionSnapshot {
            root: ResolutionRoot::new(ResolutionId::new(), CaseId::new()),
            metadata: metadata(),
            values: values(),
        })),
        sources: empty_sources(),
    }
}
pub(super) fn notification() -> FactDetail {
    let case = CaseId::new();
    let parent = FactResolutionRef {
        id: ResolutionId::new(),
        revision: FactRevision::initial(),
    };
    let values = NotificationValues::new(NotificationValuesInput {
        resolution: parent,
        character: FactDeclaration::Unknown(text("Unknown character")),
        medium: FactDeclaration::Unknown(text("Unknown medium")),
        context: FactDeclaration::Unknown(text("Unknown context")),
        outcome: FactDeclaration::Known(NotificationOutcome::Attempted),
        subtype: None,
        practiced_at: DeclaredProceduralTime::unknown(),
        received_at: Some(DeclaredProceduralTime::unknown()),
        stated_effect: None,
        intended_recipient: FactDeclaration::Unknown(text("Unknown recipient")),
        actual_receiver: FactDeclaration::Unknown(text("Unknown receiver")),
        representation: FactRepresentation::NotRecorded(text("Not declared")),
        summary: text("Declared attempt"),
        provenance: FactProvenance::OperatorNote {
            note: text("Operator statement"),
        },
    })
    .unwrap();
    let mut sources = empty_sources();
    sources.resolved.resolution = Some(FactResolutionSourceSnapshot {
        case_id: case,
        reference: parent,
        values_digest: digest(4),
        submission_digest: digest(5),
        status: FactStatus::Withdrawn,
    });
    let parent_values = self::values();
    sources.views.resolution = Some(FactResolutionView {
        reference: parent,
        class: parent_values.class().clone(),
        issuer: parent_values.issuer().clone(),
        issued_at: parent_values.issued_at(),
        summary: parent_values.summary().clone(),
    });
    FactDetail {
        snapshot: ProceduralFactSnapshot::Notification(Box::new(NotificationSnapshot {
            root: NotificationRoot::new(NotificationId::new(), case, parent.id),
            metadata: metadata(),
            values,
        })),
        sources,
    }
}
pub(super) fn draft() -> FactDraft {
    let command = ProceduralFactCommand::Resolution(ResolutionCommand::new(
        FactOperationId::new(),
        ResolutionId::new(),
        FactChange::record(values()),
    ));
    FactDraft {
        case_id: CaseId::new(),
        actor: UserId::new(),
        command,
        result_revision: FactRevision::initial(),
        values: ProceduralFactValues::Resolution(Box::new(values())),
        values_digest: digest(1),
        sources: empty_sources(),
        sources_digest: digest(2),
        submission_digest: digest(3),
        observed_administration: administration(),
    }
}
pub(super) fn meta(row: &mut FactDetail) -> &mut FactRevisionMetadata {
    match &mut row.snapshot {
        ProceduralFactSnapshot::Resolution(v) => &mut v.metadata,
        ProceduralFactSnapshot::Notification(v) => &mut v.metadata,
    }
}
pub(super) fn resolution_overview(row: &FactDetail) -> ResolutionOverview {
    let ProceduralFactSnapshot::Resolution(snapshot) = &row.snapshot else {
        panic!("resolution required")
    };
    ResolutionOverview {
        root: snapshot.root,
        revision: snapshot.metadata.revision,
        status: snapshot.metadata.status,
        class: snapshot.values.class().clone(),
        issued_at: snapshot.values.issued_at(),
    }
}
pub(super) fn notification_overview(row: &FactDetail) -> NotificationOverview {
    let ProceduralFactSnapshot::Notification(snapshot) = &row.snapshot else {
        panic!("notification required")
    };
    NotificationOverview {
        root: snapshot.root,
        revision: snapshot.metadata.revision,
        status: snapshot.metadata.status,
        resolution: snapshot.values.resolution(),
        outcome: snapshot.values.outcome().clone(),
        practiced_at: snapshot.values.practiced_at(),
    }
}
