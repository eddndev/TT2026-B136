use super::{checks, columns, incomplete, port, TABLES};
use application::ApplicationError;
use postgres::GenericClient;

pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for table in TABLES {
        let mut invalid: Vec<String> = checks::expected(table)
            .into_iter()
            .map(|check| format!("({check}) IS FALSE"))
            .collect();
        for &(name, _, required) in columns::expected(table) {
            if required {
                invalid.push(format!("{name} IS NULL"));
            }
        }
        let invalid: bool = client
            .query_one(
                &format!(
                    "SELECT EXISTS(SELECT 1 FROM {table} WHERE {})",
                    invalid.join(" OR ")
                ),
                &[],
            )
            .map_err(port)?
            .get(0);
        if invalid {
            return Err(incomplete());
        }
    }
    let invalid: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM case_report_jobs j
            LEFT JOIN users u ON u.id=j.requester_id
            LEFT JOIN case_report_snapshots s ON s.report_id=j.id
            LEFT JOIN case_report_notices n ON n.report_id=j.id
            LEFT JOIN LATERAL(SELECT count(*) AS total FROM case_report_artifacts a WHERE a.report_id=j.id) a ON TRUE
            WHERE u.id IS NULL OR j.id='00000000-0000-0000-0000-000000000000'::uuid
                OR j.operation_id='00000000-0000-0000-0000-000000000000'::uuid
                OR j.auth_generation>j.account_revision
                OR j.principal->>'id' IS DISTINCT FROM j.requester_id::text
                OR (j.scope='office' AND j.principal->>'role' IS DISTINCT FROM 'owner')
                OR (j.scope='assigned_cases' AND j.principal->>'role' IS DISTINCT FROM 'litigator')
                OR j.case_ids IS DISTINCT FROM ARRAY(SELECT DISTINCT id FROM unnest(j.case_ids) id ORDER BY id)
                OR EXISTS(SELECT 1 FROM unnest(j.case_ids) id WHERE id='00000000-0000-0000-0000-000000000000'::uuid)
                OR (j.state='ready' AND (s.report_id IS NULL OR a.total<>2 OR n.kind IS DISTINCT FROM 'ready'))
                OR (j.state='failed' AND (a.total<>0 OR n.kind IS DISTINCT FROM 'failed'))
                OR (j.state NOT IN ('ready','failed') AND (a.total<>0 OR n.report_id IS NOT NULL))
                OR (j.state IN ('queued','capturing','retry_capturing') AND (s.report_id IS NOT NULL OR cardinality(j.case_ids)<>0))
                OR (j.state IN ('rendering','retry_rendering') AND s.report_id IS NULL))
        OR EXISTS(SELECT 1 FROM case_report_snapshots s LEFT JOIN case_report_jobs j ON j.id=s.report_id WHERE j.id IS NULL)
        OR EXISTS(SELECT 1 FROM case_report_artifacts a LEFT JOIN case_report_snapshots s ON s.report_id=a.report_id
            WHERE s.report_id IS NULL OR a.snapshot_digest<>s.digest)
        OR EXISTS(SELECT 1 FROM case_report_notices n LEFT JOIN case_report_jobs j ON j.id=n.report_id WHERE j.id IS NULL)",
        &[],
    ).map_err(port)?.get(0);
    if invalid {
        return Err(incomplete());
    }
    Ok(())
}
