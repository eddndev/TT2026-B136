-- V1 remains unchanged; V2 adds commitments without replacing the captured inputs.
ALTER TABLE case_deadline_revisions
    DROP CONSTRAINT IF EXISTS deadline_action,
    DROP CONSTRAINT IF EXISTS deadline_actor_email,
    DROP CONSTRAINT IF EXISTS deadline_review_size,
    DROP CONSTRAINT IF EXISTS deadline_capture_size,
    DROP CONSTRAINT IF EXISTS deadline_submission_size,
    DROP CONSTRAINT IF EXISTS deadline_receipt_projection,
    DROP CONSTRAINT IF EXISTS deadline_tracking_shape,
    DROP CONSTRAINT IF EXISTS deadline_tracking_commitments,
    DROP CONSTRAINT IF EXISTS deadline_recorded_author,
    ADD CONSTRAINT deadline_action CHECK(
        (action COLLATE "C" IN ('register','correct','set_attention','retire','reevaluate')) IS TRUE),
    ADD CONSTRAINT deadline_actor_email CHECK(
        (recorded_by_email IS NULL OR case_administration_text_valid(recorded_by_email,320,FALSE)) IS TRUE),
    ADD CONSTRAINT deadline_review_size CHECK((
        (substring(review_canonical FROM 1 FOR 5)=convert_to('DLRV1','UTF8')
            AND octet_length(review_canonical) BETWEEN 5 AND 524288)
        OR (substring(review_canonical FROM 1 FOR 5)=convert_to('DLRV2','UTF8')
            AND octet_length(review_canonical) BETWEEN 5 AND 524341)) IS TRUE),
    ADD CONSTRAINT deadline_capture_size CHECK((
        (substring(capture_canonical FROM 1 FOR 5)=convert_to('DLST1','UTF8')
            AND octet_length(capture_canonical) BETWEEN 5 AND 524288)
        OR (substring(capture_canonical FROM 1 FOR 5)=convert_to('DLST2','UTF8')
            AND octet_length(capture_canonical) BETWEEN 5 AND 524410)) IS TRUE),
    ADD CONSTRAINT deadline_submission_size CHECK((
        (substring(submission_canonical FROM 1 FOR 5)=convert_to('DLTX1','UTF8')
            AND octet_length(submission_canonical) BETWEEN 107 AND 4115)
        OR (substring(submission_canonical FROM 1 FOR 5)=convert_to('DLTX2','UTF8')
            AND octet_length(submission_canonical) BETWEEN 151 AND 5502)) IS TRUE),
    ADD CONSTRAINT deadline_receipt_projection CHECK((
        operation_id=(submission_view->>'operation_id')::uuid
        AND deadline_id=(submission_view->>'deadline_id')::uuid
        AND case_id=(submission_view->>'case_id')::uuid
        AND action COLLATE "C"=(submission_view->>'action') COLLATE "C"
        AND revision-1=(submission_view->>'expected_revision')::bigint
        AND review_digest=decode(submission_view->>'review_digest','hex')
        AND reason COLLATE "C" IS NOT DISTINCT FROM (submission_view->>'reason') COLLATE "C"
        AND CASE substring(submission_canonical FROM 1 FOR 5)
            WHEN convert_to('DLTX1','UTF8') THEN recorded_by=(submission_view->>'actor_id')::uuid
            WHEN convert_to('DLTX2','UTF8') THEN
                CASE submission_view->'author'->>'kind'
                    WHEN 'user' THEN recorded_by=(submission_view->'author'->>'id')::uuid
                        AND recorded_by_email COLLATE "C"=(submission_view->'author'->>'email') COLLATE "C"
                    WHEN 'technical' THEN recorded_by IS NULL AND recorded_by_email IS NULL
                    ELSE FALSE END
            ELSE FALSE END) IS TRUE),
    ADD CONSTRAINT deadline_recorded_author CHECK((
        CASE substring(submission_canonical FROM 1 FOR 5)
            WHEN convert_to('DLTX1','UTF8') THEN
                recorded_by IS NOT NULL AND recorded_by_email IS NOT NULL AND action<>'reevaluate'
            WHEN convert_to('DLTX2','UTF8') THEN
                CASE submission_view->'author'->>'kind'
                    WHEN 'user' THEN recorded_by IS NOT NULL AND recorded_by_email IS NOT NULL
                        AND action<>'reevaluate' AND submission_view->'cause'='null'::jsonb
                    WHEN 'technical' THEN recorded_by IS NULL AND recorded_by_email IS NULL
                        AND action='reevaluate'
                        AND submission_view->'author'->>'service'='deadline_reevaluator'
                        AND (submission_view->'author'->>'policy_version')::bigint=1
                        AND submission_view->'cause'->>'kind' IN ('source_event','legacy_bootstrap')
                    ELSE FALSE END
            ELSE FALSE END) IS TRUE),
    ADD CONSTRAINT deadline_tracking_shape CHECK((
        CASE substring(submission_canonical FROM 1 FOR 5)
            WHEN convert_to('DLTX1','UTF8') THEN
                substring(review_canonical FROM 1 FOR 5)=convert_to('DLRV1','UTF8')
                AND substring(capture_canonical FROM 1 FOR 5)=convert_to('DLST1','UTF8')
                AND tracking_canonical IS NULL AND observations_canonical IS NULL
                AND tracking_administration_revision IS NULL AND cause_event_sequence IS NULL
            WHEN convert_to('DLTX2','UTF8') THEN
                substring(review_canonical FROM 1 FOR 5)=convert_to('DLRV2','UTF8')
                AND substring(capture_canonical FROM 1 FOR 5)=convert_to('DLST2','UTF8')
                AND tracking_canonical IS NOT NULL AND observations_canonical IS NOT NULL
                AND octet_length(tracking_canonical) BETWEEN 102 AND 122
                AND octet_length(observations_canonical) BETWEEN 111 AND 446
            ELSE FALSE END) IS TRUE),
    ADD CONSTRAINT deadline_tracking_commitments CHECK((
        CASE substring(submission_canonical FROM 1 FOR 5)
            WHEN convert_to('DLTX1','UTF8') THEN
                tracking_canonical IS NULL AND observations_canonical IS NULL
            WHEN convert_to('DLTX2','UTF8') THEN
                tracking_canonical IS NOT NULL AND observations_canonical IS NOT NULL
                AND deadline_tracking_consistent(tracking_canonical,observations_canonical)
                AND case_id=(deadline_observations(observations_canonical)->>'case_id')::uuid
                AND pg_catalog.sha256(observations_canonical)
                    =decode(deadline_tracking(tracking_canonical)->>'observations_digest','hex')
                AND pg_catalog.sha256(observations_canonical)
                    =decode(submission_view->>'observations_digest','hex')
                AND substring(capture_canonical FROM
                    octet_length(capture_canonical)-octet_length(tracking_canonical)+1)=tracking_canonical
                AND substring(review_canonical FROM
                    octet_length(review_canonical)-(37+2*get_byte(tracking_canonical,4))+1)
                    =substring(tracking_canonical FROM 1 FOR 37+2*get_byte(tracking_canonical,4))
            ELSE FALSE END) IS TRUE);
