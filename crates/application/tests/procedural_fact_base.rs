mod procedural_fact_base_support;
use application::procedural_facts::*;
use domain::cases::CaseId;
use procedural_fact_base_support::*;
use uuid::Uuid;

#[test]
fn recording_without_a_base_is_valid_for_both_families() {
    let commands = [
        resolution_command(parent(), FactChange::record(resolution_values())),
        notification_command(
            notification_id(),
            parent(),
            FactChange::record(notification_values(parent())),
        ),
    ];
    for command in commands {
        assert!(validate_fact_base(case_id(), &command, None).is_ok());
    }
}

#[test]
fn matching_notification_uuid_and_revision_cannot_hide_a_changed_parent() {
    let base = notification_base(
        case_id(),
        notification_id(),
        parent(),
        4,
        FactStatus::Recorded,
    );
    let command = notification_command(
        notification_id(),
        other_parent(),
        FactChange::correct(
            revision(4),
            notification_values(other_parent()),
            text("Replace declared details"),
        ),
    );
    assert_inconsistent(validate_fact_base(case_id(), &command, Some(&base)));
}

#[test]
fn withdrawal_cannot_hide_a_changed_parent_even_when_the_base_is_withdrawn() {
    let command = notification_command(
        notification_id(),
        other_parent(),
        FactChange::withdraw(revision(4), text("Withdraw declaration")),
    );
    for status in [FactStatus::Recorded, FactStatus::Withdrawn] {
        let base = notification_base(case_id(), notification_id(), parent(), 4, status);
        assert_inconsistent(validate_fact_base(case_id(), &command, Some(&base)));
    }
}

#[test]
fn equal_uuid_bytes_do_not_allow_a_base_from_the_other_family() {
    let uuid = Uuid::nil();
    let resolution = ResolutionId::from_uuid(uuid);
    let notification = NotificationId::from_uuid(uuid);
    let cases = [
        (
            resolution_command(
                resolution,
                FactChange::withdraw(revision(4), text("Withdraw")),
            ),
            notification_base(case_id(), notification, parent(), 4, FactStatus::Recorded),
        ),
        (
            notification_command(
                notification,
                parent(),
                FactChange::withdraw(revision(4), text("Withdraw")),
            ),
            resolution_base(case_id(), resolution, 4, FactStatus::Recorded),
        ),
    ];
    for (command, base) in cases {
        assert_inconsistent(validate_fact_base(case_id(), &command, Some(&base)));
    }
}

#[test]
fn a_foreign_case_base_is_rejected_for_each_family_before_state_validation() {
    let foreign = CaseId::from_uuid(Uuid::from_u128(99));
    let cases = [
        (
            resolution_command(parent(), FactChange::record(resolution_values())),
            resolution_base(foreign, parent(), 4, FactStatus::Withdrawn),
        ),
        (
            notification_command(
                notification_id(),
                parent(),
                FactChange::record(notification_values(parent())),
            ),
            notification_base(
                foreign,
                notification_id(),
                parent(),
                4,
                FactStatus::Withdrawn,
            ),
        ),
    ];
    for (command, base) in cases {
        assert_inconsistent(validate_fact_base(case_id(), &command, Some(&base)));
    }
}

#[test]
fn a_different_root_uuid_is_rejected_before_revision_conflict() {
    let other_notification = NotificationId::from_uuid(Uuid::from_u128(99));
    let cases = [
        (
            resolution_command(
                parent(),
                FactChange::withdraw(revision(1), text("Withdraw")),
            ),
            resolution_base(case_id(), other_parent(), 4, FactStatus::Recorded),
        ),
        (
            notification_command(
                notification_id(),
                parent(),
                FactChange::withdraw(revision(1), text("Withdraw")),
            ),
            notification_base(
                case_id(),
                other_notification,
                parent(),
                4,
                FactStatus::Recorded,
            ),
        ),
    ];
    for (command, base) in cases {
        assert_inconsistent(validate_fact_base(case_id(), &command, Some(&base)));
    }
}

#[test]
fn correction_can_reselect_parent_revision_and_people_without_changing_parent_identity() {
    let base = notification_base(
        case_id(),
        notification_id(),
        parent(),
        4,
        FactStatus::Recorded,
    );
    let mut input = notification_input(parent());
    input.resolution.revision = revision(7);
    input.intended_recipient = FactDeclaration::Known(FactPerson::Unlinked {
        label: FactLabel::new("Corrected recipient").unwrap(),
        description: text("Declared recipient"),
    });
    input.actual_receiver = input.intended_recipient.clone();
    let values = NotificationValues::new(input).unwrap();
    let command = notification_command(
        notification_id(),
        parent(),
        FactChange::correct(revision(4), values, text("Correct exact source and people")),
    );
    assert!(validate_fact_base(case_id(), &command, Some(&base)).is_ok());
}

