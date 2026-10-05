//! Shared PostgreSQL connection and schema initialization boundary.

use crate::postgres_connection::{port_error, require_utf8};
use application::ApplicationError;
use postgres::{Client, NoTls};

mod migration_lock;
mod migrations;
mod runtime_role;
use migrations::*;
use runtime_role::validate_runtime_role;

/// Applies all schema prerequisites atomically, serialized per schema.
pub(crate) fn connect(database_url: &str) -> Result<Client, ApplicationError> {
    let mut client = Client::connect(database_url, NoTls).map_err(port_error)?;
    require_utf8(&mut client)?;
    let mut transaction = client.transaction().map_err(port_error)?;
    migration_lock::acquire(&mut transaction)?;
    transaction
        .batch_execute(IDENTITY_MIGRATION)
        .map_err(port_error)?;
    transaction
        .batch_execute(CASE_MIGRATION)
        .map_err(port_error)?;
    transaction
        .batch_execute(DOCUMENT_MIGRATION)
        .map_err(port_error)?;
    transaction
        .batch_execute(VERSION_MIGRATION)
        .map_err(port_error)?;
    transaction
        .batch_execute(METADATA_MIGRATION)
        .map_err(port_error)?;
    transaction
        .batch_execute(PARTICIPANT_MIGRATION)
        .map_err(port_error)?;
    transaction
        .batch_execute(CASE_ADMINISTRATION_MIGRATION)
        .map_err(port_error)?;
    for migration in CASE_STAGE_MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    transaction
        .batch_execute(CREDENTIAL_TRUST_MIGRATION)
        .map_err(port_error)?;
    for migration in TYPED_PARTICIPANT_MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in HEARING_MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in HEARING_RESULT_MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in JUDICIAL_CALENDAR_MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in PROCEDURAL_FACT_MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    transaction
        .batch_execute(DEADLINE_SOURCE_EVENT_MIGRATION)
        .map_err(port_error)?;
    for migration in DEADLINE_PROFILE_MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in crate::deadline_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in crate::deadline_dispatch_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in crate::deadline_worker_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in crate::alert_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in crate::procedural_resource_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in crate::resource_activity_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in crate::document_integrity_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in crate::member_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in crate::case_report_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    transaction
        .batch_execute(crate::audit_query_schema::MIGRATION)
        .map_err(port_error)?;
    for migration in crate::password_reset_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in crate::owner_certificate_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in crate::resource_hearing_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in crate::precautionary_hearing_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    for migration in crate::hearing_derived_deadline_schema::MIGRATIONS {
        transaction.batch_execute(migration).map_err(port_error)?;
    }
    transaction
        .batch_execute(crate::alert_schema::RESOURCE_HEARING_MIGRATION)
        .map_err(port_error)?;
    crate::password_reset_schema::validate_schema(&mut transaction)?;
    crate::password_reset_schema::validate_inventory(&mut transaction)?;
    crate::owner_certificate_schema::validate_schema(&mut transaction)?;
    crate::owner_certificate_schema::validate_inventory(&mut transaction)?;
    crate::owner_certificate_postgres::validate_inventory(&mut transaction)?;
    transaction.commit().map_err(port_error)?;
    Ok(client)
}

/// Connects without DDL and rejects roles able to bypass persisted evidence protection.
pub(crate) fn open(
    source: &(impl crate::PostgresConnectionSource + ?Sized),
) -> Result<Client, ApplicationError> {
    crate::postgres_source::connect(source)
}

pub(crate) fn open_url(database_url: &str) -> Result<Client, ApplicationError> {
    let mut client = connect_runtime(database_url)?;
    validate_runtime(&mut client)?;
    Ok(client)
}

pub(crate) fn connect_runtime(database_url: &str) -> Result<Client, ApplicationError> {
    let mut client = Client::connect(database_url, NoTls).map_err(port_error)?;
    require_utf8(&mut client)?;
    Ok(client)
}

pub(crate) fn lock_startup_schema(client: &mut Client) -> Result<i32, ApplicationError> {
    migration_lock::acquire_startup(client)
}
pub(crate) fn unlock_startup_schema(
    client: &mut Client,
    schema: i32,
) -> Result<(), ApplicationError> {
    migration_lock::release_startup(client, schema)
}

