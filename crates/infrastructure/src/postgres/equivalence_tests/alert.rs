//! Mutation corpus of the alert catalog and runtime role checks.

use super::{legacy_verdicts, Check};
use crate::postgres::mutation_corpus::{ColumnSpec, ModuleSpec};
use crate::postgres::CatalogClient;
use application::ApplicationError;

#[derive(Clone, Copy, Debug)]
enum Alert {
    Catalog,
    RuntimeRole,
}

impl Check for Alert {
    fn run<C: CatalogClient>(self, client: &mut C, role: &str) -> Result<(), ApplicationError> {
        match self {
            Self::Catalog => crate::alert_schema::validate(client),
            Self::RuntimeRole => crate::alert_schema::validate_runtime_role(client, role),
        }
    }
}

const SPEC: ModuleSpec = ModuleSpec {
    tables: &[
        "alert_preferences",
        "alert_scan_cursor",
        "alert_email_attempts",
    ],
    columns: &[
        ColumnSpec {
            table: "alert_preferences",
            column: "recorded_nanos",
            default: "0",
            retype: "smallint",
        },
        ColumnSpec {
            table: "alert_schedule",
            column: "occurrence_key",
            default: "'occurrence'",
            retype: "varchar(200)",
        },
    ],
    checks: &[
        ("alert_preferences", "alert_preferences_revision_check"),
        (
            "alert_email_attempts",
            "alert_email_attempts_sequence_check",
        ),
    ],
    foreign_keys: &[
        ("alert_read_receipts", "alert_read_receipts_recipient_fkey"),
        ("alert_schedule", "alert_schedule_kind_subject_id_fkey"),
    ],
    indexes: &[
        (
            "alert_notifications",
            "alert_inbox_order",
            &["recipient", "created_seconds", "created_nanos", "id"],
        ),
        (
            "alert_schedule",
            "alert_schedule_subject",
            &["kind", "subject_id", "recipient"],
        ),
    ],
    functions: &[
        "alert_time_valid(bigint,integer)",
        "alert_payload_valid(bytea,bytea)",
        "preserve_alert_rows()",
    ],
    triggers: &[
        ("alert_schedule", "alert_lock"),
        ("alert_notifications", "alert_update"),
        ("alert_email_attempts", "alert_attempt_insert"),
    ],
    trigger_function: "lock_alert_mutations",
    protected_columns: &[("alert_notifications", "payload")],
    private_functions: &["preserve_alert_rows()"],
};

#[test]
fn legacy_alert_validation_is_equivalent_through_every_client_and_rejects_the_corpus() {
    let Some(accepted) = legacy_verdicts("alert", &SPEC, &[Alert::Catalog, Alert::RuntimeRole])
    else {
        return;
    };
    assert_eq!(accepted, ["unaltered"]);
}
