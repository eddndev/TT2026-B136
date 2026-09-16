DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.typed_credential_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE bound RECORD;trust RECORD;submission BYTEA;proof BYTEA:=NEW.declaration;kind INTEGER;recorded TEXT;
BEGIN
    PERFORM pg_advisory_xact_lock(280603412820);
    SELECT r.*,p.case_id INTO bound FROM %1$I.case_participant_typed_revisions r JOIN %1$I.case_participants p
        ON p.id=r.participant_id WHERE r.participant_id=NEW.participant_id AND r.revision=NEW.revision;
    SELECT r.*,a.root_fingerprint INTO trust FROM %1$I.participant_credential_trust_revisions r
        JOIN %1$I.participant_credential_authority a USING(deployment_id) WHERE r.deployment_id=NEW.deployment_id ORDER BY r.revision DESC LIMIT 1;
    SELECT submission_canonical INTO submission FROM %1$I.participant_identity_reviews
        WHERE participant_id=NEW.participant_id AND revision=NEW.revision;
    kind:=array_position(ARRAY['defendant','victim','defense_counsel','prosecutor','victim_counsel','control_judge',
        'trial_court','expert','police','precautionary_supervisor','other'],bound.role_kind)-1;
    IF NEW.accepted_at_seconds NOT BETWEEN -62135596800 AND 253402300799 THEN
        RAISE EXCEPTION 'credential acceptance time is outside the supported range' USING ERRCODE='23514';
    END IF;
    recorded:=to_char(to_timestamp(NEW.accepted_at_seconds) AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS')
        ||CASE WHEN NEW.accepted_at_nanoseconds=0 THEN 'Z'
            ELSE '.'||rtrim(lpad(NEW.accepted_at_nanoseconds::text,9,'0'),'0')||'Z' END;
    IF bound.submission_revision IS DISTINCT FROM NEW.revision OR bound.credential_origin_revision IS DISTINCT FROM NEW.revision
        OR trust.revision IS DISTINCT FROM NEW.trust_revision
        OR NEW.valid_from<trust.valid_from OR NEW.valid_until>trust.valid_until
        OR NEW.checked_at>NEW.accepted_at_seconds OR bound.changed_at IS DISTINCT FROM recorded
        OR substring(proof FROM 1 FOR 8)<>decode('5043524544310000','hex')
        OR substring(proof FROM 9 FOR 16)<>uuid_send(NEW.deployment_id)
        OR substring(proof FROM 25 FOR 32)<>trust.root_fingerprint
        OR substring(proof FROM 57 FOR 16)<>uuid_send(bound.case_id)
        OR substring(proof FROM 73 FOR 16)<>uuid_send(bound.subject_id)
        OR get_byte(proof,88)<>get_byte(submission,21)
        OR substring(proof FROM 90 FOR 40)<>substring(submission FROM 39 FOR 40)
        OR substring(proof FROM 130 FOR 16)<>uuid_send(NEW.participant_id)
        OR %1$I.typed_u32(proof,145)<>NEW.revision-1 OR %1$I.typed_u32(proof,149)<>NEW.revision
        OR get_byte(proof,153)<>kind OR substring(proof FROM 155 FOR 32)<>bound.values_digest
        OR substring(proof FROM 187 FOR 32)<>NEW.certificate_fingerprint
        OR octet_length(submission)<>615 OR get_byte(submission,166)<>1
        OR substring(submission FROM 168 FOR 32)<>NEW.statement_digest
        OR substring(submission FROM 200 FOR 32)<>NEW.certificate_fingerprint
        OR substring(submission FROM 232 FOR 384)<>NEW.signature THEN
        RAISE EXCEPTION 'credential must bind exact accepted statement and current trust' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END; $$;
$definition$,current_schema());
END; $install$;
DROP TRIGGER IF EXISTS participant_credential_guard ON participant_credential_evidence;
CREATE CONSTRAINT TRIGGER participant_credential_guard AFTER INSERT ON participant_credential_evidence DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION typed_credential_guard();
REVOKE ALL ON FUNCTION typed_credential_guard() FROM PUBLIC;
