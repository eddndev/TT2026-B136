DO $$ BEGIN
    IF EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_measure_operations'::regclass
        AND conname='measure_operation_payload' AND contype='f') THEN
        ALTER TABLE case_measure_operations DROP CONSTRAINT measure_operation_payload;
        CREATE CONSTRAINT TRIGGER measure_operation_payload AFTER INSERT ON case_measure_operations
            DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION enforce_measure_decision_complete();
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_measure_administrations'::regclass
        AND tgname='measure_decision_immutable') THEN
        CREATE TRIGGER measure_decision_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON case_measure_administrations
            FOR EACH STATEMENT EXECUTE FUNCTION preserve_measure_decision_history();
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_measure_administrations'::regclass
        AND tgname='measure_decision_lock') THEN
        CREATE TRIGGER measure_decision_lock BEFORE INSERT ON case_measure_administrations
            FOR EACH STATEMENT EXECUTE FUNCTION lock_measure_decision_history();
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_measure_administrations'::regclass
        AND tgname='measure_decision_complete') THEN
        CREATE CONSTRAINT TRIGGER measure_decision_complete AFTER INSERT ON case_measure_administrations
            DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION enforce_measure_decision_complete();
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid='case_measure_administrations'::regclass
        AND tgname='measure_administration_capture') THEN
        CREATE TRIGGER measure_administration_capture BEFORE INSERT ON case_measure_administrations
            FOR EACH ROW EXECUTE FUNCTION enforce_measure_administration_capture();
    END IF;
END; $$;
