use super::{incomplete, port, ATTEMPTS, RESULTS, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for (table, name, function, kind) in [
        (
            RESULTS,
            "deadline_worker_result_lock",
            "lock_deadline_dispatch()",
            6_i16,
        ),
        (
            RESULTS,
            "deadline_worker_result_insert",
            "validate_deadline_worker_result()",
            7,
        ),
        (
            RESULTS,
            "deadline_worker_result_immutable",
            "preserve_deadline_dispatch()",
            58,
        ),
        (
            ATTEMPTS,
            "deadline_worker_attempt_lock",
            "lock_deadline_dispatch()",
            6,
        ),
        (
            ATTEMPTS,
            "deadline_worker_attempt_insert",
            "validate_deadline_worker_attempt()",
            7,
        ),
        (
            ATTEMPTS,
            "deadline_worker_attempt_immutable",
            "preserve_deadline_dispatch()",
            58,
        ),
    ] {
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
    }
    for table in TABLES {
        let altered: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_trigger WHERE tgrelid=$1::text::regclass
                AND tgenabled NOT IN ('O','A'))
            OR (SELECT count(*) FROM pg_trigger WHERE tgrelid=$1::text::regclass
                AND NOT tgisinternal)<>3",
                &[&table],
            )
            .map_err(port)?
            .get(0);
        if altered {
            return Err(incomplete());
        }
    }
    let valid: bool = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_trigger t JOIN pg_constraint c ON c.oid=t.tgconstraint
        WHERE t.tgrelid='case_deadline_revisions'::regclass
            AND t.tgname='deadline_worker_revision_result'
            AND t.tgfoid='require_deadline_worker_result()'::regprocedure AND t.tgtype=5
            AND t.tgenabled IN ('O','A') AND NOT t.tgisinternal
            AND t.tgdeferrable AND t.tginitdeferred AND t.tgconstrrelid=0 AND t.tgconstrindid=0
            AND t.tgqual IS NULL AND t.tgnargs=0 AND octet_length(t.tgargs)=0
            AND t.tgattr=''::int2vector AND t.tgoldtable IS NULL AND t.tgnewtable IS NULL
            AND c.conrelid=t.tgrelid AND c.conname=t.tgname AND c.contype='t'
            AND c.convalidated AND c.condeferrable AND c.condeferred
            AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
            AND c.confrelid=0 AND c.conindid=0 AND c.conbin IS NULL
            AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
            AND (SELECT count(*) FROM pg_trigger linked WHERE linked.tgconstraint=c.oid)=1)",
            &[],
        )
        .map_err(port)?
        .get(0);
    if !valid {
        return Err(incomplete());
    }
    Ok(())
}
