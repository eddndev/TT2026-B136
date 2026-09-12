CREATE TABLE IF NOT EXISTS cases (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL CHECK (
        char_length(title) BETWEEN 1 AND 200
        AND title = btrim(title)
        AND title !~ '[[:cntrl:]]'
    ),
    reference TEXT NOT NULL CHECK (
        char_length(reference) BETWEEN 1 AND 100
        AND reference = btrim(reference)
        AND reference !~ '[[:cntrl:]]'
    ),
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS case_memberships (
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (case_id, user_id)
);

CREATE INDEX IF NOT EXISTS case_memberships_user_case_idx
    ON case_memberships(user_id, case_id);
