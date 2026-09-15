-- Case profiles and organizational status keep immutable, independently audited history.
DO $$ BEGIN
    IF pg_catalog.current_setting('server_encoding')<>'UTF8' THEN
        RAISE EXCEPTION 'canonical case administration requires UTF8 database encoding';
    END IF;
END; $$;
CREATE OR REPLACE FUNCTION case_administration_text_valid(value TEXT, maximum INTEGER, multiline BOOLEAN)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE code INTEGER; position INTEGER;
    whitespace INTEGER[] := ARRAY[10,32,160,5760,8192,8193,8194,8195,8196,8197,
        8198,8199,8200,8201,8202,8232,8233,8239,8287,12288];
BEGIN
    IF value IS NULL OR maximum IS NULL OR maximum<1 OR multiline IS NULL
        OR pg_catalog.char_length(value) NOT BETWEEN 1 AND maximum THEN RETURN FALSE; END IF;
    IF pg_catalog.ascii(pg_catalog.left(value,1))=ANY(whitespace)
        OR pg_catalog.ascii(pg_catalog.right(value,1))=ANY(whitespace) THEN RETURN FALSE; END IF;
    FOR position IN 1..pg_catalog.char_length(value) LOOP
        code := pg_catalog.ascii(pg_catalog.substr(value,position,1));
        IF (code BETWEEN 0 AND 31 OR code BETWEEN 127 AND 159)
            AND NOT (multiline AND code=10) THEN RETURN FALSE; END IF;
    END LOOP;
    RETURN TRUE;
END; $$;
DO $install$ BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.case_administration_is_canonical(
    administrative_status TEXT,title TEXT,reference TEXT,nuc TEXT,nuc_authority TEXT,
    judicial_case_number TEXT,judicial_authority TEXT,offenses TEXT[],
    general_information TEXT,complementary_identifiers TEXT)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE value TEXT;
BEGIN
    IF administrative_status IS NULL OR administrative_status COLLATE "C" NOT IN ('active','closed')
        OR NOT %1$I.case_administration_text_valid(title,200,FALSE)
        OR NOT %1$I.case_administration_text_valid(reference,100,FALSE) THEN RETURN FALSE; END IF;
    IF pg_catalog.num_nonnulls(nuc,nuc_authority,judicial_case_number,judicial_authority,
        offenses,general_information,complementary_identifiers)=0 THEN RETURN TRUE; END IF;
    IF NOT %1$I.case_administration_text_valid(nuc,100,FALSE)
        OR NOT %1$I.case_administration_text_valid(nuc_authority,200,FALSE)
        OR NOT %1$I.case_administration_text_valid(judicial_case_number,100,FALSE)
        OR NOT %1$I.case_administration_text_valid(judicial_authority,200,FALSE)
        OR offenses IS NULL OR COALESCE(pg_catalog.array_ndims(offenses),0)<>1
        OR pg_catalog.array_lower(offenses,1)<>1
        OR pg_catalog.cardinality(offenses) NOT BETWEEN 1 AND 8
        OR (general_information IS NOT NULL AND NOT %1$I.case_administration_text_valid(general_information,1000,TRUE))
        OR (complementary_identifiers IS NOT NULL AND NOT %1$I.case_administration_text_valid(complementary_identifiers,300,FALSE)) THEN RETURN FALSE; END IF;
    FOREACH value IN ARRAY offenses LOOP
        IF NOT %1$I.case_administration_text_valid(value,120,FALSE) THEN RETURN FALSE; END IF;
    END LOOP;
    RETURN NOT EXISTS(SELECT 1 FROM pg_catalog.unnest(offenses) item
        GROUP BY pg_catalog.convert_to(item,'UTF8') HAVING COUNT(*)>1);
END; $$;
$definition$, current_schema());
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.case_administration_bytes(
    administrative_status TEXT,title TEXT,reference TEXT,nuc TEXT,nuc_authority TEXT,
    judicial_case_number TEXT,judicial_authority TEXT,offenses TEXT[],
    general_information TEXT,complementary_identifiers TEXT)
