use super::{incomplete, port};
use application::ApplicationError;
use postgres::GenericClient;

/// Sources predating installation need no event; every stored event must be exact.
pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let invalid: bool = client.query_one("SELECT EXISTS(
        SELECT 1 FROM deadline_source_events e
        LEFT JOIN case_procedural_fact_revisions f ON f.family=e.source_kind AND f.id=e.source_id AND f.revision=e.revision
        LEFT JOIN case_hearing_result_revisions h ON e.source_kind='hearing_result' AND h.result_id=e.source_id AND h.revision=e.revision
        LEFT JOIN judicial_calendar_revisions c ON e.source_kind='calendar' AND c.calendar_id=e.source_id AND c.revision=e.revision
        LEFT JOIN deadline_profile_revisions p ON e.source_kind='profile' AND p.profile_id=e.source_id AND p.revision=e.revision
        LEFT JOIN deadline_profiles r ON e.source_kind='profile' AND r.id=p.profile_id
        WHERE CASE e.source_kind WHEN 'resolution' THEN f.operation_id IS DISTINCT FROM e.operation_id OR f.case_id IS DISTINCT FROM e.case_id
            WHEN 'notification' THEN f.operation_id IS DISTINCT FROM e.operation_id OR f.case_id IS DISTINCT FROM e.case_id
            WHEN 'hearing_result' THEN h.operation_id IS DISTINCT FROM e.operation_id OR h.case_id IS DISTINCT FROM e.case_id OR h.hearing_id IS DISTINCT FROM e.hearing_id
            WHEN 'calendar' THEN c.operation_id IS DISTINCT FROM e.operation_id
            WHEN 'profile' THEN p.operation_id IS DISTINCT FROM e.operation_id OR r.id IS NULL OR r.case_id IS DISTINCT FROM e.case_id
            ELSE TRUE END)
        OR coalesce((SELECT max(sequence) FROM deadline_source_events),0)>
            coalesce((SELECT last_value FROM pg_sequences WHERE schemaname=current_schema() AND sequencename='deadline_source_events_sequence'),0)", &[]).map_err(port)?.get(0);
    if invalid {
        return Err(incomplete());
    }
    Ok(())
}
