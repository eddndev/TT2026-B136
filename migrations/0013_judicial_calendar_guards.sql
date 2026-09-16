-- Independent write guards protect the same exact history as the audited adapter.
CREATE OR REPLACE FUNCTION preserve_judicial_calendar_history()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'judicial calendar history is immutable' USING ERRCODE='23514';
END; $$;
DO $$ DECLARE tab TEXT; BEGIN
    FOREACH tab IN ARRAY ARRAY['judicial_calendars','judicial_calendar_revisions'] LOOP
        IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=tab::regclass AND tgname='judicial_calendar_immutable') THEN
            EXECUTE format('CREATE TRIGGER judicial_calendar_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION preserve_judicial_calendar_history()',tab);
        END IF;
    END LOOP;
END; $$;
DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.enforce_judicial_calendar_sequence()
RETURNS TRIGGER LANGUAGE plpgsql SET search_path=pg_catalog AS $$
DECLARE previous RECORD; actor RECORD; initial JSONB; value JSONB;
BEGIN
    IF current_setting('transaction_isolation')<>'read committed' THEN
        RAISE EXCEPTION 'judicial calendar writes require read committed isolation' USING ERRCODE='23514';
    END IF;
    PERFORM pg_advisory_xact_lock(280603412820);
    SELECT email,role,active INTO actor FROM %1$I.users WHERE id=NEW.recorded_by FOR SHARE;
    IF actor.active IS DISTINCT FROM TRUE OR actor.role IS DISTINCT FROM 'owner'
        OR actor.email COLLATE "C" IS DISTINCT FROM NEW.recorded_by_email COLLATE "C" THEN
        RAISE EXCEPTION 'judicial calendar actor is not currently authorized' USING ERRCODE='42501';
    END IF;
    SELECT * INTO previous FROM %1$I.judicial_calendar_revisions
        WHERE calendar_id=NEW.calendar_id ORDER BY revision DESC LIMIT 1;
    IF NEW.revision IS DISTINCT FROM coalesce(previous.revision+1,1)
        OR (previous.revision IS NULL AND NEW.action<>'publish')
        OR (previous.revision IS NOT NULL AND (NEW.action='publish' OR previous.status<>'published')) THEN
        RAISE EXCEPTION 'calendar revisions must follow a published predecessor' USING ERRCODE='23514';
    END IF;
    value:=%1$I.judicial_calendar_values(NEW.values_canonical);
    IF previous.revision IS NOT NULL THEN
        SELECT values_view->'scope' INTO initial FROM %1$I.judicial_calendar_revisions
            WHERE calendar_id=NEW.calendar_id AND revision=1;
        IF initial IS DISTINCT FROM value->'scope' THEN
            RAISE EXCEPTION 'calendar scope must remain identical to first revision' USING ERRCODE='23514';
        END IF;
    END IF;
    IF NEW.action='retire' AND ROW(NEW.values_canonical,NEW.values_digest)
        IS DISTINCT FROM ROW(previous.values_canonical,previous.values_digest) THEN
        RAISE EXCEPTION 'calendar retirement must preserve exact prior values' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DROP TRIGGER IF EXISTS judicial_calendar_sequence ON judicial_calendar_revisions;
CREATE TRIGGER judicial_calendar_sequence BEFORE INSERT ON judicial_calendar_revisions
    FOR EACH ROW EXECUTE FUNCTION enforce_judicial_calendar_sequence();
REVOKE ALL ON FUNCTION preserve_judicial_calendar_history(),enforce_judicial_calendar_sequence() FROM PUBLIC;
