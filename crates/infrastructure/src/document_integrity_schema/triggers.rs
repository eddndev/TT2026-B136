use super::{incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        ordinary(
            client,
            table,
            "document_integrity_lock",
            "lock_document_integrity_history()",
            6,
        )?;
        ordinary(
            client,
            table,
            "document_integrity_immutable",
            "preserve_document_integrity_history()",
            58,
        )?;
        let expected: i64 = 2;
        let altered: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass
                AND tgenabled NOT IN ('O','A'))
            OR (SELECT count(*) FROM pg_trigger WHERE tgrelid=$1::text::regclass
                AND NOT tgisinternal)<>$2",
                &[&table, &expected],
            )
            .map_err(port)?
            .get(0);
        if altered {
            return Err(incomplete());
        }
    }
    Ok(())
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
