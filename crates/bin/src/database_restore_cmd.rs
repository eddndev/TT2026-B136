//! Offline validation and owner-only cancellation of restored identity capabilities.

use crate::database_args::InvalidateRestoredPasswordResetsArgs;
use anyhow::{anyhow, bail};
use application::ApplicationError;
use infrastructure::identity::{
    invalidate_restored_invitations, invalidate_restored_password_resets, PasswordResetRestoreHead,
    PasswordResetRestoreRequest, PasswordResetRestoreResult,
};

pub(super) fn check(database_url: &str, json: bool) -> anyhow::Result<()> {
    infrastructure::with_validated_postgres(database_url, |_| Ok::<(), ApplicationError>(()))
        .map_err(|_| anyhow!("database runtime validation failed"))?;
    if json {
        println!("{}", serde_json::json!({"validated":true}));
    } else {
        println!("database runtime schema, authority and inventory validated");
    }
    Ok(())
}

pub(super) fn invalidate(
    database_url: &str,
    args: InvalidateRestoredPasswordResetsArgs,
    json: bool,
) -> anyhow::Result<()> {
    invalidate_with(
        database_url,
        args,
        json,
        invalidate_restored_password_resets,
    )
}

pub(super) fn invalidate_invitations(
    database_url: &str,
    args: InvalidateRestoredPasswordResetsArgs,
    json: bool,
) -> anyhow::Result<()> {
    invalidate_with(database_url, args, json, invalidate_restored_invitations)
}

fn invalidate_with(
    database_url: &str,
    args: InvalidateRestoredPasswordResetsArgs,
    json: bool,
    operation: fn(
        &str,
        PasswordResetRestoreRequest,
    ) -> Result<PasswordResetRestoreResult, ApplicationError>,
) -> anyhow::Result<()> {
    let expected_head = match (
        args.expected_empty_audit,
        args.expected_audit_sequence,
        args.expected_audit_head,
    ) {
        (true, None, None) => None,
        (false, Some(sequence), Some(chain)) => Some(PasswordResetRestoreHead { sequence, chain }),
        _ => bail!("restoration requires exactly one explicit audit predecessor"),
    };
    let operation_id = args.operation_id;
    let result = operation(
        database_url,
        PasswordResetRestoreRequest {
            operation_id,
            expected_database: args.expected_database,
            expected_schema: args.expected_schema,
            expected_head,
        },
    )
    // Storage errors may follow a commit; callers retain the same request for retry.
    .map_err(|_| anyhow!("identity capability restoration validation or execution failed; retain the original request for retry"))?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "operation_id":operation_id.to_string(),
                "applied":result.applied,
                "invalidated":result.invalidated,
                "audit_sequence":result.audit_sequence,
                "audit_head":result.audit_head.to_hex(),
            })
        );
    } else {
        println!(
            "restoration {operation_id}: applied={}, invalidated={}, audit_sequence={}",
            result.applied, result.invalidated, result.audit_sequence,
        );
        println!("audit SHA-256: {}", result.audit_head.to_hex());
    }
    Ok(())
}
