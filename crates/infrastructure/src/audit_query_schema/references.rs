//! Only the exact built-in audit foreign-key triggers may attach to audit rows.
use application::ApplicationError;
use postgres::GenericClient;

use super::{incomplete, port};

pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for (table, constraint, deferred) in [
        (
            "password_reset_capabilities",
            "password_reset_capabilities_audit_sequence_fkey",
            true,
        ),
        (
            "owner_certificate_registrations",
            "owner_certificate_registrations_audit_sequence_fkey",
            false,
        ),
        (
            "owner_certificate_withdrawals",
            "owner_certificate_withdrawals_audit_sequence_fkey",
            false,
        ),
        (
            "case_hearing_derived_deadline_origins",
            "hearing_derived_deadline_audit_fk",
            false,
        ),
        (
            "case_precautionary_hearing_revisions",
            "precautionary_hearing_audit_event",
            true,
        ),
        (
            "case_measure_operations",
            "measure_operation_audit_event",
            true,
        ),
    ] {
        let rows = client.query("SELECT g.tgtype,
            g.tgisinternal AND g.tgenabled IN ('O','A') AND g.tgdeferrable=$3 AND g.tginitdeferred=$3
            AND c.contype='f' AND c.confrelid=a.oid AND c.convalidated AND c.condeferrable=$3 AND c.condeferred=$3
            AND c.confupdtype='a' AND c.confdeltype='a' AND c.confmatchtype='s'
            AND c.connamespace=t.relnamespace AND t.relnamespace=a.relnamespace AND t.relowner=a.relowner
            AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
            AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
            AND ARRAY(SELECT x.attname::text FROM unnest(c.conkey) WITH ORDINALITY k(n,ord)
                JOIN pg_attribute x ON x.attrelid=t.oid AND x.attnum=k.n ORDER BY k.ord)=ARRAY['audit_sequence']::text[]
            AND ARRAY(SELECT x.attname::text FROM unnest(c.confkey) WITH ORDINALITY k(n,ord)
                JOIN pg_attribute x ON x.attrelid=a.oid AND x.attnum=k.n ORDER BY k.ord)=ARRAY['sequence']::text[]
            AND g.tgconstrrelid=c.conrelid AND g.tgconstrindid=c.conindid
            AND g.tgqual IS NULL AND g.tgnargs=0 AND octet_length(g.tgargs)=0
            AND g.tgattr=''::int2vector AND g.tgoldtable IS NULL AND g.tgnewtable IS NULL
            AND ((g.tgtype=9 AND g.tgfoid='pg_catalog.\"RI_FKey_noaction_del\"()'::regprocedure)
                OR (g.tgtype=17 AND g.tgfoid='pg_catalog.\"RI_FKey_noaction_upd\"()'::regprocedure))
            FROM pg_trigger g JOIN pg_constraint c ON c.oid=g.tgconstraint
            JOIN pg_class t ON t.oid=c.conrelid JOIN pg_class a ON a.oid=g.tgrelid
            WHERE g.tgrelid='audit_events'::regclass AND c.conrelid=$1::text::regclass
                AND c.conname=$2 ORDER BY g.tgtype", &[&table, &constraint, &deferred]).map_err(port)?;
        if rows.len() != 2
            || rows.iter().zip([9_i16, 17]).any(|(row, kind)| {
                row.get::<_, i16>(0) != kind || row.get::<_, Option<bool>>(1) != Some(true)
            })
        {
            return Err(incomplete());
        }
    }
    let count: i64 = client
        .query_one(
            "SELECT count(*) FROM pg_trigger WHERE tgrelid='audit_events'::regclass",
            &[],
        )
        .map_err(port)?
        .get(0);
    if count != 12 {
        return Err(incomplete());
    }
    Ok(())
}