RETURNS BYTEA LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE result BYTEA := pg_catalog.convert_to('CADM1','UTF8'); value TEXT; encoded BYTEA;
BEGIN
    IF NOT %1$I.case_administration_is_canonical(administrative_status,title,reference,nuc,
        nuc_authority,judicial_case_number,judicial_authority,offenses,general_information,complementary_identifiers) THEN
        RAISE EXCEPTION 'case administration values must be canonical' USING ERRCODE='23514';
    END IF;
    result := result || CASE administrative_status WHEN 'active' THEN pg_catalog.decode('00','hex') ELSE pg_catalog.decode('01','hex') END;
    FOREACH value IN ARRAY ARRAY[title,reference] LOOP
        encoded := pg_catalog.convert_to(value,'UTF8');
        result := result || pg_catalog.int4send(pg_catalog.octet_length(encoded)) || encoded;
    END LOOP;
    IF nuc IS NULL THEN RETURN result || pg_catalog.decode('00','hex'); END IF;
    result := result || pg_catalog.decode('01','hex');
    FOREACH value IN ARRAY ARRAY[nuc,nuc_authority,judicial_case_number,judicial_authority] LOOP
        encoded := pg_catalog.convert_to(value,'UTF8');
        result := result || pg_catalog.int4send(pg_catalog.octet_length(encoded)) || encoded;
    END LOOP;
    result := result || pg_catalog.int4send(pg_catalog.cardinality(offenses));
    FOREACH value IN ARRAY offenses LOOP
        encoded := pg_catalog.convert_to(value,'UTF8');
        result := result || pg_catalog.int4send(pg_catalog.octet_length(encoded)) || encoded;
    END LOOP;
    FOREACH value IN ARRAY ARRAY[general_information,complementary_identifiers] LOOP
        IF value IS NULL THEN result := result || pg_catalog.decode('00','hex');
        ELSE
            encoded := pg_catalog.convert_to(value,'UTF8');
            result := result || pg_catalog.decode('01','hex') || pg_catalog.int4send(pg_catalog.octet_length(encoded)) || encoded;
        END IF;
    END LOOP;
    RETURN result;
