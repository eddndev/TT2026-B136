WITH authorized_hearings AS MATERIALIZED (
    SELECT h.case_id, h.id
    FROM case_hearings h
    WHERE $3::smallint IN (-1, 0)
      AND ($1::boolean OR EXISTS (
          SELECT 1 FROM case_memberships m
          WHERE m.case_id = h.case_id AND m.user_id = $2::uuid
      ))
), authorized_deadlines AS MATERIALIZED (
    SELECT d.case_id, d.id
    FROM case_deadlines d
    WHERE $3::smallint IN (-1, 1)
      AND ($1::boolean OR EXISTS (
          SELECT 1 FROM case_memberships m
          WHERE m.case_id = d.case_id AND m.user_id = $2::uuid
      ))
), heads AS (
    SELECT h.case_id, h.id, r.revision, 0::smallint AS kind_rank,
           (r.values_view->'time'->>'seconds')::bigint AS seconds,
           0::integer AS nanoseconds, r.status
    FROM authorized_hearings h
    CROSS JOIN LATERAL (
        SELECT revision, values_view, status
        FROM case_hearing_revisions
        WHERE hearing_id = h.id AND case_id = h.case_id
        ORDER BY revision DESC LIMIT 1
    ) r
    WHERE $4::text IS NULL OR r.status = $4
    UNION ALL
    SELECT d.case_id, d.id, r.revision, 1::smallint AS kind_rank,
           r.due_at_seconds AS seconds, r.due_at_nanoseconds AS nanoseconds,
           r.status
    FROM authorized_deadlines d
    CROSS JOIN LATERAL (
        SELECT revision, status, due_at_seconds, due_at_nanoseconds
        FROM case_deadline_revisions
        WHERE deadline_id = d.id AND case_id = d.case_id
        ORDER BY revision DESC LIMIT 1
    ) r
    WHERE r.status = 'active' AND r.due_at_seconds IS NOT NULL
      AND r.due_at_nanoseconds IS NOT NULL
)
SELECT case_id, id, revision, kind_rank, seconds, nanoseconds, status
FROM heads
WHERE (seconds, nanoseconds) >= ($5::bigint, 0::integer)
  AND (seconds, nanoseconds) < ($6::bigint, 0::integer)
  AND ($7::bigint IS NULL OR
       (seconds, nanoseconds, kind_rank, id) >
       ($7::bigint, $8::integer, $9::smallint, $10::uuid))
ORDER BY seconds, nanoseconds, kind_rank, id
LIMIT 101;
