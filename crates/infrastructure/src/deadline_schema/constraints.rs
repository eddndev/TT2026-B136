use super::{incomplete, port};
use application::ApplicationError;
use postgres::GenericClient;
pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for (table, name, kind, names) in [
        (
            "case_deadlines",
            "deadline_root_primary",
            "p",
            &["id"] as &[&str],
        ),
        (
            "case_deadlines",
            "deadline_root_scope",
            "u",
            &["id", "case_id"] as &[&str],
        ),
        (
            "case_deadline_revisions",
            "deadline_revision_primary",
            "p",
            &["deadline_id", "revision"] as &[&str],
        ),
        (
            "case_deadline_revisions",
            "deadline_operation_unique",
            "u",
            &["operation_id"] as &[&str],
        ),
    ] {
        let valid: bool = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_constraint c WHERE c.conrelid=$1::text::regclass AND c.conname=$2 AND c.contype::text=$3 AND c.convalidated AND NOT c.condeferrable AND NOT c.condeferred AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n) JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$4::text[])", &[&table,&name,&kind,&names]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for (table, name, target, names, targets, deferred) in [
        (
            "case_deadlines",
            "deadline_root_case",
            "cases",
            &["case_id"] as &[&str],
            &["id"] as &[&str],
            false,
        ),
        (
            "case_deadlines",
            "deadline_first_revision",
            "case_deadline_revisions",
            &["id", "initial_revision"] as &[&str],
            &["deadline_id", "revision"] as &[&str],
            true,
        ),
        (
            "case_deadline_revisions",
            "deadline_revision_root",
            "case_deadlines",
            &["deadline_id", "case_id"] as &[&str],
            &["id", "case_id"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_profile_revision",
            "deadline_profile_revisions",
            &["profile_id", "profile_revision"] as &[&str],
            &["profile_id", "revision"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_observed_administration",
            "case_administration_revisions",
            &["case_id", "observed_administration_revision"] as &[&str],
            &["case_id", "revision"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_responsible",
            "users",
            &["responsible_id"] as &[&str],
            &["id"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_recorded_actor",
            "users",
            &["recorded_by"] as &[&str],
            &["id"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_source_fact_scope",
            "case_procedural_facts",
            &["source_fact_family", "source_fact_id", "case_id"] as &[&str],
            &["family", "id", "case_id"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_source_fact_revision",
            "case_procedural_fact_revisions",
            &["source_fact_family", "source_fact_id", "source_revision"] as &[&str],
            &["family", "id", "revision"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_source_fact_head",
            "case_procedural_fact_revisions",
            &[
                "source_fact_family",
                "source_fact_id",
                "source_head_revision",
            ] as &[&str],
            &["family", "id", "revision"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_source_result_scope",
            "case_hearing_results",
            &["source_result_id", "case_id", "source_hearing_id"] as &[&str],
            &["id", "case_id", "hearing_id"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_source_result_revision",
            "case_hearing_result_revisions",
            &["source_result_id", "source_revision"] as &[&str],
            &["result_id", "revision"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_source_result_head",
            "case_hearing_result_revisions",
            &["source_result_id", "source_head_revision"] as &[&str],
            &["result_id", "revision"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_source_parent_revision",
            "case_procedural_fact_revisions",
            &[
                "source_parent_family",
                "source_parent_resolution_id",
                "source_parent_resolution_revision",
            ] as &[&str],
            &["family", "id", "revision"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_source_head_parent_revision",
            "case_procedural_fact_revisions",
            &[
                "source_parent_family",
                "source_parent_resolution_id",
                "source_head_parent_resolution_revision",
            ] as &[&str],
            &["family", "id", "revision"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_calendar_revision",
            "judicial_calendar_revisions",
            &["calendar_id", "calendar_revision"] as &[&str],
            &["calendar_id", "revision"] as &[&str],
            false,
        ),
        (
            "case_deadline_revisions",
            "deadline_calendar_head",
            "judicial_calendar_revisions",
            &["calendar_id", "calendar_head_revision"] as &[&str],
            &["calendar_id", "revision"] as &[&str],
            false,
        ),
    ] {
        let valid:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_constraint c WHERE c.conrelid=$1::text::regclass AND c.conname=$2 AND c.confrelid=$3::text::regclass AND c.contype='f' AND c.convalidated AND c.condeferrable=$6 AND c.condeferred=$6 AND c.confupdtype='a' AND c.confdeltype='a' AND c.confmatchtype='s'
        AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.conkey) WITH ORDINALITY k(num,n) JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.num)=$4::text[]
        AND (SELECT array_agg(a.attname::text ORDER BY k.n) FROM unnest(c.confkey) WITH ORDINALITY k(num,n) JOIN pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=k.num)=$5::text[])",&[&table,&name,&target,&names,&targets,&deferred]).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    // CHECK definitions are validated separately; PostgreSQL 18 reports NOT NULL constraints too.
    let extra:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid IN ('case_deadlines'::regclass,'case_deadline_revisions'::regclass) AND contype NOT IN ('n','c')) AND (SELECT count(*) FROM pg_constraint WHERE conrelid IN ('case_deadlines'::regclass,'case_deadline_revisions'::regclass) AND contype NOT IN ('n','c'))<>$1", &[&21_i64]).map_err(port)?.get(0);
    if extra {
        return Err(incomplete());
    }
    Ok(())
}
