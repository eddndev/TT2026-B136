DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.typed_review_values(b BYTEA) RETURNS JSONB LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE p INTEGER:=37;n BIGINT;item JSONB;result JSONB;decisions JSONB:='[]';i INTEGER;tag INTEGER;
    candidate UUID;revision BIGINT;key BYTEA;previous BYTEA;reason JSONB;support JSONB;
BEGIN
    IF b IS NULL OR octet_length(b) NOT BETWEEN 46 AND 28941 OR substring(b FROM 1 FOR 5)<>convert_to('PREV1','UTF8') THEN
        RAISE EXCEPTION 'invalid PREV1 bytes' USING ERRCODE='23514';
    END IF;
    item:=%1$I.typed_read(b,p,'text500');p:=(item->>'next')::integer;
    result:=jsonb_build_object('directory_stamp',encode(substring(b FROM 6 FOR 32),'hex'),'selection_reason',item->'value');
    n:=%1$I.typed_u32(b,p);p:=p+4;
    IF n>16 THEN RAISE EXCEPTION 'too many identity review decisions' USING ERRCODE='23514'; END IF;
    FOR i IN 1..n LOOP
        IF p+21>octet_length(b) THEN RAISE EXCEPTION 'truncated review candidate' USING ERRCODE='23514'; END IF;
        key:=substring(b FROM p+1 FOR 21);tag:=get_byte(b,p);p:=p+1;
        candidate:=encode(substring(b FROM p+1 FOR 16),'hex')::uuid;p:=p+16;
        revision:=%1$I.typed_u32(b,p);p:=p+4;
        IF tag NOT IN (0,1) OR revision=0 OR (previous IS NOT NULL AND previous>=key) THEN
            RAISE EXCEPTION 'review candidates must be valid unique ordered references' USING ERRCODE='23514';
        END IF;
        previous:=key;
        reason:=%1$I.typed_read(b,p,'text200');p:=(reason->>'next')::integer;
        support:=%1$I.typed_read(b,p,'locator');p:=(support->>'next')::integer;
        decisions:=decisions||jsonb_build_array(jsonb_build_object('kind',tag,'id',candidate,'revision',revision,
            'reason',reason->'value','support',support->'value'));
    END LOOP;
    IF p<>octet_length(b) THEN RAISE EXCEPTION 'trailing PREV1 bytes' USING ERRCODE='23514'; END IF;
    RETURN result||jsonb_build_object('different',decisions);
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.typed_directory_stamp(scope UUID,subject_id_ UUID,subject_revision_ BIGINT,
    participant_id_ UUID,participant_revision_ BIGINT) RETURNS BYTEA LANGUAGE plpgsql AS $$
DECLARE result BYTEA:=sha256(convert_to('DIRST1','UTF8')||uuid_send(scope));head RECORD;
BEGIN
    FOR head IN SELECT s.id,r.revision,r.values_digest FROM %1$I.case_subjects s JOIN LATERAL(
        SELECT revision,values_digest FROM %1$I.case_subject_revisions WHERE subject_id=s.id
            AND (s.id IS DISTINCT FROM subject_id_ OR revision<subject_revision_) ORDER BY revision DESC LIMIT 1) r ON TRUE
        WHERE s.case_id=scope ORDER BY s.id LOOP
        result:=sha256(result||decode('00','hex')||uuid_send(head.id)||decode(lpad(to_hex(head.revision),8,'0'),'hex')||head.values_digest);
    END LOOP;
    FOR head IN SELECT p.id,r.revision,r.values_digest FROM %1$I.case_participants p JOIN LATERAL(
        SELECT revision,values_digest FROM (
            SELECT revision,values_digest FROM %1$I.case_participant_revisions WHERE participant_id=p.id
            UNION ALL SELECT revision,values_digest FROM %1$I.case_participant_typed_revisions WHERE participant_id=p.id) h
        WHERE p.id IS DISTINCT FROM participant_id_ OR revision<participant_revision_ ORDER BY revision DESC LIMIT 1) r ON TRUE
        WHERE p.case_id=scope ORDER BY p.id LOOP
        result:=sha256(result||decode('01','hex')||uuid_send(head.id)||decode(lpad(to_hex(head.revision),8,'0'),'hex')||head.values_digest);
    END LOOP;
    RETURN result;
END; $$;
$definition$,current_schema());
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.typed_review_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE value JSONB;scope UUID;candidate JSONB;actual BYTEA;bound RECORD;subject_ UUID;subject_revision_ BIGINT;
    participant_ UUID;participant_revision_ BIGINT;b BYTEA;operation INTEGER;stamp BYTEA;
