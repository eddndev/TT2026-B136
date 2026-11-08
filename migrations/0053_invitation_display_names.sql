-- Display name of the invited person, written by the Owner at issue and copied
-- to the account at acceptance: docs/adr/0087-account-display-names.md.
-- Existing invitations receive the local part of their email as a placeholder,
-- trimmed and cut to 120 characters, or "cuenta" when nothing remains.
-- Repeating this migration changes nothing. The runtime role gets INSERT but
-- never UPDATE on the column, so a resend keeps the name.
ALTER TABLE member_invitations ADD COLUMN IF NOT EXISTS display_name TEXT;
UPDATE member_invitations SET display_name=COALESCE(
        NULLIF(btrim(left(btrim(split_part(email,'@',1)),120)),''),'cuenta')
    WHERE display_name IS NULL;
ALTER TABLE member_invitations ALTER COLUMN display_name SET NOT NULL;
DO $migration$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid='member_invitations'::regclass
            AND conname='member_invitations_display_name_check') THEN
        ALTER TABLE member_invitations ADD CONSTRAINT member_invitations_display_name_check
            CHECK (display_name = btrim(display_name)
                AND char_length(display_name) BETWEEN 1 AND 120);
    END IF;
END;
$migration$;