pub(crate) fn validate_runtime(client: &mut Client) -> Result<(), ApplicationError> {
    crate::postgres_version_schema::validate(client)?;
    crate::postgres_metadata_schema::validate(client)?;
    crate::postgres_participant_schema::validate(client)?;
    crate::postgres_case_administration_schema::validate(client)?;
    crate::postgres_case_stages_schema::validate(client)?;
    crate::credential_trust_postgres::schema::validate(client)?;
    crate::typed_participant_schema::validate(client)?;
    crate::hearing_schema::validate(client)?;
    crate::hearing_result_schema::validate(client)?;
    crate::judicial_calendar_schema::validate(client)?;
    crate::procedural_fact_schema::validate(client)?;
    crate::deadline_profile_schema::validate(client)?;
    crate::deadline_schema::validate(client)?;
    crate::deadline_source_event_schema::validate(client)?;
    crate::deadline_dispatch_schema::validate(client)?;
    crate::deadline_worker_schema::validate(client)?;
    crate::alert_schema::validate(client)?;
    crate::procedural_resource_schema::validate(client)?;
    crate::resource_activity_schema::validate(client)?;
    crate::resource_hearing_schema::validate(client)?;
    crate::precautionary_hearing_schema::validate(client)?;
    crate::hearing_derived_deadline_schema::validate(client)?;
    crate::document_integrity_schema::validate(client)?;
    crate::member_schema::validate(client)?;
    crate::case_report_schema::validate(client)?;
    crate::audit_query_schema::validate(client)?;
    crate::password_reset_schema::validate_schema(client)?;
    crate::owner_certificate_schema::validate_schema(client)?;
    let role: String = client
        .query_one("SELECT current_user", &[])
        .map_err(port_error)?
        .get(0);
    validate_runtime_role(client, &role)?;
    crate::postgres_version_schema::validate_inventory(client)?;
    crate::postgres_metadata_schema::validate_inventory(client)?;
    crate::postgres_participant_schema::validate_inventory(client)?;
    crate::postgres_case_administration_inventory::validate(client)?;
    crate::postgres_case_stages_inventory::validate(client)?;
    crate::credential_trust_postgres::schema::validate_inventory(client)?;
    crate::typed_participant_schema::validate_inventory(client)?;
    crate::hearing_schema::validate_inventory(client)?;
    crate::hearing_result_schema::validate_inventory(client)?;
    crate::judicial_calendar_schema::validate_inventory(client)?;
    crate::procedural_fact_schema::validate_inventory(client)?;
    crate::deadline_profile_schema::validate_inventory(client)?;
    crate::deadline_schema::validate_inventory(client)?;
    crate::deadline_source_event_schema::validate_inventory(client)?;
    crate::deadline_dispatch_schema::validate_inventory(client)?;
    crate::deadline_worker_schema::validate_inventory(client)?;
    crate::alerts_postgres::validate_inventory(client)?;
    crate::procedural_resource_schema::validate_inventory(client)?;
    crate::resource_activity_schema::validate_inventory(client)?;
    crate::resource_hearing_schema::validate_inventory(client)?;
    crate::precautionary_hearing_schema::validate_inventory(client)?;
    crate::hearing_derived_deadline_schema::validate_inventory(client)?;
    crate::document_integrity_schema::validate_inventory(client)?;
    crate::member_schema::validate_inventory(client)?;
    crate::case_report_schema::validate_inventory(client)?;
    crate::audit_query_schema::validate_inventory(client)?;
    crate::password_reset_schema::validate_inventory(client)?;
    crate::owner_certificate_schema::validate_inventory(client)?;
    crate::owner_certificate_postgres::validate_inventory(client)?;
    Ok(())
}

