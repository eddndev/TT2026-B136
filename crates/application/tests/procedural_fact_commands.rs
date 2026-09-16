use application::procedural_facts::*;
use domain::procedural_time::DeclaredProceduralTime;
use uuid::Uuid;

fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
fn resolution_id() -> ResolutionId {
    ResolutionId::from_uuid(Uuid::from_u128(1))
}
fn notification_id() -> NotificationId {
    NotificationId::from_uuid(Uuid::from_u128(2))
}
fn operation() -> FactOperationId {
    FactOperationId::from_uuid(Uuid::from_u128(3))
}
fn resolution_values() -> ResolutionValues {
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
fn notification_input(parent: ResolutionId, revision: u32) -> NotificationValuesInput {
    NotificationValuesInput {
        resolution: FactResolutionRef {
            id: parent,
            revision: FactRevision::new(revision).unwrap(),
        },
        character: FactDeclaration::Known(NotificationCharacter::Personal),
        medium: FactDeclaration::Known(NotificationMedium::InPerson),
        context: FactDeclaration::Known(NotificationContext::OutsideHearing),
        outcome: FactDeclaration::Known(NotificationOutcome::Attempted),
        subtype: None,
        practiced_at: DeclaredProceduralTime::unknown(),
        received_at: None,
        stated_effect: None,
        intended_recipient: FactDeclaration::Unknown(text("Recipient not stated")),
        actual_receiver: FactDeclaration::Unknown(text("Receiver not stated")),
        representation: FactRepresentation::NotRecorded(text("No declaration")),
        summary: text("Declared practice"),
        provenance: FactProvenance::OperatorNote {
            note: text("Operator declaration"),
        },
    }
}
fn notification_values(parent: ResolutionId, revision: u32) -> NotificationValues {
    NotificationValues::new(notification_input(parent, revision)).unwrap()
}

#[test]
fn record_has_no_base_and_proposes_revision_one() {
    let command = ResolutionCommand::new(
        operation(),
        resolution_id(),
        FactChange::record(resolution_values()),
    );
    assert_eq!(command.action(), FactAction::Record);
    assert_eq!(command.expected_revision(), 0);
    assert_eq!(command.result_revision().unwrap(), FactRevision::initial());
    assert_eq!(command.reason(), None);
    assert_eq!(command.validate_base(None), Ok(()));
    assert_eq!(command.operation_id(), operation());
}

#[test]
fn correction_keeps_its_expected_revision_values_and_reason() {
    let revision = FactRevision::new(7).unwrap();
    let values = resolution_values();
    let reason = text("Correct the declared issuer");
    let command = ResolutionCommand::new(
        operation(),
        resolution_id(),
        FactChange::correct(revision, values.clone(), reason.clone()),
    );
    assert_eq!(command.action(), FactAction::Correct);
    assert_eq!(command.expected_revision(), 7);
    assert_eq!(command.result_revision().unwrap().get(), 8);
    assert_eq!(command.change().values(), Some(&values));
    assert_eq!(command.reason(), Some(&reason));
    assert_eq!(
        command.validate_base(Some((revision, FactStatus::Recorded))),
        Ok(())
    );
}

#[test]
fn withdrawal_accepts_no_replacement_values() {
    let command = ResolutionCommand::new(
        operation(),
        resolution_id(),
        FactChange::withdraw(FactRevision::initial(), text("Duplicate declaration")),
    );
    assert_eq!(command.action(), FactAction::Withdraw);
    assert_eq!(command.change().values(), None);
    assert_eq!(command.result_revision().unwrap().get(), 2);
    assert_eq!(command.action().resulting_status(), FactStatus::Withdrawn);
}

#[test]
fn exhausted_revisions_never_wrap_for_correction_or_withdrawal() {
    let maximum = FactRevision::new(u32::MAX).unwrap();
    for change in [
        FactChange::correct(maximum, resolution_values(), text("Correction")),
        FactChange::withdraw(maximum, text("Withdrawal")),
    ] {
        let command = ResolutionCommand::new(operation(), resolution_id(), change);
        assert_eq!(
            command.result_revision(),
            Err(ProceduralFactError::RevisionExhausted)
        );
        assert_eq!(
            command.validate_base(Some((maximum, FactStatus::Recorded))),
            Err(ProceduralFactError::RevisionExhausted)
        );
    }
}

#[test]
fn withdrawn_roots_reject_correction_and_another_withdrawal() {
    let revision = FactRevision::new(2).unwrap();
    for change in [
        FactChange::correct(revision, resolution_values(), text("Correction")),
        FactChange::withdraw(revision, text("Withdrawal")),
    ] {
        assert_eq!(
            change.validate_base(Some((revision, FactStatus::Withdrawn))),
            Err(ProceduralFactError::AlreadyWithdrawn)
        );
    }
}

#[test]
fn missing_existing_and_stale_bases_are_distinct() {
    let revision = FactRevision::initial();
    let record = FactChange::record(resolution_values());
    assert_eq!(
        record.validate_base(Some((revision, FactStatus::Recorded))),
        Err(ProceduralFactError::RevisionConflict)
    );
    let correction = FactChange::correct(revision, resolution_values(), text("Correction"));
    assert_eq!(
        correction.validate_base(None),
        Err(ProceduralFactError::NotFound)
    );
    assert_eq!(
        correction.validate_base(Some((revision.next().unwrap(), FactStatus::Recorded))),
        Err(ProceduralFactError::RevisionConflict)
    );
}

#[test]
fn notification_record_rejects_a_different_resolution_parent() {
    let other = ResolutionId::from_uuid(Uuid::from_u128(4));
    let result = NotificationCommand::new(
        operation(),
        notification_id(),
        resolution_id(),
        FactChange::record(notification_values(other, 1)),
    );
    assert_eq!(result, Err(ProceduralFactError::InvalidReference));
}

#[test]
fn notification_correction_allows_another_exact_revision_of_the_same_parent() {
    let values = notification_values(resolution_id(), 4);
    let command = NotificationCommand::new(
        operation(),
        notification_id(),
        resolution_id(),
        FactChange::correct(
            FactRevision::initial(),
            values.clone(),
            text("Reselect source"),
        ),
    )
    .unwrap();
    assert_eq!(command.resolution_id(), resolution_id());
    assert_eq!(command.change().values(), Some(&values));
    assert_eq!(
        command
            .change()
            .values()
            .unwrap()
            .resolution()
            .revision
            .get(),
        4
    );
}

#[test]
fn notification_correction_cannot_move_to_another_resolution_root() {
    let other = ResolutionId::from_uuid(Uuid::from_u128(4));
    let result = NotificationCommand::new(
        operation(),
        notification_id(),
        resolution_id(),
        FactChange::correct(
            FactRevision::initial(),
            notification_values(other, 2),
            text("Reselect source"),
        ),
    );
    assert_eq!(result, Err(ProceduralFactError::InvalidReference));
}

#[test]
fn notification_correction_preserves_replacement_person_and_provenance() {
    let mut input = notification_input(resolution_id(), 3);
    input.intended_recipient = FactDeclaration::Known(FactPerson::Unlinked {
        label: FactLabel::new("Corrected recipient").unwrap(),
        description: text("Recipient stated by the operator"),
    });
    input.provenance = FactProvenance::ExternalReference {
        reference: text("Corrected external reference"),
        support: None,
    };
    let replacement = NotificationValues::new(input).unwrap();
    let command = NotificationCommand::new(
        operation(),
        notification_id(),
        resolution_id(),
        FactChange::correct(
            FactRevision::initial(),
            replacement.clone(),
            text("Correct source and person"),
        ),
    )
    .unwrap();
    assert_eq!(command.change().values(), Some(&replacement));
    assert_ne!(
        command.change().values(),
        Some(&notification_values(resolution_id(), 1))
    );
}

#[test]
fn notification_withdrawal_keeps_parent_without_resubmitting_sources() {
    let command = NotificationCommand::new(
        operation(),
        notification_id(),
        resolution_id(),
        FactChange::withdraw(FactRevision::initial(), text("Withdraw duplicate")),
    )
    .unwrap();
    let command = ProceduralFactCommand::Notification(command);
    assert_eq!(
        command.target(),
        FactTarget::Notification {
            id: notification_id(),
            resolution_id: resolution_id(),
        }
    );
    assert_eq!(command.action(), FactAction::Withdraw);
    assert_eq!(command.expected_revision(), 1);
}

#[test]
fn equal_uuid_bytes_do_not_conflate_families() {
    let uuid = Uuid::nil();
    let resolution = FactTarget::Resolution(ResolutionId::from_uuid(uuid));
    let notification = FactTarget::Notification {
        id: NotificationId::from_uuid(uuid),
        resolution_id: resolution_id(),
    };
    assert_ne!(resolution, notification);
    assert_eq!(resolution.family(), FactFamily::Resolution);
    assert_eq!(notification.family(), FactFamily::Notification);
}

#[test]
fn list_and_history_queries_enforce_separate_finite_limits() {
    assert!(ResolutionQuery::new(100, None, FactStatusFilter::All).is_ok());
    assert!(NotificationQuery::new(100, None, FactStatusFilter::Recorded).is_ok());
    for limit in [0, 101, u32::MAX] {
        assert!(ResolutionQuery::new(limit, None, FactStatusFilter::All).is_err());
        assert!(NotificationQuery::new(limit, None, FactStatusFilter::All).is_err());
    }
    assert!(FactHistoryQuery::new(20, Some(1)).is_ok());
    for (limit, before) in [(0, None), (21, None), (20, Some(0))] {
        assert!(FactHistoryQuery::new(limit, before).is_err());
    }
}
