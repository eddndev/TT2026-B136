//! Shared PostgreSQL connection and schema initialization boundary.

use application::ApplicationError;
use postgres::{Client, NoTls};

const IDENTITY_MIGRATION: &str = include_str!("../../../migrations/0001_identity.sql");
const CASE_MIGRATION: &str = include_str!("../../../migrations/0002_cases.sql");
const DOCUMENT_MIGRATION: &str = include_str!("../../../migrations/0003_case_documents_audit.sql");
const VERSION_MIGRATION: &str = include_str!("../../../migrations/0004_document_versions.sql");
const METADATA_MIGRATION: &str = include_str!("../../../migrations/0005_document_metadata.sql");
const PARTICIPANT_MIGRATION: &str = include_str!("../../../migrations/0006_case_participants.sql");
const CASE_ADMINISTRATION_MIGRATION: &str =
    include_str!("../../../migrations/0007_case_administration.sql");
const CASE_STAGE_MIGRATIONS: [&str; 3] = [
    include_str!("../../../migrations/0008_case_stages.sql"),
    include_str!("../../../migrations/0008_case_stage_values.sql"),
    include_str!("../../../migrations/0008_case_stage_guards.sql"),
];
const CREDENTIAL_TRUST_MIGRATION: &str =
    include_str!("../../../migrations/0009_participant_credential_trust.sql");
const TYPED_PARTICIPANT_MIGRATIONS: [&str; 6] = [
    include_str!("../../../migrations/0010_typed_values_primitives.sql"),
    include_str!("../../../migrations/0010_typed_values.sql"),
    include_str!("../../../migrations/0010_typed_participants.sql"),
    include_str!("../../../migrations/0010_typed_guards.sql"),
    include_str!("../../../migrations/0010_typed_reviews.sql"),
    include_str!("../../../migrations/0010_typed_credentials.sql"),
];
const HEARING_MIGRATIONS: [&str; 4] = [
    include_str!("../../../migrations/0011_hearings_values.sql"),
    include_str!("../../../migrations/0011_hearings_receipts.sql"),
    include_str!("../../../migrations/0011_hearings.sql"),
    include_str!("../../../migrations/0011_hearings_guards.sql"),
];
const HEARING_RESULT_MIGRATIONS: [&str; 5] = [
    include_str!("../../../migrations/0012_hearing_results_time.sql"),
    include_str!("../../../migrations/0012_hearing_results_values.sql"),
    include_str!("../../../migrations/0012_hearing_results_receipts.sql"),
    include_str!("../../../migrations/0012_hearing_results.sql"),
    include_str!("../../../migrations/0012_hearing_results_guards.sql"),
];
const JUDICIAL_CALENDAR_MIGRATIONS: [&str; 6] = [
    include_str!("../../../migrations/0013_judicial_calendar_primitives.sql"),
    include_str!("../../../migrations/0013_judicial_calendar_sources.sql"),
    include_str!("../../../migrations/0013_judicial_calendar_values.sql"),
    include_str!("../../../migrations/0013_judicial_calendar_receipts.sql"),
    include_str!("../../../migrations/0013_judicial_calendar_tables.sql"),
    include_str!("../../../migrations/0013_judicial_calendar_guards.sql"),
];
const PROCEDURAL_FACT_MIGRATIONS: [&str; 11] = [
    include_str!("../../../migrations/0014_procedural_fact_primitives.sql"),
    include_str!("../../../migrations/0014_procedural_fact_time.sql"),
    include_str!("../../../migrations/0014_procedural_fact_people.sql"),
    include_str!("../../../migrations/0014_procedural_fact_provenance.sql"),
    include_str!("../../../migrations/0014_procedural_fact_values.sql"),
    include_str!("../../../migrations/0014_procedural_fact_source_items.sql"),
    include_str!("../../../migrations/0014_procedural_fact_sources.sql"),
    include_str!("../../../migrations/0014_procedural_fact_receipts.sql"),
    include_str!("../../../migrations/0014_procedural_facts.sql"),
    include_str!("../../../migrations/0014_procedural_fact_source_guards.sql"),
    include_str!("../../../migrations/0014_procedural_facts_guards.sql"),
];
const DEADLINE_SOURCE_EVENT_MIGRATION: &str =
    include_str!("../../../migrations/0015_deadline_source_events.sql");
