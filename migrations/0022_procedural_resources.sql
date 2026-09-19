-- Resources, declared acts and receipts are immutable independent histories.
CREATE TABLE IF NOT EXISTS case_procedural_resources (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL REFERENCES cases(id),
    initial_revision BIGINT NOT NULL DEFAULT 1 CHECK(initial_revision=1),
    UNIQUE(id,case_id)
);
CREATE TABLE IF NOT EXISTS case_procedural_resource_acts (
    id UUID PRIMARY KEY,
    resource_id UUID NOT NULL,
    case_id UUID NOT NULL,
    initial_resource_revision BIGINT NOT NULL CHECK((initial_resource_revision>=2 AND initial_resource_revision<=4294967295)),
    UNIQUE(id,resource_id,case_id),
    FOREIGN KEY(resource_id,case_id) REFERENCES case_procedural_resources(id,case_id)
);
CREATE TABLE IF NOT EXISTS case_procedural_resource_revisions (
    resource_id UUID NOT NULL,
    case_id UUID NOT NULL,
    revision BIGINT NOT NULL CHECK((revision>=1 AND revision<=4294967295)),
    operation_id UUID NOT NULL CONSTRAINT procedural_resource_operation_unique UNIQUE,
    action TEXT NOT NULL CHECK(action COLLATE "C" IN ('register','correct','record_act','correct_act','archive','reactivate')),
    status TEXT NOT NULL CHECK(status COLLATE "C" IN ('active','archived')),
    kind TEXT NOT NULL CHECK(kind COLLATE "C" IN ('revocation','appeal')),
    reason TEXT CHECK(reason IS NULL OR ((char_length(reason)>=1 AND char_length(reason)<=1000) AND octet_length(reason)<=4000)),
    values_canonical BYTEA NOT NULL CHECK((octet_length(values_canonical)>=5 AND octet_length(values_canonical)<=262144)),
    values_view JSONB NOT NULL CHECK(octet_length(values_view::text)<=524288),
    values_digest BYTEA NOT NULL CHECK(values_digest=pg_catalog.sha256(values_canonical)),
    sources_canonical BYTEA NOT NULL CHECK((octet_length(sources_canonical)>=5 AND octet_length(sources_canonical)<=524288)),
    sources_digest BYTEA NOT NULL CHECK(sources_digest=pg_catalog.sha256(sources_canonical)),
    supports_view JSONB NOT NULL CHECK(jsonb_typeof(supports_view)='array' AND jsonb_array_length(supports_view)<=2 AND octet_length(supports_view::text)<=16384),
    submission_canonical BYTEA NOT NULL CHECK((octet_length(submission_canonical)>=5 AND octet_length(submission_canonical)<=1048576)),
    submission_digest BYTEA NOT NULL CHECK(submission_digest=pg_catalog.sha256(submission_canonical)),
    capture_canonical BYTEA NOT NULL CHECK(octet_length(capture_canonical)=57),
    capture_digest BYTEA NOT NULL CHECK(capture_digest=pg_catalog.sha256(capture_canonical)),
    previous_capture_digest BYTEA CHECK(octet_length(previous_capture_digest)=32),
    act_id UUID,
    act_revision BIGINT CHECK((act_revision>=1 AND act_revision<=4294967295)),
    act_values_canonical BYTEA CHECK((octet_length(act_values_canonical)>=5 AND octet_length(act_values_canonical)<=32768)),
    act_values_view JSONB CHECK(octet_length(act_values_view::text)<=65536),
    act_supports_view JSONB CHECK(jsonb_typeof(act_supports_view)='array' AND jsonb_array_length(act_supports_view)<=2 AND octet_length(act_supports_view::text)<=16384),
    act_previous_resource_revision BIGINT CHECK((act_previous_resource_revision>=2 AND act_previous_resource_revision<=4294967295)),
    act_previous_capture_digest BYTEA CHECK(octet_length(act_previous_capture_digest)=32),
    recorded_administration_revision BIGINT,
    recorded_administration_digest BYTEA,
    recorded_administration_title TEXT CHECK(octet_length(recorded_administration_title)<=800),
    recorded_administration_reference TEXT CHECK(octet_length(recorded_administration_reference)<=400),
    recorded_stage_revision BIGINT CHECK((recorded_stage_revision>=1 AND recorded_stage_revision<=4294967295)),
    recorded_at_seconds BIGINT NOT NULL CHECK((recorded_at_seconds>=-62135596800 AND recorded_at_seconds<=253402300799)),
    recorded_at_nanoseconds INTEGER NOT NULL CHECK((recorded_at_nanoseconds>=0 AND recorded_at_nanoseconds<=999999999)),
    recorded_by UUID NOT NULL REFERENCES users(id),
    recorded_by_email TEXT NOT NULL CHECK((octet_length(recorded_by_email)>=1 AND octet_length(recorded_by_email)<=1280)),
    PRIMARY KEY(resource_id,revision),
    UNIQUE(resource_id,case_id,revision),
    UNIQUE(act_id,act_revision),
    FOREIGN KEY(resource_id,case_id) REFERENCES case_procedural_resources(id,case_id),
    FOREIGN KEY(act_id,resource_id,case_id) REFERENCES case_procedural_resource_acts(id,resource_id,case_id),
    FOREIGN KEY(case_id,recorded_administration_revision) REFERENCES case_administration_revisions(case_id,revision),
    CONSTRAINT procedural_resource_administration_shape CHECK(
        (recorded_administration_revision IS NULL AND recorded_administration_digest IS NULL
            AND recorded_administration_title IS NOT NULL AND recorded_administration_reference IS NOT NULL)
        OR ((recorded_administration_revision>=1 AND recorded_administration_revision<=4294967295) AND recorded_administration_revision IS NOT NULL
            AND recorded_administration_digest IS NOT NULL AND octet_length(recorded_administration_digest)=32
            AND recorded_administration_title IS NULL AND recorded_administration_reference IS NULL)),
    CONSTRAINT procedural_resource_action_shape CHECK(
        (action='register' AND revision=1 AND previous_capture_digest IS NULL AND reason IS NULL)
        OR (action<>'register' AND revision>1 AND previous_capture_digest IS NOT NULL
            AND ((action='record_act' AND reason IS NULL) OR (action<>'record_act' AND reason IS NOT NULL)))),
    CONSTRAINT procedural_resource_status_shape CHECK((action='archive')=(status='archived')),
    CONSTRAINT procedural_resource_act_shape CHECK(
        (action IN ('record_act','correct_act') AND act_id IS NOT NULL AND act_revision IS NOT NULL
            AND act_values_canonical IS NOT NULL AND act_values_view IS NOT NULL AND act_supports_view IS NOT NULL
            AND ((action='record_act' AND act_revision=1 AND act_previous_resource_revision IS NULL AND act_previous_capture_digest IS NULL)
                OR (action='correct_act' AND act_revision>1 AND act_previous_resource_revision IS NOT NULL AND act_previous_capture_digest IS NOT NULL)))
        OR (action NOT IN ('record_act','correct_act') AND act_id IS NULL AND act_revision IS NULL
            AND act_values_canonical IS NULL AND act_values_view IS NULL AND act_supports_view IS NULL
            AND act_previous_resource_revision IS NULL AND act_previous_capture_digest IS NULL))
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_procedural_resources'::regclass AND conname='procedural_resource_first_revision') THEN
        ALTER TABLE case_procedural_resources ADD CONSTRAINT procedural_resource_first_revision FOREIGN KEY(id,initial_revision)
            REFERENCES case_procedural_resource_revisions(resource_id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_procedural_resource_acts'::regclass AND conname='procedural_resource_act_first_revision') THEN
        ALTER TABLE case_procedural_resource_acts ADD CONSTRAINT procedural_resource_act_first_revision FOREIGN KEY(resource_id,case_id,initial_resource_revision)
            REFERENCES case_procedural_resource_revisions(resource_id,case_id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
END; $$;
CREATE INDEX IF NOT EXISTS procedural_resource_case_order ON case_procedural_resources(case_id,id);
REVOKE ALL ON case_procedural_resources,case_procedural_resource_acts,case_procedural_resource_revisions FROM PUBLIC;
