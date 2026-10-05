#!/usr/bin/env bash
# Exact SQL snapshot for the disposable API migration and restore campaign.
# Separate JSON objects keep each constructor within PostgreSQL's argument limit.

migration_demo_state() {
  psql "$1" -v ON_ERROR_STOP=1 -Atc \
    "SELECT jsonb_build_object(
      'documents',(SELECT jsonb_agg(to_jsonb(d) ORDER BY id,version) FROM documents d),
      'document_integrity_incidents',(SELECT jsonb_agg(to_jsonb(i) ORDER BY id)
        FROM document_integrity_incidents i),
      'series',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM document_series s),
      'metadata',(SELECT jsonb_agg(to_jsonb(m) ORDER BY document_id,metadata_revision)
        FROM document_metadata_revisions m),
      'participants',(SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM case_participants p),
      'participant_revisions',(SELECT jsonb_agg(to_jsonb(p) ORDER BY participant_id,revision)
        FROM case_participant_revisions p),
      'subjects',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM case_subjects s),
      'subject_revisions',(SELECT jsonb_agg(to_jsonb(s) ORDER BY subject_id,revision)
        FROM case_subject_revisions s),
      'typed_participants',(SELECT jsonb_agg(to_jsonb(p) ORDER BY participant_id,revision)
        FROM case_participant_typed_revisions p),
      'subject_reviews',(SELECT jsonb_agg(to_jsonb(s) ORDER BY subject_id,revision)
        FROM subject_identity_reviews s),
      'participant_reviews',(SELECT jsonb_agg(to_jsonb(p) ORDER BY participant_id,revision)
        FROM participant_identity_reviews p),
      'participant_credentials',(SELECT jsonb_agg(to_jsonb(p) ORDER BY participant_id,revision)
        FROM participant_credential_evidence p),
      'credential_trust',(SELECT jsonb_agg(to_jsonb(t) ORDER BY deployment_id,revision)
        FROM participant_credential_trust_revisions t),
      'credential_authority',(SELECT jsonb_agg(to_jsonb(a) ORDER BY deployment_id)
        FROM participant_credential_authority a),
      'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a),
      'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY fingerprint) FROM migration_receipts r),
      'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
      'cases',(SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM cases c),
      'case_administration',(SELECT jsonb_agg(to_jsonb(c) ORDER BY case_id,revision)
        FROM case_administration_revisions c),
      'initial_stages',(SELECT jsonb_agg(to_jsonb(s) ORDER BY case_id)
        FROM case_initial_stage_registrations s),
      'stage_revisions',(SELECT jsonb_agg(to_jsonb(s) ORDER BY case_id,revision)
        FROM case_stage_revisions s),
      'hearings',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM case_hearings h),
      'hearing_revisions',(SELECT jsonb_agg(to_jsonb(h) ORDER BY hearing_id,revision)
        FROM case_hearing_revisions h),
      'hearing_results',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM case_hearing_results h),
      'hearing_result_revisions',(SELECT jsonb_agg(to_jsonb(h) ORDER BY result_id,revision)
        FROM case_hearing_result_revisions h),
      'judicial_calendars',(SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM judicial_calendars c),
      'judicial_calendar_revisions',(SELECT jsonb_agg(to_jsonb(c) ORDER BY calendar_id,revision)
        FROM judicial_calendar_revisions c),
      'procedural_facts',(SELECT jsonb_agg(to_jsonb(f) ORDER BY family,id) FROM case_procedural_facts f),
      'procedural_fact_revisions',(SELECT jsonb_agg(to_jsonb(f) ORDER BY family,id,revision)
        FROM case_procedural_fact_revisions f),
      'procedural_resources',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM case_procedural_resources r),
      'procedural_resource_acts',(SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM case_procedural_resource_acts a),
      'procedural_resource_revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY resource_id,revision)
        FROM case_procedural_resource_revisions r),
      'resource_activity_associations',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id)
        FROM case_resource_activity_associations r),
      'resource_activity_revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY association_id,revision)
        FROM case_resource_activity_association_revisions r),
      'deadline_profiles',(SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM deadline_profiles p),
      'deadline_profile_revisions',(SELECT jsonb_agg(to_jsonb(p) ORDER BY profile_id,revision)
        FROM deadline_profile_revisions p),
      'deadlines',(SELECT jsonb_agg(to_jsonb(d) ORDER BY id) FROM case_deadlines d),
      'deadline_revisions',(SELECT jsonb_agg(to_jsonb(d) ORDER BY deadline_id,revision)
        FROM case_deadline_revisions d),
      'deadline_source_events',(SELECT jsonb_agg(to_jsonb(e) ORDER BY sequence)
        FROM deadline_source_events e),
      'deadline_source_sequence',(SELECT jsonb_build_object(
        'last_value',last_value,'is_called',is_called)
        FROM deadline_source_events_sequence),
      'alert_preferences',(SELECT jsonb_agg(to_jsonb(a) ORDER BY user_id,revision)
        FROM alert_preferences a),
      'alert_subject_state',(SELECT jsonb_agg(to_jsonb(a) ORDER BY kind,id)
        FROM alert_subject_state a),
      'alert_scan_cursor',(SELECT jsonb_agg(to_jsonb(a) ORDER BY singleton)
        FROM alert_scan_cursor a),
      'alert_schedule',(SELECT jsonb_agg(to_jsonb(a) ORDER BY id)
        FROM alert_schedule a),
      'alert_notifications',(SELECT jsonb_agg(to_jsonb(a) ORDER BY id)
        FROM alert_notifications a),
      'alert_read_receipts',(SELECT jsonb_agg(to_jsonb(a) ORDER BY operation_id)
        FROM alert_read_receipts a),
      'alert_email_outbox',(SELECT jsonb_agg(to_jsonb(a) ORDER BY id)
        FROM alert_email_outbox a),
      'alert_email_attempts',(SELECT jsonb_agg(to_jsonb(a) ORDER BY delivery_id,sequence)
        FROM alert_email_attempts a),
      'memberships',(SELECT jsonb_agg(to_jsonb(m) ORDER BY case_id,user_id) FROM case_memberships m)
    ) || jsonb_build_object(
      'hearing_derived_deadline_origins',(SELECT jsonb_agg(to_jsonb(o) ORDER BY operation_id)
        FROM case_hearing_derived_deadline_origins o),
      'resource_hearings',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id)
        FROM case_resource_hearings h),
      'resource_hearing_revisions',(SELECT jsonb_agg(to_jsonb(h) ORDER BY hearing_id,revision)
        FROM case_resource_hearing_revisions h),
      'precautionary_hearings',(SELECT jsonb_agg(to_jsonb(c) ORDER BY id)
        FROM case_precautionary_hearings c),
      'precautionary_hearing_revisions',(SELECT jsonb_agg(to_jsonb(c) ORDER BY hearing_id,revision)
        FROM case_precautionary_hearing_revisions c),
      'measure_operations',(SELECT jsonb_agg(to_jsonb(c) ORDER BY operation_id)
        FROM case_measure_operations c),
      'measure_decisions',(SELECT jsonb_agg(to_jsonb(c) ORDER BY decision_id)
        FROM case_measure_decisions c),
      'measures',(SELECT jsonb_agg(to_jsonb(c) ORDER BY id)
        FROM case_measures c),
      'measure_revisions',(SELECT jsonb_agg(to_jsonb(c) ORDER BY measure_id,revision)
        FROM case_measure_revisions c),
      'measure_administrations',(SELECT jsonb_agg(to_jsonb(c) ORDER BY operation_id)
        FROM case_measure_administrations c),
      'owner_certificate_registrations',(SELECT jsonb_agg(to_jsonb(r) ORDER BY binding_id)
        FROM owner_certificate_registrations r),
      'owner_certificate_withdrawals',(SELECT jsonb_agg(to_jsonb(w) ORDER BY binding_id)
        FROM owner_certificate_withdrawals w)
    )" | jq -Sc .
}