/// Applies schema using administrative credentials and grants a pre-existing runtime role.
pub fn initialize_database(database_url: &str, runtime_role: &str) -> Result<(), ApplicationError> {
    let mut client = connect(database_url)?;
    let exists: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_roles WHERE rolname=$1)",
            &[&runtime_role],
        )
        .map_err(port_error)?
        .get(0);
    if !exists {
        return Err(ApplicationError::InvalidConfiguration(
            "runtime database role does not exist".into(),
        ));
    }
    let role: String = client
        .query_one("SELECT quote_ident($1)", &[&runtime_role])
        .map_err(port_error)?
        .get(0);
    let schema: String = client
        .query_one("SELECT quote_ident(current_schema())", &[])
        .map_err(port_error)?
        .get(0);
    let mut transaction = client.transaction().map_err(port_error)?;
    transaction
        .batch_execute(&format!(
            "GRANT USAGE ON SCHEMA {schema} TO {role};
         REVOKE ALL ON audit_events, documents, document_series, document_metadata_revisions, case_participants, case_participant_revisions, migration_receipts FROM {role};
         GRANT SELECT, INSERT ON audit_events TO {role};
         GRANT SELECT, INSERT ON documents TO {role};
         GRANT SELECT, INSERT ON document_series TO {role};
         GRANT SELECT, INSERT ON document_metadata_revisions TO {role};
         GRANT EXECUTE ON FUNCTION document_metadata_text_valid(TEXT,INTEGER),
             document_metadata_is_canonical(TEXT,TEXT,TEXT[]), document_metadata_bytes(TEXT,TEXT,TEXT[]) TO {role};
         GRANT SELECT, INSERT ON case_participants, case_participant_revisions TO {role};
         GRANT EXECUTE ON FUNCTION participant_text_valid(TEXT,INTEGER),
             participant_values_is_canonical(TEXT,TEXT,TEXT,TEXT,TEXT),
             participant_values_bytes(TEXT,TEXT,TEXT,TEXT,TEXT) TO {role};
         GRANT UPDATE(evidence) ON documents TO {role};
         GRANT SELECT ON migration_receipts TO {role};
         GRANT SELECT, INSERT ON users TO {role};
         REVOKE INSERT ON cases FROM {role};
         GRANT SELECT, INSERT(id,title,reference,created_by) ON cases TO {role};
         REVOKE ALL ON case_administration_revisions,case_initial_stage_registrations FROM {role};
         GRANT SELECT, INSERT ON case_administration_revisions,case_initial_stage_registrations TO {role};
         REVOKE ALL ON case_stage_revisions FROM {role};
         GRANT SELECT, INSERT ON case_stage_revisions TO {role};
         GRANT EXECUTE ON FUNCTION case_stage_time_bounds(TEXT,DATE,BIGINT,INTEGER,INTEGER),
             case_stage_time_bytes(TEXT,DATE,BIGINT,INTEGER,INTEGER),
             case_stage_support_valid(UUID,BIGINT,BYTEA,TEXT,TEXT,TEXT),
             case_stage_values_canonical(case_stage_revisions),case_stage_values_bytes(case_stage_revisions),
             case_stage_recording_valid(case_stage_revisions) TO {role};
         GRANT EXECUTE ON FUNCTION case_administration_text_valid(TEXT,INTEGER,BOOLEAN),
             case_administration_is_canonical(TEXT,TEXT,TEXT,TEXT,TEXT,TEXT,TEXT,TEXT[],TEXT,TEXT),
             case_administration_bytes(TEXT,TEXT,TEXT,TEXT,TEXT,TEXT,TEXT,TEXT[],TEXT,TEXT) TO {role};
         GRANT UPDATE(recovery_codes, revision, updated_at) ON users TO {role};
         GRANT SELECT, INSERT, DELETE ON case_memberships TO {role};
         GRANT UPDATE(assigned_at) ON case_memberships TO {role};"
        ))
        .map_err(port_error)?;
    crate::credential_trust_postgres::schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::typed_participant_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::hearing_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::hearing_result_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::judicial_calendar_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::procedural_fact_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::deadline_profile_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::deadline_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::deadline_source_event_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::deadline_dispatch_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::deadline_worker_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::alert_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::procedural_resource_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::resource_activity_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::resource_hearing_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::precautionary_hearing_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::hearing_derived_deadline_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::document_integrity_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::member_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::case_report_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::audit_query_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::password_reset_schema::grant_runtime(&mut transaction, runtime_role)?;
    crate::owner_certificate_schema::grant_runtime(&mut transaction, runtime_role)?;
    validate_runtime_role(&mut transaction, runtime_role)?;
    transaction.commit().map_err(port_error)
}
