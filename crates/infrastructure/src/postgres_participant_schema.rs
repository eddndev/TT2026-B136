use application::ApplicationError;
use postgres::Client;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let present: bool = client.query_one(
        "SELECT pg_catalog.to_regclass('case_participants') IS NOT NULL
         AND pg_catalog.to_regclass('case_participant_revisions') IS NOT NULL
         AND pg_catalog.to_regprocedure('participant_text_valid(text,integer)') IS NOT NULL
         AND pg_catalog.to_regprocedure('participant_values_is_canonical(text,text,text,text,text)') IS NOT NULL
         AND pg_catalog.to_regprocedure('participant_values_bytes(text,text,text,text,text)') IS NOT NULL
         AND pg_catalog.to_regprocedure('preserve_participant_history()') IS NOT NULL
         AND pg_catalog.to_regprocedure('enforce_participant_sequence()') IS NOT NULL", &[],
    ).map_err(port)?.get(0);
    if !present {
        return Err(incomplete());
    }
    for (table, key) in [
        ("case_participants", vec!["id"]),
        (
            "case_participant_revisions",
            vec!["participant_id", "revision"],
        ),
    ] {
        let valid: bool = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_constraint c
             WHERE c.conrelid=$1::text::pg_catalog.regclass AND c.contype='p'
             AND (SELECT array_agg(a.attname::text ORDER BY k.ordinality)
                 FROM unnest(c.conkey) WITH ORDINALITY k(attnum,ordinality)
                 JOIN pg_catalog.pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=k.attnum)=$2::text[])",
            &[&table,&key],
        ).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    let protected: bool = client.query_one(
        "SELECT (SELECT count(*) FROM pg_catalog.pg_constraint WHERE convalidated AND (
             (conrelid='case_participants'::pg_catalog.regclass AND (
                 (contype='c' AND conname='participant_initial_revision')
                 OR (contype='f' AND conname='participant_case_fk' AND confrelid='cases'::pg_catalog.regclass)
                 OR (contype='f' AND conname='participant_first_revision_fk'
                     AND confrelid='case_participant_revisions'::pg_catalog.regclass AND condeferrable AND condeferred)))
             OR (conrelid='case_participant_revisions'::pg_catalog.regclass AND (
                 (contype='c' AND conname IN ('participant_revision_range','participant_first_active','participant_canonical','participant_digest'))
                 OR (contype='f' AND conname='participant_root_fk' AND confrelid='case_participants'::pg_catalog.regclass)
                 OR (contype='f' AND conname='participant_actor_fk' AND confrelid='users'::pg_catalog.regclass)))))=9
         AND NOT EXISTS(SELECT 1 FROM pg_catalog.pg_trigger t JOIN pg_catalog.pg_constraint c ON c.oid=t.tgconstraint
             WHERE c.conrelid IN ('case_participants'::pg_catalog.regclass,'case_participant_revisions'::pg_catalog.regclass)
                 AND t.tgenabled NOT IN ('O','A'))
         AND (SELECT count(*) FROM pg_catalog.pg_trigger WHERE tgenabled IN ('O','A') AND (
             (tgrelid='case_participants'::pg_catalog.regclass AND tgname='participant_root_immutable'
                 AND tgtype=19 AND tgfoid='preserve_participant_history()'::pg_catalog.regprocedure)
             OR (tgrelid='case_participant_revisions'::pg_catalog.regclass AND (
                 (tgname='participant_revision_immutable' AND tgtype=19 AND tgfoid='preserve_participant_history()'::pg_catalog.regprocedure)
                 OR (tgname='participant_sequence' AND tgtype=7 AND tgfoid='enforce_participant_sequence()'::pg_catalog.regprocedure)))))=3
         AND (SELECT count(*) FROM pg_catalog.pg_proc WHERE NOT prosecdef AND provolatile='i' AND oid=ANY(ARRAY[
             'participant_text_valid(text,integer)'::pg_catalog.regprocedure,
             'participant_values_is_canonical(text,text,text,text,text)'::pg_catalog.regprocedure,
             'participant_values_bytes(text,text,text,text,text)'::pg_catalog.regprocedure]))=3
         AND (SELECT count(*) FROM pg_catalog.pg_attribute WHERE NOT attisdropped AND attnotnull AND (
             (attrelid='case_participants'::pg_catalog.regclass AND attname IN ('id','case_id','initial_revision'))
             OR (attrelid='case_participant_revisions'::pg_catalog.regclass AND attname IN (
                 'participant_id','revision','display_name','procedural_role','directory_status','values_digest','changed_at','changed_by','changed_by_email'))))=12",
        &[],
    ).map_err(port)?.get(0);
    if !protected {
        return Err(incomplete());
    }
    Ok(())
}

/// Scans metadata at startup; document content and captured evidence remain unloaded.
pub(crate) fn validate_inventory(client: &mut Client) -> Result<(), ApplicationError> {
    let broken: bool = client.query_one(
        "SELECT EXISTS(SELECT 1 FROM case_participants p LEFT JOIN cases c ON c.id=p.case_id
             LEFT JOIN (SELECT participant_id,MIN(revision) first,MAX(revision) last,COUNT(*) count
                 FROM case_participant_revisions GROUP BY participant_id) r ON r.participant_id=p.id
             WHERE c.id IS NULL OR p.initial_revision<>1 OR r.first IS DISTINCT FROM 1 OR r.count<>r.last)
         OR EXISTS(SELECT 1 FROM case_participant_revisions r
             LEFT JOIN case_participants p ON p.id=r.participant_id LEFT JOIN users u ON u.id=r.changed_by
             WHERE p.id IS NULL OR u.id IS NULL OR r.revision NOT BETWEEN 1 AND 4294967295
                 OR (r.revision=1 AND r.directory_status COLLATE \"C\"<>'active')
                 OR CASE WHEN participant_values_is_canonical(r.display_name,r.procedural_role,r.organization,r.legal_status,r.directory_status)
                     THEN r.values_digest<>pg_catalog.sha256(participant_values_bytes(r.display_name,r.procedural_role,r.organization,r.legal_status,r.directory_status))
                     ELSE TRUE END)", &[],
    ).map_err(port)?.get(0);
    if broken {
        return Err(inconsistent());
    }
    for row in client
        .query(
            "SELECT changed_at,changed_by_email FROM case_participant_revisions",
            &[],
        )
        .map_err(port)?
    {
        let timestamp: String = row.try_get(0).map_err(|_| inconsistent())?;
        let at = OffsetDateTime::parse(&timestamp, &Rfc3339).map_err(|_| inconsistent())?;
        if at
            .to_offset(UtcOffset::UTC)
            .format(&Rfc3339)
            .map_err(|_| inconsistent())?
            != timestamp
        {
            return Err(inconsistent());
        }
        let email: String = row.try_get(1).map_err(|_| inconsistent())?;
        if email.is_empty() || email.trim() != email || email.chars().any(char::is_control) {
            return Err(inconsistent());
        }
    }
    Ok(())
}

fn incomplete() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "participant schema is incomplete; run database migrate with an administrative role".into(),
    )
}
fn inconsistent() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "participant inventory is inconsistent; restore a consistent database".into(),
    )
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("participant schema: {error}"))
}