const DEADLINE_PROFILE_MIGRATIONS: &[&str] = &[
    include_str!("../../../migrations/0016_deadline_profile_projection.sql"),
    include_str!("../../../migrations/0016_deadline_profile_receipts.sql"),
    include_str!("../../../migrations/0016_deadline_profile_tables.sql"),
    include_str!("../../../migrations/0016_deadline_profile_guards.sql"),
    include_str!("../../../migrations/0016_deadline_profile_events.sql"),
];
// Every adapter uses this same database-scoped lock before applying schema DDL.
const SCHEMA_MIGRATION_LOCK: i64 = 0x4341534553;

/// Applies all schema prerequisites atomically, serialized across processes.
pub(crate) fn connect(database_url: &str) -> Result<Client, ApplicationError> {
    let mut client = Client::connect(database_url, NoTls).map_err(port_error)?;
    require_utf8(&mut client)?;
    let mut transaction = client.transaction().map_err(port_error)?;
    transaction
        .query_one(
            "SELECT pg_advisory_xact_lock($1)",
            &[&SCHEMA_MIGRATION_LOCK],
        )
        .map_err(port_error)?;
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
    transaction.commit().map_err(port_error)?;
    Ok(client)
}

/// Connects without DDL and rejects roles able to bypass persisted evidence protection.
pub(crate) fn open(database_url: &str) -> Result<Client, ApplicationError> {
    let mut client = Client::connect(database_url, NoTls).map_err(port_error)?;
    require_utf8(&mut client)?;
    crate::postgres_version_schema::validate(&mut client)?;
    crate::postgres_metadata_schema::validate(&mut client)?;
    crate::postgres_participant_schema::validate(&mut client)?;
    crate::postgres_case_administration_schema::validate(&mut client)?;
    crate::postgres_case_stages_schema::validate(&mut client)?;
    crate::credential_trust_postgres::schema::validate(&mut client)?;
    crate::typed_participant_schema::validate(&mut client)?;
    crate::hearing_schema::validate(&mut client)?;
    crate::hearing_result_schema::validate(&mut client)?;
    crate::judicial_calendar_schema::validate(&mut client)?;
    crate::procedural_fact_schema::validate(&mut client)?;
    crate::deadline_profile_schema::validate(&mut client)?;
    crate::deadline_schema::validate(&mut client)?;
    crate::deadline_source_event_schema::validate(&mut client)?;
    crate::deadline_dispatch_schema::validate(&mut client)?;
    crate::deadline_worker_schema::validate(&mut client)?;
    crate::alert_schema::validate(&mut client)?;
    crate::procedural_resource_schema::validate(&mut client)?;
    crate::resource_activity_schema::validate(&mut client)?;
    let role: String = client
        .query_one("SELECT current_user", &[])
        .map_err(port_error)?
        .get(0);
    validate_runtime_role(&mut client, &role)?;
    crate::postgres_version_schema::validate_inventory(&mut client)?;
    crate::postgres_metadata_schema::validate_inventory(&mut client)?;
    crate::postgres_participant_schema::validate_inventory(&mut client)?;
    crate::postgres_case_administration_inventory::validate(&mut client)?;
    crate::postgres_case_stages_inventory::validate(&mut client)?;
    crate::credential_trust_postgres::schema::validate_inventory(&mut client)?;
    crate::typed_participant_schema::validate_inventory(&mut client)?;
    crate::hearing_schema::validate_inventory(&mut client)?;
    crate::hearing_result_schema::validate_inventory(&mut client)?;
    crate::judicial_calendar_schema::validate_inventory(&mut client)?;
    crate::procedural_fact_schema::validate_inventory(&mut client)?;
    crate::deadline_profile_schema::validate_inventory(&mut client)?;
    crate::deadline_schema::validate_inventory(&mut client)?;
    crate::deadline_source_event_schema::validate_inventory(&mut client)?;
    crate::deadline_dispatch_schema::validate_inventory(&mut client)?;
    crate::deadline_worker_schema::validate_inventory(&mut client)?;
    crate::alerts_postgres::validate_inventory(&mut client)?;
    crate::procedural_resource_schema::validate_inventory(&mut client)?;
    crate::resource_activity_schema::validate_inventory(&mut client)?;
    Ok(client)
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
    validate_runtime_role(&mut transaction, runtime_role)?;
    transaction.commit().map_err(port_error)
}