#[test]
fn exact_recorded_bases_accept_correction_and_withdrawal() {
    let resolution = resolution_base(case_id(), parent(), 4, FactStatus::Recorded);
    let notification = notification_base(
        case_id(),
        notification_id(),
        parent(),
        4,
        FactStatus::Recorded,
    );
    for change in [
        FactChange::correct(revision(4), resolution_values(), text("Correct")),
        FactChange::withdraw(revision(4), text("Withdraw")),
    ] {
        assert!(validate_fact_base(
            case_id(),
            &resolution_command(parent(), change),
            Some(&resolution)
        )
        .is_ok());
    }
    for change in [
        FactChange::correct(revision(4), notification_values(parent()), text("Correct")),
        FactChange::withdraw(revision(4), text("Withdraw")),
    ] {
        assert!(validate_fact_base(
            case_id(),
            &notification_command(notification_id(), parent(), change),
            Some(&notification)
        )
        .is_ok());
    }
}

#[test]
fn missing_bases_preserve_not_found_for_correction_and_withdrawal() {
    for change in [
        FactChange::correct(revision(1), resolution_values(), text("Correct")),
        FactChange::withdraw(revision(1), text("Withdraw")),
    ] {
        assert_error(
            validate_fact_base(case_id(), &resolution_command(parent(), change), None),
            ProceduralFactError::NotFound,
        );
    }
    for change in [
        FactChange::correct(revision(1), notification_values(parent()), text("Correct")),
        FactChange::withdraw(revision(1), text("Withdraw")),
    ] {
        assert_error(
            validate_fact_base(
                case_id(),
                &notification_command(notification_id(), parent(), change),
                None,
            ),
            ProceduralFactError::NotFound,
        );
    }
}

#[test]
fn recording_an_existing_root_preserves_revision_conflict() {
    let cases = [
        (
            resolution_command(parent(), FactChange::record(resolution_values())),
            resolution_base(case_id(), parent(), 1, FactStatus::Recorded),
        ),
        (
            notification_command(
                notification_id(),
                parent(),
                FactChange::record(notification_values(parent())),
            ),
            notification_base(
                case_id(),
                notification_id(),
                parent(),
                1,
                FactStatus::Recorded,
            ),
        ),
    ];
    for (command, base) in cases {
        assert_error(
            validate_fact_base(case_id(), &command, Some(&base)),
            ProceduralFactError::RevisionConflict,
        );
    }
}

#[test]
fn stale_revision_precedes_withdrawn_state_but_matching_withdrawn_bases_are_terminal() {
    for expected in [3, 4] {
        let resolution = resolution_base(case_id(), parent(), 4, FactStatus::Withdrawn);
        let notification = notification_base(
            case_id(),
            notification_id(),
            parent(),
            4,
            FactStatus::Withdrawn,
        );
        let commands = [
            (
                resolution_command(
                    parent(),
                    FactChange::correct(revision(expected), resolution_values(), text("Correct")),
                ),
                &resolution,
            ),
            (
                resolution_command(
                    parent(),
                    FactChange::withdraw(revision(expected), text("Withdraw")),
                ),
                &resolution,
            ),
            (
                notification_command(
                    notification_id(),
                    parent(),
                    FactChange::correct(
                        revision(expected),
                        notification_values(parent()),
                        text("Correct"),
                    ),
                ),
                &notification,
            ),
            (
                notification_command(
                    notification_id(),
                    parent(),
                    FactChange::withdraw(revision(expected), text("Withdraw")),
                ),
                &notification,
            ),
        ];
        for (command, base) in commands {
            let error = if expected == 4 {
                ProceduralFactError::AlreadyWithdrawn
            } else {
                ProceduralFactError::RevisionConflict
            };
            assert_error(validate_fact_base(case_id(), &command, Some(base)), error);
        }
    }
}

#[test]
fn exhausted_revision_rejects_correction_and_withdrawal_without_wrapping() {
    let resolution = resolution_base(case_id(), parent(), u32::MAX, FactStatus::Recorded);
    let notification = notification_base(
        case_id(),
        notification_id(),
        parent(),
        u32::MAX,
        FactStatus::Recorded,
    );
    let commands = [
        (
            resolution_command(
                parent(),
                FactChange::correct(revision(u32::MAX), resolution_values(), text("Correct")),
            ),
            &resolution,
        ),
        (
            resolution_command(
                parent(),
                FactChange::withdraw(revision(u32::MAX), text("Withdraw")),
            ),
            &resolution,
        ),
        (
            notification_command(
                notification_id(),
                parent(),
                FactChange::correct(
                    revision(u32::MAX),
                    notification_values(parent()),
                    text("Correct"),
                ),
            ),
            &notification,
        ),
        (
            notification_command(
                notification_id(),
                parent(),
                FactChange::withdraw(revision(u32::MAX), text("Withdraw")),
            ),
            &notification,
        ),
    ];
    for (command, base) in commands {
        assert_error(
            validate_fact_base(case_id(), &command, Some(base)),
            ProceduralFactError::RevisionExhausted,
        );
    }
}
