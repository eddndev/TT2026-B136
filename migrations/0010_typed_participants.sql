-- Canonical values are immutable; generated JSON is an independently checked projection.
CREATE TABLE IF NOT EXISTS case_subjects (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL REFERENCES cases(id),
    initial_revision BIGINT NOT NULL DEFAULT 1 CHECK(initial_revision=1)
);
CREATE TABLE IF NOT EXISTS case_subject_revisions (
    subject_id UUID NOT NULL REFERENCES case_subjects(id),
    revision BIGINT NOT NULL CHECK(revision BETWEEN 1 AND 4294967295),
    values_canonical BYTEA NOT NULL CHECK(octet_length(values_canonical)<=5676),
    values_digest BYTEA NOT NULL CHECK(values_digest=pg_catalog.sha256(values_canonical)),
    values_view JSONB GENERATED ALWAYS AS (typed_subject_values(values_canonical)) STORED,
    subject_kind TEXT NOT NULL,
    display_name TEXT NOT NULL,
    name_known BOOLEAN NOT NULL,
    declared_identifier TEXT,
    identity_document_id UUID NOT NULL,
    identity_document_version BIGINT NOT NULL,
    identity_document_digest BYTEA NOT NULL,
    identity_document_locator TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    changed_by UUID NOT NULL REFERENCES users(id),
    changed_by_email TEXT NOT NULL,
    PRIMARY KEY(subject_id,revision),
    CONSTRAINT subject_document_fk FOREIGN KEY(identity_document_id,identity_document_version) REFERENCES documents(id,version),
    CONSTRAINT subject_projection CHECK(
        subject_kind=values_view->>'kind'
        AND display_name=CASE subject_kind WHEN 'natural_person' THEN COALESCE(values_view->'name'->>'known',values_view->'name'->>'label') ELSE values_view->>'name' END
        AND name_known=(subject_kind='institutional_body' OR values_view->'name'?'known')
        AND declared_identifier IS NOT DISTINCT FROM CASE subject_kind WHEN 'natural_person' THEN values_view->'curp'->>'known' ELSE values_view->'institutional_identifier'->>'known' END
        AND identity_document_id=(values_view->'identity_support'->>'document_id')::uuid
        AND identity_document_version=(values_view->'identity_support'->>'version')::bigint
        AND identity_document_digest=decode(values_view->'identity_support'->>'digest','hex')
        AND identity_document_locator=values_view->'identity_support'->>'locator')
);
CREATE TABLE IF NOT EXISTS case_participant_typed_revisions (
    participant_id UUID NOT NULL REFERENCES case_participants(id),
    revision BIGINT NOT NULL CHECK(revision BETWEEN 1 AND 4294967295),
    values_canonical BYTEA NOT NULL CHECK(octet_length(values_canonical)<=8380),
    values_digest BYTEA NOT NULL CHECK(values_digest=pg_catalog.sha256(values_canonical)),
    values_view JSONB GENERATED ALWAYS AS (typed_participant_values(values_canonical)) STORED,
    subject_id UUID NOT NULL,
    subject_revision BIGINT NOT NULL,
    role_kind TEXT NOT NULL,
    organization TEXT,
    directory_status TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    changed_by UUID NOT NULL REFERENCES users(id),
    changed_by_email TEXT NOT NULL,
    submission_digest BYTEA NOT NULL CHECK(octet_length(submission_digest)=32),
    submission_revision BIGINT NOT NULL CHECK(submission_revision BETWEEN 1 AND revision),
    credential_origin_revision BIGINT CHECK(credential_origin_revision BETWEEN 1 AND revision),
    PRIMARY KEY(participant_id,revision),
    CONSTRAINT typed_subject_fk FOREIGN KEY(subject_id,subject_revision) REFERENCES case_subject_revisions(subject_id,revision),
    CONSTRAINT typed_projection CHECK(subject_id=(values_view->'subject'->>'id')::uuid
        AND subject_revision=(values_view->'subject'->>'revision')::bigint
        AND role_kind=values_view->'profile'->>'kind'
        AND organization IS NOT DISTINCT FROM values_view->>'organization'
        AND directory_status=values_view->>'directory_status'),
    CONSTRAINT typed_initial_active CHECK(revision<>1 OR directory_status='active')
);
CREATE TABLE IF NOT EXISTS subject_identity_reviews (
    subject_id UUID NOT NULL,
    revision BIGINT NOT NULL,
    review_canonical BYTEA NOT NULL CHECK(octet_length(review_canonical)<=28941),
    review_digest BYTEA NOT NULL CHECK(review_digest=pg_catalog.sha256(review_canonical)),
    PRIMARY KEY(subject_id,revision),
    FOREIGN KEY(subject_id,revision) REFERENCES case_subject_revisions(subject_id,revision) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE IF NOT EXISTS participant_identity_reviews (
    participant_id UUID NOT NULL,
    revision BIGINT NOT NULL,
    review_canonical BYTEA NOT NULL CHECK(octet_length(review_canonical)<=28941),
    review_digest BYTEA NOT NULL CHECK(review_digest=pg_catalog.sha256(review_canonical)),
    submission_canonical BYTEA NOT NULL CHECK(octet_length(submission_canonical) IN (167,615)),
    submission_digest BYTEA NOT NULL CHECK(submission_digest=pg_catalog.sha256(submission_canonical)),
    PRIMARY KEY(participant_id,revision),
    FOREIGN KEY(participant_id,revision) REFERENCES case_participant_typed_revisions(participant_id,revision) DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE IF NOT EXISTS participant_credential_evidence (
    participant_id UUID NOT NULL,
    revision BIGINT NOT NULL,
    deployment_id UUID NOT NULL,
    trust_revision BIGINT NOT NULL,
    declaration BYTEA NOT NULL CHECK(octet_length(declaration)=218),
    statement_digest BYTEA NOT NULL CHECK(statement_digest=pg_catalog.sha256(declaration)),
    certificate_der BYTEA NOT NULL CHECK(octet_length(certificate_der) BETWEEN 1 AND 16384),
    certificate_fingerprint BYTEA NOT NULL CHECK(certificate_fingerprint=pg_catalog.sha256(certificate_der)),
    signature BYTEA NOT NULL CHECK(octet_length(signature)=384),
    checked_at BIGINT NOT NULL,
    valid_from BIGINT NOT NULL,
    valid_until BIGINT NOT NULL,
    accepted_at_seconds BIGINT NOT NULL,
    accepted_at_nanoseconds INTEGER NOT NULL CHECK(accepted_at_nanoseconds BETWEEN 0 AND 999999999),
    PRIMARY KEY(participant_id,revision),
    CONSTRAINT participant_credential_window CHECK(valid_from<=checked_at AND checked_at<=valid_until
        AND valid_from<=accepted_at_seconds AND accepted_at_seconds<=valid_until),
    CONSTRAINT participant_credential_revision_fk FOREIGN KEY(participant_id,revision)
        REFERENCES case_participant_typed_revisions(participant_id,revision) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT participant_credential_trust_fk FOREIGN KEY(deployment_id,trust_revision)
        REFERENCES participant_credential_trust_revisions(deployment_id,revision)
);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_subjects'::regclass AND conname='subject_first_revision_fk') THEN
        ALTER TABLE case_subjects ADD CONSTRAINT subject_first_revision_fk FOREIGN KEY(id,initial_revision)
            REFERENCES case_subject_revisions(subject_id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_subject_revisions'::regclass AND conname='subject_review_fk') THEN
        ALTER TABLE case_subject_revisions ADD CONSTRAINT subject_review_fk FOREIGN KEY(subject_id,revision)
            REFERENCES subject_identity_reviews(subject_id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
    IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='case_participant_typed_revisions'::regclass AND conname='typed_review_origin_fk') THEN
        ALTER TABLE case_participant_typed_revisions ADD CONSTRAINT typed_review_origin_fk FOREIGN KEY(participant_id,submission_revision)
            REFERENCES participant_identity_reviews(participant_id,revision) DEFERRABLE INITIALLY DEFERRED;
        ALTER TABLE case_participant_typed_revisions ADD CONSTRAINT typed_credential_origin_fk FOREIGN KEY(participant_id,credential_origin_revision)
            REFERENCES participant_credential_evidence(participant_id,revision) DEFERRABLE INITIALLY DEFERRED;
    END IF;
END; $$;
CREATE INDEX IF NOT EXISTS subject_case_order ON case_subjects(case_id,id);
CREATE INDEX IF NOT EXISTS typed_subject_role ON case_participant_typed_revisions(subject_id,role_kind,participant_id,revision DESC);
CREATE INDEX IF NOT EXISTS participant_certificate_fingerprint ON participant_credential_evidence(certificate_fingerprint,participant_id);
REVOKE ALL ON case_subjects,case_subject_revisions,case_participant_typed_revisions,
    subject_identity_reviews,participant_identity_reviews,participant_credential_evidence FROM PUBLIC;