fn validate_runtime_role<C: postgres::GenericClient>(
    client: &mut C,
    role: &str,
) -> Result<(), ApplicationError> {
    crate::credential_trust_postgres::schema::validate_runtime_role(client, role)?;
    crate::typed_participant_schema::validate_runtime_role(client, role)?;
    crate::hearing_schema::validate_runtime_role(client, role)?;
    crate::hearing_result_schema::validate_runtime_role(client, role)?;
    crate::judicial_calendar_schema::validate_runtime_role(client, role)?;
    crate::procedural_fact_schema::validate_runtime_role(client, role)?;
    crate::deadline_profile_schema::validate_runtime_role(client, role)?;
    crate::deadline_schema::validate_runtime_role(client, role)?;
    crate::deadline_source_event_schema::validate_runtime_role(client, role)?;
    crate::deadline_dispatch_schema::validate_runtime_role(client, role)?;
    crate::deadline_worker_schema::validate_runtime_role(client, role)?;
    crate::alert_schema::validate_runtime_role(client, role)?;
    crate::procedural_resource_schema::validate_runtime_role(client, role)?;
    crate::resource_activity_schema::validate_runtime_role(client, role)?;
    // Catalog resolution prevents spoofing; membership checks also cover SET ROLE escalation.
    let unsafe_role: bool = client
        .query_one(
            "SELECT pg_catalog.bool_or(r.rolsuper OR r.rolcreaterole
         OR EXISTS (
             SELECT 1 FROM pg_catalog.pg_class c
             JOIN pg_catalog.pg_namespace n ON n.oid OPERATOR(pg_catalog.=) c.relnamespace
             WHERE c.oid OPERATOR(pg_catalog.=) ANY(ARRAY['audit_events'::pg_catalog.regclass,
                 'documents'::pg_catalog.regclass, 'document_series'::pg_catalog.regclass,
                 'document_metadata_revisions'::pg_catalog.regclass,
                 'case_participants'::pg_catalog.regclass,'case_participant_revisions'::pg_catalog.regclass,
                 'migration_receipts'::pg_catalog.regclass, 'cases'::pg_catalog.regclass,
                 'case_administration_revisions'::pg_catalog.regclass,
                 'case_stage_revisions'::pg_catalog.regclass,
                 'case_initial_stage_registrations'::pg_catalog.regclass])
             AND (pg_catalog.pg_has_role(r.oid, c.relowner, 'MEMBER')
                 OR pg_catalog.pg_has_role(r.oid, n.nspowner, 'MEMBER')
                 OR pg_catalog.has_table_privilege(r.oid, c.oid, 'DELETE, TRUNCATE, TRIGGER')
                 OR (c.oid OPERATOR(pg_catalog.<>) 'documents'::pg_catalog.regclass
                     AND pg_catalog.has_any_column_privilege(r.oid, c.oid, 'UPDATE'))
                 OR (c.oid OPERATOR(pg_catalog.=) 'migration_receipts'::pg_catalog.regclass
                     AND pg_catalog.has_any_column_privilege(r.oid, c.oid, 'INSERT'))
                 OR (c.oid OPERATOR(pg_catalog.=) 'cases'::pg_catalog.regclass AND (
                     pg_catalog.has_column_privilege(r.oid,c.oid,'created_at','INSERT')
                     OR pg_catalog.has_column_privilege(r.oid,c.oid,'required_initial_revision','INSERT')))
                 OR (c.oid OPERATOR(pg_catalog.=) 'documents'::pg_catalog.regclass AND EXISTS (
                     SELECT 1 FROM pg_catalog.pg_attribute a
                     WHERE a.attrelid OPERATOR(pg_catalog.=) c.oid
                         AND a.attnum OPERATOR(pg_catalog.>) 0 AND NOT a.attisdropped
                         AND a.attname OPERATOR(pg_catalog.<>) 'evidence'
                         AND pg_catalog.has_column_privilege(r.oid, c.oid, a.attnum, 'UPDATE')
                 )))
         ) OR EXISTS (
             SELECT 1 FROM pg_catalog.pg_proc p
             WHERE p.oid OPERATOR(pg_catalog.=) ANY(ARRAY[
                 'preserve_document_evidence()'::pg_catalog.regprocedure,
                 'case_stage_time_bounds(text,date,bigint,integer,integer)'::pg_catalog.regprocedure,
                 'case_stage_time_bytes(text,date,bigint,integer,integer)'::pg_catalog.regprocedure,
                 'case_stage_support_valid(uuid,bigint,bytea,text,text,text)'::pg_catalog.regprocedure,
                 'case_stage_values_canonical(case_stage_revisions)'::pg_catalog.regprocedure,
                 'case_stage_values_bytes(case_stage_revisions)'::pg_catalog.regprocedure,
                 'case_stage_recording_valid(case_stage_revisions)'::pg_catalog.regprocedure,
                 'preserve_case_stage_history()'::pg_catalog.regprocedure,
                 'enforce_case_stage_sequence()'::pg_catalog.regprocedure,
                 'preserve_case_stage_initial_exclusivity()'::pg_catalog.regprocedure,
                 'case_administration_text_valid(text,integer,boolean)'::pg_catalog.regprocedure,
                 'case_administration_is_canonical(text,text,text,text,text,text,text,text[],text,text)'::pg_catalog.regprocedure,
                 'case_administration_bytes(text,text,text,text,text,text,text,text[],text,text)'::pg_catalog.regprocedure,
                 'preserve_case_administration_history()'::pg_catalog.regprocedure,
                 'enforce_case_administration_sequence()'::pg_catalog.regprocedure,
                 'validate_case_administration_heads()'::pg_catalog.regprocedure,
                 'validate_case_initial_stage_registration()'::pg_catalog.regprocedure,
                 'preserve_participant_history()'::pg_catalog.regprocedure,
                 'enforce_participant_sequence()'::pg_catalog.regprocedure,
                 'participant_text_valid(text,integer)'::pg_catalog.regprocedure,
                 'participant_values_is_canonical(text,text,text,text,text)'::pg_catalog.regprocedure,
                 'participant_values_bytes(text,text,text,text,text)'::pg_catalog.regprocedure,
                 'preserve_document_series()'::pg_catalog.regprocedure,
                 'preserve_document_metadata()'::pg_catalog.regprocedure,
                 'enforce_document_metadata_sequence()'::pg_catalog.regprocedure,
                 'document_metadata_text_valid(text,integer)'::pg_catalog.regprocedure,
                 'document_metadata_is_canonical(text,text,text[])'::pg_catalog.regprocedure,
                 'document_metadata_bytes(text,text,text[])'::pg_catalog.regprocedure,
                 'enforce_document_version_sequence()'::pg_catalog.regprocedure])
                 AND pg_catalog.pg_has_role(r.oid, p.proowner, 'MEMBER')
         ) OR EXISTS (
             SELECT 1 FROM pg_catalog.pg_namespace n
             WHERE n.nspname OPERATOR(pg_catalog.=) ANY(pg_catalog.current_schemas(true))
                 AND (pg_catalog.pg_has_role(r.oid, n.nspowner, 'MEMBER')
                     OR pg_catalog.has_schema_privilege(r.oid, n.oid, 'CREATE'))
         )) FROM pg_catalog.pg_roles r WHERE pg_catalog.pg_has_role($1, r.oid, 'MEMBER')",
            &[&role],
        )
        .map_err(port_error)?
        .get(0);
    if unsafe_role {
        return Err(ApplicationError::InvalidConfiguration(
            "runtime database role must not own protected objects, modify immutable data, create search-path objects, or administer roles".into()
        ));
    }
    Ok(())
}

fn port_error(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("postgres initialization: {error}"))
}

fn require_utf8(client: &mut Client) -> Result<(), ApplicationError> {
    let encoding: String = client
        .query_one("SELECT pg_catalog.current_setting('server_encoding')", &[])
        .map_err(port_error)?
        .get(0);
    if encoding != "UTF8" {
        return Err(ApplicationError::InvalidConfiguration(
            "PostgreSQL database encoding must be UTF8 for canonical document metadata".into(),
        ));
    }
    Ok(())
}
