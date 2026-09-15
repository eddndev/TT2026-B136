use application::ApplicationError;
use postgres::Client;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use crate::postgres_case_administration_schema::{inconsistent, port};

/// Validates administrative metadata once at startup without loading document content.
pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let broken: bool = client.query_one(
        "WITH heads AS (SELECT DISTINCT ON(case_id) * FROM case_administration_revisions ORDER BY case_id,revision DESC),
         history AS (SELECT r.*,lag(revision) OVER w previous_revision,
             lag(administrative_status) OVER w previous_status,lag(nuc) OVER w previous_nuc,
             lag(case_administration_bytes('active',title,reference,nuc,nuc_authority,judicial_case_number,judicial_authority,offenses,general_information,complementary_identifiers)) OVER w previous_values
             FROM case_administration_revisions r WINDOW w AS (PARTITION BY case_id ORDER BY revision))
         SELECT EXISTS(SELECT 1 FROM cases c LEFT JOIN users u ON u.id=c.created_by
             LEFT JOIN case_administration_revisions r ON r.case_id=c.id AND r.revision=1
             WHERE u.id IS NULL OR NOT isfinite(c.created_at)
                 OR NOT case_administration_text_valid(c.title,200,FALSE)
                 OR NOT case_administration_text_valid(c.reference,100,FALSE)
                 OR (c.required_initial_revision IS NOT NULL AND c.required_initial_revision<>1)
                 OR (c.required_initial_revision=1 AND (r.case_id IS NULL
                     OR r.administrative_status COLLATE \"C\"<>'active'
                     OR r.title IS DISTINCT FROM c.title OR r.reference IS DISTINCT FROM c.reference
                     OR r.changed_by IS DISTINCT FROM c.created_by)))
         OR EXISTS(SELECT 1 FROM case_administration_revisions r LEFT JOIN cases c ON c.id=r.case_id
             LEFT JOIN users u ON u.id=r.changed_by WHERE c.id IS NULL OR u.id IS NULL
                 OR r.revision NOT BETWEEN 1 AND 4294967295
                 OR CASE WHEN case_administration_is_canonical(r.administrative_status,r.title,r.reference,r.nuc,r.nuc_authority,r.judicial_case_number,r.judicial_authority,r.offenses,r.general_information,r.complementary_identifiers)
                     THEN r.values_digest<>sha256(case_administration_bytes(r.administrative_status,r.title,r.reference,r.nuc,r.nuc_authority,r.judicial_case_number,r.judicial_authority,r.offenses,r.general_information,r.complementary_identifiers)) ELSE TRUE END)
         OR EXISTS(SELECT 1 FROM history h JOIN cases c ON c.id=h.case_id WHERE
             h.revision<>COALESCE(h.previous_revision+1,1)
             OR (h.previous_nuc IS NOT NULL AND h.nuc IS NULL)
             OR ((COALESCE(h.previous_status,'active') COLLATE \"C\"='closed'
                     OR h.administrative_status IS DISTINCT FROM COALESCE(h.previous_status,'active'))
                 AND COALESCE(h.previous_values,case_administration_bytes('active',c.title,c.reference,NULL,NULL,NULL,NULL,NULL,NULL,NULL))
                     IS DISTINCT FROM case_administration_bytes('active',h.title,h.reference,h.nuc,h.nuc_authority,h.judicial_case_number,h.judicial_authority,h.offenses,h.general_information,h.complementary_identifiers)))
         OR EXISTS(SELECT 1 FROM heads WHERE nuc IS NOT NULL GROUP BY nuc COLLATE \"C\" HAVING count(*)>1)
         OR EXISTS(SELECT 1 FROM heads WHERE judicial_case_number IS NOT NULL GROUP BY judicial_case_number COLLATE \"C\" HAVING count(*)>1)
         OR EXISTS(SELECT 1 FROM cases c LEFT JOIN case_administration_revisions r ON r.case_id=c.id AND r.revision=1
             LEFT JOIN case_initial_stage_registrations s ON s.case_id=c.id
             WHERE COALESCE(c.required_initial_revision=1 AND r.nuc IS NOT NULL,FALSE) IS DISTINCT FROM (s.case_id IS NOT NULL))
         OR EXISTS(SELECT 1 FROM case_initial_stage_registrations s LEFT JOIN cases c ON c.id=s.case_id
             LEFT JOIN case_administration_revisions r ON r.case_id=s.case_id AND r.revision=s.administration_revision
             WHERE c.id IS NULL OR r.case_id IS NULL OR s.stage_revision<>1 OR s.administration_revision<>1
                 OR s.stage COLLATE \"C\"<>'investigation')", &[],
    ).map_err(|_| inconsistent())?.get(0);
    if broken {
        return Err(inconsistent());
    }
    for row in client
        .query(
            "SELECT (created_at AT TIME ZONE 'UTC')::text FROM cases",
            &[],
        )
        .map_err(port)?
    {
        // PostgreSQL's timestamptz range exceeds the application's calendar range.
        let text: String = row.get(0);
        let year = text.split('-').next().and_then(|v| v.parse::<i32>().ok());
        if !matches!(year, Some(1..=9999)) || text.ends_with("BC") {
            return Err(inconsistent());
        }
    }
    for row in client
        .query(
            "SELECT changed_at,changed_by_email FROM case_administration_revisions",
            &[],
        )
        .map_err(port)?
    {
        let text: String = row.get(0);
        let at = OffsetDateTime::parse(&text, &Rfc3339).map_err(|_| inconsistent())?;
        if at
            .to_offset(UtcOffset::UTC)
            .format(&Rfc3339)
            .map_err(|_| inconsistent())?
            != text
        {
            return Err(inconsistent());
        }
        let email: String = row.get(1);
        if email.is_empty() || email.trim() != email || email.chars().any(char::is_control) {
            return Err(inconsistent());
        }
    }
    Ok(())
}
