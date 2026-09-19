use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        ordinary(
            client,
            table,
            "member_write_lock",
            "lock_member_writes()",
            62,
        )?;
        let expected: i64 = if table == "users" { 3 } else { 1 };
        let altered: bool = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass AND tgenabled NOT IN ('O','A'))
            OR (SELECT count(*) FROM pg_trigger WHERE tgrelid=$1::text::regclass AND NOT tgisinternal)<>$2",
            &[&table,&expected]).map_err(port)?.get(0);
        if altered {
            return Err(incomplete());
        }
    }
    ordinary(
        client,
        "users",
        "member_access_guard",
        "guard_member_access()",
        19,
    )?;
    ordinary(
        client,
        "users",
        "member_no_delete",
        "preserve_member_accounts()",
        42,
    )
}

fn ordinary<C: GenericClient>(
    client: &mut C,
    table: &str,
    name: &str,
    function: &str,
    kind: i16,
) -> Result<(), ApplicationError> {
    let valid: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass
            AND tgname=$2 AND tgfoid=$3::text::regprocedure AND tgtype=$4
            AND tgenabled IN ('O','A') AND NOT tgisinternal AND NOT tgdeferrable
            AND NOT tginitdeferred AND tgconstraint=0 AND tgconstrrelid=0 AND tgconstrindid=0
            AND tgqual IS NULL AND tgnargs=0 AND octet_length(tgargs)=0
            AND tgattr=''::int2vector AND tgoldtable IS NULL AND tgnewtable IS NULL)",
            &[&table, &name, &function, &kind],
        )
        .map_err(port)?
        .get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
