-- Durable personal notification plans, evidence and provider handoffs.
CREATE TABLE IF NOT EXISTS alert_preferences (
    user_id UUID NOT NULL REFERENCES users(id),
    revision BIGINT NOT NULL CHECK(revision BETWEEN 1 AND 4294967295),
    operation_id UUID NOT NULL UNIQUE,
    recorded_seconds BIGINT NOT NULL,
    recorded_nanos INTEGER NOT NULL,
    payload BYTEA NOT NULL,
    payload_digest BYTEA NOT NULL,
    PRIMARY KEY(user_id,revision),
    CHECK(alert_time_valid(recorded_seconds,recorded_nanos)),
    CHECK(alert_payload_valid(payload,payload_digest))
);
CREATE TABLE IF NOT EXISTS alert_subject_state (
    kind SMALLINT NOT NULL CHECK(kind IN (0,1)),
    id UUID NOT NULL,
    case_id UUID NOT NULL REFERENCES cases(id),
    generation BIGINT NOT NULL CHECK(generation>0),
    dirty BOOLEAN NOT NULL,
    payload BYTEA NOT NULL,
    payload_digest BYTEA NOT NULL,
    PRIMARY KEY(kind,id),
    CHECK(alert_payload_valid(payload,payload_digest))
);
CREATE TABLE IF NOT EXISTS alert_scan_cursor (
    singleton BOOLEAN PRIMARY KEY CHECK(singleton),
    kind SMALLINT NOT NULL CHECK(kind IN (0,1)),
    id UUID,
    active_kind SMALLINT,
    active_id UUID,
    after_recipient UUID,
    cycle BIGINT NOT NULL CHECK(cycle>=0),
    next_seconds BIGINT,
    next_nanos INTEGER,
    CHECK(alert_optional_time_valid(next_seconds,next_nanos)),
    CHECK((active_kind IS NULL AND active_id IS NULL AND after_recipient IS NULL)
        OR (active_kind IN (0,1) AND active_id IS NOT NULL))
);
INSERT INTO alert_scan_cursor(singleton,kind,id,active_kind,active_id,after_recipient,cycle)
    VALUES(true,0,NULL,NULL,NULL,NULL,0) ON CONFLICT DO NOTHING;
CREATE TABLE IF NOT EXISTS alert_schedule (
    id UUID PRIMARY KEY,
    kind SMALLINT NOT NULL,
    subject_id UUID NOT NULL,
    case_id UUID NOT NULL REFERENCES cases(id),
    recipient UUID NOT NULL REFERENCES users(id),
    occurrence_key TEXT COLLATE "C" NOT NULL CHECK(length(occurrence_key) BETWEEN 1 AND 200),
    occurrence_id UUID NOT NULL,
    trigger_seconds BIGINT NOT NULL,
    trigger_nanos INTEGER NOT NULL,
    status TEXT COLLATE "C" NOT NULL CHECK(status IN ('planned','activated','superseded')),
    generation BIGINT NOT NULL CHECK(generation>0),
    payload BYTEA NOT NULL,
    payload_digest BYTEA NOT NULL,
    UNIQUE(recipient,occurrence_key),
    FOREIGN KEY(kind,subject_id) REFERENCES alert_subject_state(kind,id),
    CHECK(alert_time_valid(trigger_seconds,trigger_nanos)),
    CHECK(alert_payload_valid(payload,payload_digest))
);
CREATE INDEX IF NOT EXISTS alert_schedule_due ON alert_schedule(status,trigger_seconds,trigger_nanos,id);
CREATE INDEX IF NOT EXISTS alert_schedule_subject ON alert_schedule(kind,subject_id,recipient);
CREATE TABLE IF NOT EXISTS alert_notifications (
    id UUID PRIMARY KEY,
    schedule_id UUID NOT NULL UNIQUE REFERENCES alert_schedule(id),
    recipient UUID NOT NULL REFERENCES users(id),
    kind SMALLINT NOT NULL,
    subject_id UUID NOT NULL,
    case_id UUID NOT NULL REFERENCES cases(id),
    created_seconds BIGINT NOT NULL,
    created_nanos INTEGER NOT NULL,
    internal_enabled BOOLEAN NOT NULL,
    payload BYTEA NOT NULL,
    payload_digest BYTEA NOT NULL,
    read_seconds BIGINT,
    read_nanos INTEGER,
    resolved_seconds BIGINT,
    resolved_nanos INTEGER,
    resolved_reason TEXT COLLATE "C",
    FOREIGN KEY(kind,subject_id) REFERENCES alert_subject_state(kind,id),
    CHECK(alert_time_valid(created_seconds,created_nanos)),
    CHECK(alert_optional_time_valid(read_seconds,read_nanos)),
    CHECK(alert_optional_time_valid(resolved_seconds,resolved_nanos)),
    CHECK((resolved_seconds IS NULL)=(resolved_reason IS NULL)),
    CHECK(resolved_reason IS NULL OR resolved_reason IN
        ('superseded','attention_recorded','target_retired','cancelled_hearing','no_longer_eligible')),
    CHECK(alert_payload_valid(payload,payload_digest))
);
CREATE INDEX IF NOT EXISTS alert_inbox_order ON alert_notifications(recipient,created_seconds,created_nanos,id);
CREATE TABLE IF NOT EXISTS alert_read_receipts (
    operation_id UUID PRIMARY KEY,
    recipient UUID NOT NULL REFERENCES users(id),
    alert_id UUID NOT NULL REFERENCES alert_notifications(id),
    read_seconds BIGINT NOT NULL,
    read_nanos INTEGER NOT NULL,
    CHECK(alert_time_valid(read_seconds,read_nanos))
);
CREATE TABLE IF NOT EXISTS alert_email_outbox (
    id UUID PRIMARY KEY,
    alert_id UUID NOT NULL UNIQUE REFERENCES alert_notifications(id),
    status TEXT COLLATE "C" NOT NULL CHECK(status IN
        ('pending','sending','accepted','failed','unknown','cancelled','disabled')),
    next_seconds BIGINT,
    next_nanos INTEGER,
    sequence BIGINT NOT NULL CHECK(sequence>=0),
    payload BYTEA NOT NULL,
    payload_digest BYTEA NOT NULL,
    CHECK(alert_optional_time_valid(next_seconds,next_nanos)),
    CHECK(alert_payload_valid(payload,payload_digest))
);
CREATE INDEX IF NOT EXISTS alert_outbox_due ON alert_email_outbox(status,next_seconds,next_nanos,id);
CREATE TABLE IF NOT EXISTS alert_email_attempts (
    delivery_id UUID NOT NULL REFERENCES alert_email_outbox(id),
    sequence BIGINT NOT NULL CHECK(sequence>=0),
    payload BYTEA NOT NULL,
    payload_digest BYTEA NOT NULL,
    PRIMARY KEY(delivery_id,sequence),
    CHECK(alert_payload_valid(payload,payload_digest))
);
REVOKE ALL ON alert_preferences,alert_subject_state,alert_scan_cursor,alert_schedule,
    alert_notifications,alert_read_receipts,alert_email_outbox,alert_email_attempts FROM PUBLIC;