BEGIN
    PERFORM pg_advisory_xact_lock(280603412820);
    value:=%1$I.typed_review_values(NEW.review_canonical);
    IF TG_TABLE_NAME='participant_identity_reviews' THEN
        SELECT r.*,p.case_id INTO bound FROM %1$I.case_participant_typed_revisions r
            JOIN %1$I.case_participants p ON p.id=r.participant_id WHERE r.participant_id=NEW.participant_id AND r.revision=NEW.revision;
        scope:=bound.case_id;participant_:=NEW.participant_id;participant_revision_:=NEW.revision;b:=NEW.submission_canonical;
        operation:=get_byte(b,21);
        IF substring(b FROM 1 FOR 5)<>convert_to('PTXN1','UTF8') OR operation NOT IN (0,1)
            OR substring(b FROM 6 FOR 16)<>uuid_send(scope)
            OR substring(b FROM 23 FOR 16)<>uuid_send(bound.subject_id)
            OR %1$I.typed_u32(b,42)<>bound.subject_revision
            OR %1$I.typed_u32(b,38)<>bound.subject_revision-operation
            OR substring(b FROM 47 FOR 32)<>substring(bound.values_canonical FROM 26 FOR 32)
            OR substring(b FROM 79 FOR 16)<>uuid_send(participant_)
            OR %1$I.typed_u32(b,94)<>NEW.revision-1 OR %1$I.typed_u32(b,98)<>NEW.revision
            OR substring(b FROM 103 FOR 32)<>bound.values_digest
            OR substring(b FROM 135 FOR 32)<>NEW.review_digest
            OR NEW.submission_digest<>bound.submission_digest OR bound.submission_revision<>NEW.revision
            OR (get_byte(b,166)=0 AND (octet_length(b)<>167 OR bound.credential_origin_revision IS NOT NULL))
            OR (get_byte(b,166)=1 AND (octet_length(b)<>615 OR bound.credential_origin_revision IS DISTINCT FROM NEW.revision))
            OR get_byte(b,166) NOT IN (0,1) THEN
            RAISE EXCEPTION 'submission must bind exact proposal and identity review' USING ERRCODE='23514';
        END IF;
        IF operation=1 THEN
            subject_:=bound.subject_id;subject_revision_:=bound.subject_revision;
            IF NOT EXISTS(SELECT 1 FROM %1$I.case_subject_revisions s JOIN %1$I.subject_identity_reviews r
                ON r.subject_id=s.subject_id AND r.revision=s.revision WHERE s.subject_id=subject_ AND s.revision=subject_revision_
                AND s.changed_at=bound.changed_at AND s.changed_by=bound.changed_by AND s.changed_by_email=bound.changed_by_email
                AND r.review_digest=NEW.review_digest) THEN
                RAISE EXCEPTION 'compound subject and role must capture one actor and review' USING ERRCODE='23514';
            END IF;
        END IF;
    ELSE
        SELECT s.case_id INTO scope FROM %1$I.case_subjects s WHERE s.id=NEW.subject_id;
        subject_:=NEW.subject_id;subject_revision_:=NEW.revision;
        SELECT r.participant_id,r.revision INTO participant_,participant_revision_ FROM %1$I.participant_identity_reviews r
            WHERE get_byte(r.submission_canonical,21)=1 AND substring(r.submission_canonical FROM 23 FOR 16)=uuid_send(subject_)
                AND %1$I.typed_u32(r.submission_canonical,42)=subject_revision_ AND r.review_digest=NEW.review_digest;
    END IF;
    stamp:=%1$I.typed_directory_stamp(scope,subject_,subject_revision_,participant_,participant_revision_);
    IF stamp<>decode(value->>'directory_stamp','hex') THEN
        RAISE EXCEPTION 'directory changed since identity review' USING ERRCODE='23514';
    END IF;
    FOR candidate IN SELECT jsonb_array_elements(value->'different') LOOP
        IF (candidate->>'kind')::integer=0 THEN
            IF NOT EXISTS(SELECT 1 FROM %1$I.case_subjects s JOIN %1$I.case_subject_revisions r ON r.subject_id=s.id
                WHERE s.case_id=scope AND s.id=(candidate->>'id')::uuid AND r.revision=(candidate->>'revision')::bigint
                    AND s.id IS DISTINCT FROM subject_) THEN
                RAISE EXCEPTION 'review candidate must be another case identity' USING ERRCODE='23514';
            END IF;
        ELSE
            IF NOT EXISTS(SELECT 1 FROM %1$I.case_participants p JOIN %1$I.case_participant_revisions r ON r.participant_id=p.id
                WHERE p.case_id=scope AND p.id=(candidate->>'id')::uuid AND r.revision=(candidate->>'revision')::bigint
                    AND p.id IS DISTINCT FROM participant_) THEN
                RAISE EXCEPTION 'review candidate must be another manual participant' USING ERRCODE='23514';
            END IF;
        END IF;
        PERFORM %1$I.typed_support_guard(scope,candidate->'support');
    END LOOP;
    RETURN NULL;
END; $$;
$definition$,current_schema());
END; $install$;
DROP TRIGGER IF EXISTS subject_review_guard ON subject_identity_reviews;
CREATE CONSTRAINT TRIGGER subject_review_guard AFTER INSERT ON subject_identity_reviews DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION typed_review_guard();
DROP TRIGGER IF EXISTS participant_review_guard ON participant_identity_reviews;
CREATE CONSTRAINT TRIGGER participant_review_guard AFTER INSERT ON participant_identity_reviews DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION typed_review_guard();
REVOKE ALL ON FUNCTION typed_review_values(BYTEA),typed_directory_stamp(UUID,UUID,BIGINT,UUID,BIGINT),typed_review_guard() FROM PUBLIC;
