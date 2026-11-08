-- Required display name of an office account, written by the Owner at creation:
-- docs/adr/0087-account-display-names.md. Existing rows receive the local part of
-- their email as a placeholder, trimmed and cut to 120 characters, or "cuenta"
-- when nothing remains. Repeating this migration changes nothing. The runtime
-- role gets INSERT but never UPDATE on the column, so the name is immutable.
ALTER TABLE users ADD COLUMN IF NOT EXISTS display_name TEXT;
UPDATE users SET display_name=COALESCE(
        NULLIF(btrim(left(btrim(split_part(email,'@',1)),120)),''),'cuenta')
    WHERE display_name IS NULL;
ALTER TABLE users ALTER COLUMN display_name SET NOT NULL;
DO $migration$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid='users'::regclass
            AND conname='member_display_name_check') THEN
        ALTER TABLE users ADD CONSTRAINT member_display_name_check
            CHECK (display_name = btrim(display_name)
                AND char_length(display_name) BETWEEN 1 AND 120);
    END IF;
END;
$migration$;
