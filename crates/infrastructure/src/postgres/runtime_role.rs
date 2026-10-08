use crate::postgres_connection::port_error;
use application::ApplicationError;

pub(super) fn validate_runtime_role<C: postgres::GenericClient>(
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
    crate::resource_hearing_schema::validate_runtime_role(client, role)?;
    crate::precautionary_hearing_schema::validate_runtime_role(client, role)?;
    crate::measure_decision_schema::validate_runtime_role(client, role)?;
    crate::hearing_derived_deadline_schema::validate_runtime_role(client, role)?;
    crate::document_integrity_schema::validate_runtime_role(client, role)?;
    crate::member_schema::validate_runtime_role(client, role)?;
    crate::case_report_schema::validate_runtime_role(client, role)?;
    crate::audit_query_schema::validate_runtime_role(client, role)?;
    crate::password_reset_schema::validate_runtime(client, role)?;
    crate::owner_certificate_schema::validate_runtime(client, role)?;
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
