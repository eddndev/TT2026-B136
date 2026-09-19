-- Completion and failed attempts are independent immutable ledgers.
CREATE TABLE IF NOT EXISTS deadline_reevaluation_results (
    job_id UUID CONSTRAINT deadline_worker_result_primary PRIMARY KEY,
    base_revision BIGINT NOT NULL,
    base_submission_digest BYTEA NOT NULL,
    base_capture_digest BYTEA NOT NULL,
    outcome TEXT NOT NULL,
    result_revision BIGINT,
    result_submission_digest BYTEA,
    result_capture_digest BYTEA,
    checked_observations_canonical BYTEA,
    checked_administration_revision BIGINT,
    checked_administration_evidence_digest BYTEA,
    completed_at_seconds BIGINT NOT NULL,
    completed_at_nanoseconds INTEGER NOT NULL,
    CONSTRAINT deadline_worker_result_job FOREIGN KEY(job_id) REFERENCES deadline_reevaluation_jobs(id),
    CONSTRAINT deadline_worker_result_base CHECK((base_revision>=1 AND base_revision<=4294967295
        AND octet_length(base_submission_digest)=32 AND octet_length(base_capture_digest)=32) IS TRUE),
    CONSTRAINT deadline_worker_result_outcome CHECK((outcome COLLATE "C" IN
        ('revision','retired','already_observed','dependency_not_selected','already_initialized')) IS TRUE),
    CONSTRAINT deadline_worker_result_shape CHECK((
        (outcome='revision' AND result_revision=base_revision+1 AND result_revision<=4294967295
            AND octet_length(result_submission_digest)=32 AND octet_length(result_capture_digest)=32
            AND num_nonnulls(checked_observations_canonical,checked_administration_revision,
                checked_administration_evidence_digest)=0)
        OR (outcome IN ('retired','already_initialized')
            AND num_nonnulls(result_revision,result_submission_digest,result_capture_digest,
                checked_observations_canonical,checked_administration_revision,
                checked_administration_evidence_digest)=0)
        OR (outcome IN ('already_observed','dependency_not_selected')
            AND num_nonnulls(result_revision,result_submission_digest,result_capture_digest)=0
            AND octet_length(checked_observations_canonical)>=111 AND octet_length(checked_observations_canonical)<=446
            AND substring(checked_observations_canonical FROM 1 FOR 5)=convert_to('DLOB1','UTF8')
            AND (checked_administration_revision IS NULL
                OR checked_administration_revision>=1 AND checked_administration_revision<=4294967295)
            AND octet_length(checked_administration_evidence_digest)=32)) IS TRUE),
    CONSTRAINT deadline_worker_result_seconds CHECK(completed_at_seconds>=-62135596800 AND completed_at_seconds<=253402300799),
    CONSTRAINT deadline_worker_result_nanoseconds CHECK(completed_at_nanoseconds>=0 AND completed_at_nanoseconds<=999999999)
);
CREATE TABLE IF NOT EXISTS deadline_reevaluation_attempts (
    attempt_id UUID CONSTRAINT deadline_worker_attempt_primary PRIMARY KEY,
    job_id UUID NOT NULL,
    attempt_number BIGINT NOT NULL,
    checked_base_revision BIGINT,
    checked_base_submission_digest BYTEA,
    checked_base_capture_digest BYTEA,
    failure_kind TEXT NOT NULL,
    error_code TEXT NOT NULL,
    failed_at_seconds BIGINT NOT NULL,
    failed_at_nanoseconds INTEGER NOT NULL,
    retry_at_seconds BIGINT NOT NULL,
    retry_at_nanoseconds INTEGER NOT NULL,
    CONSTRAINT deadline_worker_attempt_job FOREIGN KEY(job_id) REFERENCES deadline_reevaluation_jobs(id),
    CONSTRAINT deadline_worker_attempt_number UNIQUE(job_id,attempt_number),
    CONSTRAINT deadline_worker_attempt_positive CHECK(attempt_number>0),
    CONSTRAINT deadline_worker_attempt_base CHECK((
        num_nonnulls(checked_base_revision,checked_base_submission_digest,checked_base_capture_digest)=0
        OR (checked_base_revision>=1 AND checked_base_revision<=4294967295
            AND octet_length(checked_base_submission_digest)=32
            AND octet_length(checked_base_capture_digest)=32)) IS TRUE),
    CONSTRAINT deadline_worker_attempt_failure CHECK((
        (failure_kind COLLATE "C"='inconsistent' AND error_code COLLATE "C" IN
            ('invalid_stored_evidence','invalid_durable_job'))
        OR (failure_kind COLLATE "C"='transient' AND error_code COLLATE "C" IN
            ('lock_unavailable','database_unavailable','transaction_interrupted','execution_failed'))) IS TRUE),
    CONSTRAINT deadline_worker_attempt_seconds CHECK(
        failed_at_seconds>=-62135596800 AND failed_at_seconds<=253402300799
        AND retry_at_seconds>=-62135596800 AND retry_at_seconds<=253402300799),
    CONSTRAINT deadline_worker_attempt_nanoseconds CHECK(
        failed_at_nanoseconds>=0 AND failed_at_nanoseconds<=999999999 AND retry_at_nanoseconds>=0 AND retry_at_nanoseconds<=999999999),
    CONSTRAINT deadline_worker_attempt_retry CHECK(
        ROW(retry_at_seconds,retry_at_nanoseconds)>ROW(failed_at_seconds,failed_at_nanoseconds))
);
CREATE INDEX IF NOT EXISTS deadline_worker_attempt_latest
    ON deadline_reevaluation_attempts(job_id,attempt_number DESC);
REVOKE ALL ON deadline_reevaluation_results,deadline_reevaluation_attempts FROM PUBLIC;
