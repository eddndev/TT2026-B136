DO $install$ BEGIN
EXECUTE format($definition$
CREATE OR REPLACE FUNCTION %1$I.validate_deadline_dispatch_cursor()
RETURNS TRIGGER LANGUAGE plpgsql SECURITY INVOKER SET search_path=pg_catalog AS $$
DECLARE event_changed BOOLEAN; bootstrap_changed BOOLEAN; next_event BIGINT;
    previous_after UUID; last_after UUID;
BEGIN
    IF NEW.singleton IS DISTINCT FROM OLD.singleton THEN
        RAISE EXCEPTION 'deadline dispatch cursor identity is immutable' USING ERRCODE='23514';
    END IF;
    event_changed:=ROW(NEW.completed_event_sequence,NEW.active_event_sequence,NEW.after_deadline_id)
        IS DISTINCT FROM ROW(OLD.completed_event_sequence,OLD.active_event_sequence,OLD.after_deadline_id);
    bootstrap_changed:=NEW.bootstrap_after_deadline_id IS DISTINCT FROM OLD.bootstrap_after_deadline_id;
    IF event_changed AND bootstrap_changed THEN
        RAISE EXCEPTION 'deadline dispatch advances one stream per update' USING ERRCODE='23514';
    END IF;
    IF NOT event_changed AND NOT bootstrap_changed THEN RETURN NEW; END IF;
    IF event_changed THEN
        SELECT min(e.sequence) INTO next_event FROM %1$I.deadline_source_events e
            WHERE e.sequence>coalesce(OLD.completed_event_sequence,0);
        IF next_event IS NULL OR (OLD.active_event_sequence IS NOT NULL
            AND OLD.active_event_sequence IS DISTINCT FROM next_event) THEN
            RAISE EXCEPTION 'deadline dispatch cannot skip the next stored event' USING ERRCODE='23514';
        END IF;
        previous_after:=OLD.after_deadline_id;
        IF NEW.active_event_sequence IS NULL THEN
            IF NEW.completed_event_sequence IS DISTINCT FROM next_event OR NEW.after_deadline_id IS NOT NULL THEN
                RAISE EXCEPTION 'deadline event completion must clear the exclusive position' USING ERRCODE='23514';
            END IF;
            last_after:=NULL;
        ELSE
            IF NEW.active_event_sequence IS DISTINCT FROM next_event
                OR NEW.completed_event_sequence IS DISTINCT FROM OLD.completed_event_sequence
                OR NEW.after_deadline_id IS NULL
                OR (previous_after IS NOT NULL AND NEW.after_deadline_id<=previous_after) THEN
                RAISE EXCEPTION 'deadline event position must advance exclusively' USING ERRCODE='23514';
            END IF;
            last_after:=NEW.after_deadline_id;
            IF NOT EXISTS(SELECT 1 FROM %1$I.deadline_dispatch_candidates(next_event,last_after,TRUE,last_after,FALSE,1) c
                JOIN %1$I.deadline_reevaluation_jobs j ON j.deadline_id=c.deadline_id
                    AND j.event_sequence=next_event
                WHERE c.deadline_id=last_after) THEN
                RAISE EXCEPTION 'deadline event position must name a dispatched current candidate' USING ERRCODE='23514';
            END IF;
            IF NOT EXISTS(SELECT 1 FROM %1$I.deadline_dispatch_candidates(next_event,last_after,FALSE,NULL,FALSE,1) c) THEN
                RAISE EXCEPTION 'deadline event without later candidates must complete' USING ERRCODE='23514';
            END IF;
        END IF;
        IF EXISTS(SELECT 1 FROM %1$I.deadline_dispatch_candidates(next_event,previous_after,FALSE,last_after,TRUE,1) c) THEN
            RAISE EXCEPTION 'deadline event cursor cannot skip an unassigned candidate' USING ERRCODE='23514';
        END IF;
    ELSE
        previous_after:=OLD.bootstrap_after_deadline_id;
        last_after:=NEW.bootstrap_after_deadline_id;
        IF last_after IS NOT NULL THEN
            IF previous_after IS NOT NULL AND last_after<=previous_after THEN
                RAISE EXCEPTION 'deadline legacy position must advance or finish its sweep' USING ERRCODE='23514';
            END IF;
            IF NOT EXISTS(SELECT 1 FROM %1$I.deadline_dispatch_candidates(NULL,last_after,TRUE,last_after,FALSE,1) c
                JOIN %1$I.deadline_reevaluation_jobs j ON j.deadline_id=c.deadline_id
                    AND j.event_sequence IS NULL AND j.bootstrap_policy_version=1
                WHERE c.deadline_id=last_after) THEN
                RAISE EXCEPTION 'deadline legacy position must name a dispatched current candidate' USING ERRCODE='23514';
            END IF;
            IF NOT EXISTS(SELECT 1 FROM %1$I.deadline_dispatch_candidates(NULL,last_after,FALSE,NULL,TRUE,1) c) THEN
                RAISE EXCEPTION 'deadline legacy sweep without later candidates must complete' USING ERRCODE='23514';
            END IF;
        END IF;
        IF EXISTS(SELECT 1 FROM %1$I.deadline_dispatch_candidates(NULL,previous_after,FALSE,last_after,TRUE,1) c) THEN
            RAISE EXCEPTION 'deadline legacy cursor cannot skip an unassigned candidate' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END; $$;
$definition$,current_schema());
END; $install$;
DROP TRIGGER IF EXISTS deadline_dispatch_lock ON deadline_dispatch_cursor;
CREATE TRIGGER deadline_dispatch_lock BEFORE UPDATE ON deadline_dispatch_cursor
    FOR EACH STATEMENT EXECUTE FUNCTION lock_deadline_dispatch();
DROP TRIGGER IF EXISTS deadline_dispatch_update ON deadline_dispatch_cursor;
CREATE TRIGGER deadline_dispatch_update BEFORE UPDATE ON deadline_dispatch_cursor
    FOR EACH ROW EXECUTE FUNCTION validate_deadline_dispatch_cursor();
DROP TRIGGER IF EXISTS deadline_dispatch_protected ON deadline_dispatch_cursor;
CREATE TRIGGER deadline_dispatch_protected BEFORE INSERT OR DELETE OR TRUNCATE ON deadline_dispatch_cursor
    FOR EACH STATEMENT EXECUTE FUNCTION preserve_deadline_dispatch();
REVOKE ALL ON FUNCTION validate_deadline_dispatch_cursor() FROM PUBLIC;
