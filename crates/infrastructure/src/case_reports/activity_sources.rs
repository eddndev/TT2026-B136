// Only committed original records contribute; administrative edits stay historical.
pub(super) const EVENTS: &str = "
    SELECT case_id,actor_id,recorded_at_seconds AS seconds,recorded_at_nanoseconds AS nanos,
        0 AS kind,document_id::text AS identity FROM document_upload_origins
    UNION ALL
    SELECT case_id,recorded_by,recorded_at_seconds,recorded_at_nanoseconds,
        1,family||':'||id::text FROM case_procedural_fact_revisions WHERE action='record'
    UNION ALL
    SELECT case_id,recorded_by,recorded_at_seconds,recorded_at_nanoseconds,
        1,'resource:'||act_id::text FROM case_procedural_resource_revisions WHERE action='record_act'
    UNION ALL
    SELECT case_id,recorded_by,recorded_at_seconds,recorded_at_nanoseconds,
        1,'hearing_result:'||result_id::text FROM case_hearing_result_revisions WHERE action='record'
    UNION ALL
    SELECT case_id,recorded_by,recorded_at_seconds,recorded_at_nanoseconds,
        1,'measure_decision:'||decision_id::text FROM case_measure_decisions
    UNION ALL
    SELECT r.case_id,r.recorded_by,r.recorded_at_seconds,r.recorded_at_nanoseconds,
        2,r.deadline_id::text FROM case_deadline_revisions r
    JOIN case_deadline_revisions prior ON prior.deadline_id=r.deadline_id AND prior.revision=r.revision-1
    WHERE r.action='set_attention' AND r.attention->>'status'='recorded'
        AND prior.attention->>'status'='pending'";