END; $$;
$definition$, current_schema());
END; $install$;
ALTER TABLE cases ADD COLUMN IF NOT EXISTS required_initial_revision BIGINT;
CREATE TABLE IF NOT EXISTS case_administration_revisions (
    case_id UUID NOT NULL, revision BIGINT NOT NULL,
    title TEXT NOT NULL, reference TEXT NOT NULL, administrative_status TEXT NOT NULL,
    nuc TEXT, nuc_authority TEXT, judicial_case_number TEXT, judicial_authority TEXT,
    offenses TEXT[], general_information TEXT, complementary_identifiers TEXT,
    values_digest BYTEA NOT NULL, changed_at TEXT NOT NULL,
    changed_by UUID NOT NULL, changed_by_email TEXT NOT NULL,
    PRIMARY KEY(case_id,revision),
    CONSTRAINT case_administration_root_fk FOREIGN KEY(case_id) REFERENCES cases(id),
    CONSTRAINT case_administration_actor_fk FOREIGN KEY(changed_by) REFERENCES users(id),
    CONSTRAINT case_administration_revision_range CHECK(revision BETWEEN 1 AND 4294967295),
    CONSTRAINT case_administration_canonical CHECK(case_administration_is_canonical(administrative_status,title,reference,nuc,nuc_authority,judicial_case_number,judicial_authority,offenses,general_information,complementary_identifiers)),
    CONSTRAINT case_administration_digest CHECK(values_digest=pg_catalog.sha256(case_administration_bytes(administrative_status,title,reference,nuc,nuc_authority,judicial_case_number,judicial_authority,offenses,general_information,complementary_identifiers)))
);
CREATE TABLE IF NOT EXISTS case_initial_stage_registrations (
    case_id UUID PRIMARY KEY, stage_revision BIGINT NOT NULL DEFAULT 1,
    administration_revision BIGINT NOT NULL DEFAULT 1, stage TEXT NOT NULL,
    CONSTRAINT case_initial_stage_revision CHECK(stage_revision=1),
    CONSTRAINT case_initial_stage_administration_revision CHECK(administration_revision=1),
    CONSTRAINT case_initial_stage_value CHECK(stage COLLATE "C"='investigation'),
    CONSTRAINT case_initial_stage_administration_fk FOREIGN KEY(case_id,administration_revision)
        REFERENCES case_administration_revisions(case_id,revision)
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint WHERE conrelid='cases'::regclass AND conname='case_root_metadata_canonical') THEN
        ALTER TABLE cases ADD CONSTRAINT case_root_metadata_canonical CHECK(case_administration_text_valid(title,200,FALSE) AND case_administration_text_valid(reference,100,FALSE));
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint WHERE conrelid='cases'::regclass AND conname='case_created_at_finite') THEN
        ALTER TABLE cases ADD CONSTRAINT case_created_at_finite CHECK(pg_catalog.isfinite(created_at));
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint WHERE conrelid='cases'::regclass AND conname='case_required_initial_revision') THEN
        ALTER TABLE cases ADD CONSTRAINT case_required_initial_revision CHECK(required_initial_revision IS NULL OR required_initial_revision=1);
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint WHERE conrelid='cases'::regclass AND conname='case_first_administration_revision_fk') THEN
        ALTER TABLE cases ADD CONSTRAINT case_first_administration_revision_fk FOREIGN KEY(id,required_initial_revision)
            REFERENCES case_administration_revisions(case_id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
END; $$;
ALTER TABLE cases ALTER COLUMN required_initial_revision SET DEFAULT 1;
CREATE OR REPLACE FUNCTION preserve_case_administration_history() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF OLD IS DISTINCT FROM NEW THEN
        RAISE EXCEPTION 'case roots, administration and initial registrations are immutable' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
DO $install$ BEGIN
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_case_administration_sequence() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE root RECORD; previous RECORD; previous_status TEXT; previous_bytes BYTEA;
BEGIN
    IF pg_catalog.current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'case administration writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    -- Match crates/infrastructure/src/audit_postgres.rs before reading any head.
    PERFORM pg_catalog.pg_advisory_xact_lock(280603412820);
    SELECT * INTO root FROM %1$I.cases WHERE id=NEW.case_id;
    IF NOT FOUND THEN RAISE EXCEPTION 'case root is missing' USING ERRCODE='23503'; END IF;
    SELECT * INTO previous FROM %1$I.case_administration_revisions WHERE case_id=NEW.case_id ORDER BY revision DESC LIMIT 1;
    IF NEW.revision IS DISTINCT FROM COALESCE(previous.revision+1,1) THEN
        RAISE EXCEPTION 'case administration revisions must be contiguous' USING ERRCODE='23514';
    END IF;
    IF NEW.revision=1 AND root.required_initial_revision=1 AND
        (NEW.administrative_status COLLATE "C"<>'active' OR NEW.title IS DISTINCT FROM root.title
            OR NEW.reference IS DISTINCT FROM root.reference OR NEW.changed_by IS DISTINCT FROM root.created_by) THEN
        RAISE EXCEPTION 'new case revision one must match its active creation' USING ERRCODE='23514';
    END IF;
    IF previous.revision IS NULL THEN
        previous_status := 'active';
        previous_bytes := %1$I.case_administration_bytes('active',root.title,root.reference,NULL,NULL,NULL,NULL,NULL,NULL,NULL);
    ELSE
        previous_status := previous.administrative_status;
        previous_bytes := %1$I.case_administration_bytes('active',previous.title,previous.reference,previous.nuc,previous.nuc_authority,previous.judicial_case_number,previous.judicial_authority,previous.offenses,previous.general_information,previous.complementary_identifiers);
        IF previous.nuc IS NOT NULL AND NEW.nuc IS NULL THEN
            RAISE EXCEPTION 'a complete case profile cannot be removed' USING ERRCODE='23514';
        END IF;
    END IF;
    IF (previous_status COLLATE "C"='closed' OR NEW.administrative_status IS DISTINCT FROM previous_status)
        AND previous_bytes IS DISTINCT FROM %1$I.case_administration_bytes('active',NEW.title,NEW.reference,NEW.nuc,NEW.nuc_authority,NEW.judicial_case_number,NEW.judicial_authority,NEW.offenses,NEW.general_information,NEW.complementary_identifiers) THEN
        RAISE EXCEPTION 'case status changes must preserve current profile values' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$, current_schema());
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.validate_case_administration_heads() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE required BOOLEAN; present BOOLEAN;
BEGIN
    IF pg_catalog.current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'case administration writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_catalog.pg_advisory_xact_lock(280603412820);
    IF EXISTS(SELECT 1 FROM (SELECT DISTINCT ON(case_id) nuc FROM %1$I.case_administration_revisions ORDER BY case_id,revision DESC) heads
        WHERE nuc IS NOT NULL GROUP BY nuc COLLATE "C" HAVING COUNT(*)>1) THEN
        RAISE EXCEPTION 'current case identifier is already occupied' USING ERRCODE='23505',CONSTRAINT='case_current_nuc_unique';
    END IF;
    IF EXISTS(SELECT 1 FROM (SELECT DISTINCT ON(case_id) judicial_case_number FROM %1$I.case_administration_revisions ORDER BY case_id,revision DESC) heads
        WHERE judicial_case_number IS NOT NULL GROUP BY judicial_case_number COLLATE "C" HAVING COUNT(*)>1) THEN
        RAISE EXCEPTION 'current case identifier is already occupied' USING ERRCODE='23505',CONSTRAINT='case_current_judicial_case_number_unique';
    END IF;
    SELECT COALESCE(c.required_initial_revision=1 AND r.nuc IS NOT NULL,FALSE) INTO required
        FROM %1$I.cases c JOIN %1$I.case_administration_revisions r ON r.case_id=c.id AND r.revision=1 WHERE c.id=NEW.case_id;
    SELECT EXISTS(SELECT 1 FROM %1$I.case_initial_stage_registrations WHERE case_id=NEW.case_id) INTO present;
    IF required IS DISTINCT FROM present THEN
        RAISE EXCEPTION 'initial case stage must match complete case registration' USING ERRCODE='23514',CONSTRAINT='case_initial_stage_required';
    END IF;
    RETURN NEW;
