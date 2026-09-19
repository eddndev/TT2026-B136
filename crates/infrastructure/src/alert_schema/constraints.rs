use super::{
    checks, incomplete, port,
    specifications::{FOREIGN_KEYS, KEYS},
    TABLES,
};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for &(table, name, columns, primary) in KEYS {
        let kind = if primary { "p" } else { "u" };
        let valid: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_constraint c
            JOIN pg_class t ON t.oid=c.conrelid JOIN pg_class i ON i.oid=c.conindid
            WHERE c.conrelid=$1::text::regclass AND c.conname=$2 AND c.contype::text=$3
                AND c.convalidated AND NOT c.condeferrable AND NOT c.condeferred
                AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
                AND c.connamespace=t.relnamespace
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
                AND i.relname=c.conname AND i.relnamespace=t.relnamespace
                AND ARRAY(SELECT a.attname::text FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
                    JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num
                    ORDER BY k.n)=$4::text[])",
                &[&table, &name, &kind, &columns],
            )
            .map_err(port)?
            .get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for &(table, name, columns, target, targets) in FOREIGN_KEYS {
        let valid: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_constraint c JOIN pg_class t ON t.oid=c.conrelid
            WHERE c.conrelid=$1::text::regclass AND c.conname=$2 AND c.contype='f'
                AND c.confrelid=$4::text::regclass
                AND c.convalidated AND NOT c.condeferrable AND NOT c.condeferred
                AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
                AND c.connamespace=t.relnamespace
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
                AND c.confupdtype='a' AND c.confdeltype='a' AND c.confmatchtype='s'
                AND ARRAY(SELECT a.attname::text FROM unnest(c.conkey) WITH ORDINALITY k(num,n)
                    JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num
                    ORDER BY k.n)=$3::text[]
                AND ARRAY(SELECT a.attname::text FROM unnest(c.confkey) WITH ORDINALITY k(num,n)
                    JOIN pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=k.num
                    ORDER BY k.n)=$5::text[]
                AND (SELECT count(*) FROM pg_trigger g WHERE g.tgconstraint=c.oid
                    AND g.tgisinternal AND g.tgenabled IN ('O','A'))=4
                AND NOT EXISTS(SELECT 1 FROM pg_trigger g WHERE g.tgconstraint=c.oid
                    AND g.tgenabled NOT IN ('O','A')))",
                &[&table, &name, &columns, &target, &targets],
            )
            .map_err(port)?
            .get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for table in TABLES {
        let expected = KEYS.iter().filter(|key| key.0 == table).count()
            + FOREIGN_KEYS.iter().filter(|key| key.0 == table).count()
            + checks::expected(table).len()
            + usize::from(table == "alert_email_outbox");
        // PostgreSQL 18 also exposes NOT NULL constraints through pg_constraint.
        let count: i64 = client.query_one(
            "SELECT count(*) FROM pg_constraint WHERE conrelid=$1::text::regclass AND contype<>'n'",
            &[&table]
        ).map_err(port)?.get(0);
        if usize::try_from(count).ok() != Some(expected) {
            return Err(incomplete());
        }
    }
    checks::validate(client)
}
