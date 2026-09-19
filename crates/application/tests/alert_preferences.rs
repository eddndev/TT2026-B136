use application::{alerts::*, ApplicationError};
use domain::alerts::{AlertAnticipations, AlertLeadHours, AlertTimingError};
use domain::identity::UserId;
use time::macros::datetime;
use uuid::Uuid;

#[test]
fn defaults_enable_both_channels_and_48_24_hours_for_both_families() {
    let values = AlertPreferenceValues::default();
    for family in [&values.hearing_upcoming, &values.deadline_upcoming] {
        assert_eq!(family.anticipations, AlertAnticipations::default());
        assert_eq!(
            family.channels,
            AlertChannels {
                internal: true,
                email: true
            }
        );
    }
    for channels in [
        values.overdue_unattended,
        values.review_required,
        values.due_changed_soon,
    ] {
        assert!(channels.internal && channels.email);
    }
}

#[test]
fn implicit_preferences_require_defaults_and_no_mutation_receipt() {
    let actor = UserId::new();
    let mut preferences = AlertPreferences::initial(actor, AlertEmailTransport::Disabled);
    assert!(preferences.validate(actor).is_ok());
    preferences.values.deadline_upcoming.anticipations =
        AlertAnticipations::new(vec![AlertLeadHours::new(12).unwrap()]).unwrap();
    assert!(preferences.validate(actor).is_err());
}

#[test]
fn persisted_preferences_bind_owner_revision_receipt_and_timestamp() {
    let actor = UserId::new();
    let command = AlertPreferenceCommand {
        operation_id: AlertOperationId::from_uuid(Uuid::from_u128(1)),
        expected_revision: 2,
        values: AlertPreferenceValues::default(),
    };
    let value = AlertPreferences {
        user_id: actor,
        revision: 3,
        values: command.values.clone(),
        updated_at: Some(datetime!(2026-09-01 12:00 UTC)),
        receipt: Some(AlertPreferenceReceipt {
            operation_id: command.operation_id,
            expected_revision: command.expected_revision,
        }),
        email_transport: AlertEmailTransport::Ready,
    };
    assert!(value.validate_command(actor, &command).is_ok());
    for field in 0..5 {
        let mut invalid = value.clone();
        match field {
            0 => invalid.user_id = UserId::new(),
            1 => invalid.revision = 4,
            2 => {
                invalid.receipt.as_mut().unwrap().operation_id =
                    AlertOperationId::from_uuid(Uuid::from_u128(2))
            }
            3 => invalid.updated_at = None,
            _ => invalid.values.review_required.email = false,
        }
        assert!(
            invalid.validate_command(actor, &command).is_err(),
            "field {field}"
        );
    }
}

#[test]
fn domain_timing_errors_are_input_errors_and_stored_errors_remain_distinct() {
    let invalid: ApplicationError = AlertError::from(AlertTimingError::InvalidLeadHours).into();
    assert!(matches!(
        invalid,
        ApplicationError::Alert(AlertError::Timing(_))
    ));
    let stored: ApplicationError = AlertError::Stored("corrupt receipt".into()).into();
    assert!(matches!(
        stored,
        ApplicationError::Alert(AlertError::Stored(_))
    ));
}