END; $$;
$definition$, current_schema());
    EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.validate_case_initial_stage_registration() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF pg_catalog.current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'case administration writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_catalog.pg_advisory_xact_lock(280603412820);
    IF NOT EXISTS(SELECT 1 FROM %1$I.cases c JOIN %1$I.case_administration_revisions r ON r.case_id=c.id AND r.revision=1
        WHERE c.id=NEW.case_id AND c.required_initial_revision=1 AND r.nuc IS NOT NULL) THEN
        RAISE EXCEPTION 'initial stage requires a complete new case registration' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$, current_schema());
END; $install$;
DROP TRIGGER IF EXISTS case_root_immutable ON cases;
CREATE TRIGGER case_root_immutable BEFORE UPDATE ON cases FOR EACH ROW EXECUTE FUNCTION preserve_case_administration_history();
DROP TRIGGER IF EXISTS case_administration_immutable ON case_administration_revisions;
CREATE TRIGGER case_administration_immutable BEFORE UPDATE ON case_administration_revisions FOR EACH ROW EXECUTE FUNCTION preserve_case_administration_history();
DROP TRIGGER IF EXISTS case_initial_stage_immutable ON case_initial_stage_registrations;
CREATE TRIGGER case_initial_stage_immutable BEFORE UPDATE ON case_initial_stage_registrations FOR EACH ROW EXECUTE FUNCTION preserve_case_administration_history();
DROP TRIGGER IF EXISTS case_administration_sequence ON case_administration_revisions;
CREATE TRIGGER case_administration_sequence BEFORE INSERT ON case_administration_revisions FOR EACH ROW EXECUTE FUNCTION enforce_case_administration_sequence();
DROP TRIGGER IF EXISTS case_administration_heads_valid ON case_administration_revisions;
CREATE CONSTRAINT TRIGGER case_administration_heads_valid AFTER INSERT ON case_administration_revisions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION validate_case_administration_heads();
DROP TRIGGER IF EXISTS case_initial_stage_valid ON case_initial_stage_registrations;
CREATE TRIGGER case_initial_stage_valid BEFORE INSERT ON case_initial_stage_registrations FOR EACH ROW EXECUTE FUNCTION validate_case_initial_stage_registration();
REVOKE ALL ON cases,case_administration_revisions,case_initial_stage_registrations FROM PUBLIC;
REVOKE ALL ON FUNCTION case_administration_text_valid(TEXT,INTEGER,BOOLEAN),
    case_administration_is_canonical(TEXT,TEXT,TEXT,TEXT,TEXT,TEXT,TEXT,TEXT[],TEXT,TEXT),
    case_administration_bytes(TEXT,TEXT,TEXT,TEXT,TEXT,TEXT,TEXT,TEXT[],TEXT,TEXT),
    preserve_case_administration_history(),enforce_case_administration_sequence(),
    validate_case_administration_heads(),validate_case_initial_stage_registration() FROM PUBLIC;
