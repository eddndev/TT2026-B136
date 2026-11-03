-- Let an Owner invite a Client account, the counterparty of a bilateral
-- contract: docs/adr/0086-bilateral-contract-signing.md. Existing invitations
-- keep their roles; only the role constraint widens.
DO $$ BEGIN
    IF EXISTS(SELECT 1 FROM pg_constraint c WHERE c.conrelid='member_invitations'::regclass
        AND c.conname='member_invitations_role_check' AND c.contype='c'
        AND pg_get_expr(c.conbin,c.conrelid)=
            '(role = ANY (ARRAY[''litigator''::text, ''paralegal''::text]))') THEN
        ALTER TABLE member_invitations DROP CONSTRAINT member_invitations_role_check;
        ALTER TABLE member_invitations ADD CONSTRAINT member_invitations_role_check
            CHECK(role IN ('litigator','paralegal','client'));
    END IF;
END; $$;
